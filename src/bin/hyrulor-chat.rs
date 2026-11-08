use std::{
    env,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};

const DEFAULT_SOCKET: &str = "/tmp/omni.sock";

enum Reply {
    Ok(Value),
    RequiresAuth {
        mode: String,
        request_id: String,
        form: Value,
    },
    Error(String),
}

enum Decision {
    Approve(Option<Value>),
    Cancel,
}

fn main() {
    let socket: PathBuf = env::var_os("HYORY_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET));

    let args: Vec<String> = env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("--secure") {
        let value = args.get(1).map(String::as_str).unwrap_or("status");
        if let Err(error) = mode_command(&socket, value) {
            eprintln!("Error: {error}");
            std::process::exit(1);
        }
        return;
    }

    if let Some(flag) = args.first().map(String::as_str) {
        let result = match flag {
            "--imagen" => imagen_command(&socket, &args[1..]),
            "--voz" => voz_command(&socket, &args[1..]),
            "--escuchar" => escuchar_command(&socket, &args[1..]),
            "--hablar" => hablar_command(&socket, &args[1..]),
            _ => Ok(false),
        };
        match result {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
        }
    }

    if !args.is_empty() {
        let query = args.join(" ");
        match ask(&socket, &query) {
            Ok(answer) => println!("{answer}"),
            Err(error) => {
                eprintln!("Error: {error}");
                std::process::exit(1);
            }
        }
        return;
    }

    repl(&socket);
}

fn repl(socket: &Path) {
    println!("Hyrulor REPL · escribe tu pregunta (/ayuda para comandos)");
    println!("Socket: {}", socket.display());
    match mode_value(socket) {
        Ok(mode) => println!("Modo: {mode}"),
        Err(_) => println!("Modo: desconocido"),
    }
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("Tú > ");
        if let Err(error) = stdout.flush() {
            eprintln!("Error escribiendo el prompt: {error}");
            break;
        }

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("Error leyendo la entrada: {error}");
                break;
            }
        }

        let query = line.trim();
        if query.is_empty() {
            continue;
        }
        if matches!(
            query.to_lowercase().as_str(),
            "/salir" | "salir" | "exit" | "quit"
        ) {
            println!("Hasta luego.");
            break;
        }

        if let Some(rest) = query.strip_prefix("/secure") {
            let value = rest.trim();
            let value = if value.is_empty() { "status" } else { value };
            if let Err(error) = mode_command(socket, value) {
                eprintln!("Error: {error}\n");
            }
            continue;
        }
        if matches!(query, "/mode" | "/modo") {
            match mode_value(socket) {
                Ok(mode) => println!("🔐 Modo actual: {mode}\n"),
                Err(error) => eprintln!("Error: {error}\n"),
            }
            continue;
        }
        if let Some(rest) = query.strip_prefix("/imagen") {
            if let Err(error) = imagen_command(socket, &split_args(rest)) {
                eprintln!("Error: {error}\n");
            }
            continue;
        }
        if let Some(rest) = query.strip_prefix("/voz") {
            if let Err(error) = voz_command(socket, &split_args(rest)) {
                eprintln!("Error: {error}\n");
            }
            continue;
        }
        if let Some(rest) = query.strip_prefix("/escuchar") {
            if let Err(error) = escuchar_command(socket, &split_args(rest)) {
                eprintln!("Error: {error}\n");
            }
            continue;
        }
        if let Some(rest) = query.strip_prefix("/hablar") {
            if let Err(error) = hablar_command(socket, &split_args(rest)) {
                eprintln!("Error: {error}\n");
            }
            continue;
        }
        if matches!(query, "/ayuda" | "/help") {
            print_help();
            continue;
        }

        match ask(socket, query) {
            Ok(answer) => println!("Hyrulor > {answer}\n"),
            Err(error) => eprintln!("Error: {error}\n"),
        }
    }
}

fn print_help() {
    println!(
        "\nComandos:\n\
         \x20 /ayuda              muestra esta ayuda\n\
         \x20 /mode | /modo       muestra el modo actual\n\
         \x20 /secure on|off      activa o desactiva el modo seguro\n\
         \x20 /imagen ruta [msg]  analiza una imagen con Qwen2.5-VL\n\
         \x20 /escuchar [seg]     graba del micrófono, transcribe y responde\n\
         \x20 /voz archivo.wav    transcribe un WAV y responde\n\
         \x20 /hablar texto       convierte texto a voz con Piper\n\
         \x20 /salir              termina la sesión\n\
         \nModo seguro: formulario completo de autorización para acciones ofensivas.\n\
         Modo simple: solo pregunta «¿Seguro que quieres hacer esto?» (y/n).\n"
    );
}

fn split_args(input: &str) -> Vec<String> {
    input.split_whitespace().map(str::to_string).collect()
}

fn imagen_command(socket: &Path, args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(path) = args.first() else {
        return Err("uso: /imagen <ruta> [pregunta]".into());
    };
    let prompt = args.get(1..).map(|rest| rest.join(" ")).unwrap_or_default();
    let prompt = if prompt.is_empty() {
        Value::Null
    } else {
        Value::String(prompt)
    };

    let data = send_ok(
        socket,
        json!({"request": "describe_image", "path": path, "prompt": prompt}),
    )?;
    println!(
        "👁️  Hyrulor > {}",
        data.get("answer")
            .and_then(Value::as_str)
            .unwrap_or("(sin respuesta)")
    );
    Ok(true)
}

fn voz_command(socket: &Path, args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(path) = args.first() else {
        return Err("uso: /voz <archivo.wav>".into());
    };
    let data = send_ok(socket, json!({"request": "transcribe", "path": path}))?;
    let text = data
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if text.is_empty() {
        return Err("whisper no devolvió texto".into());
    }
    voice_turn(socket, &text)?;
    Ok(true)
}

fn escuchar_command(socket: &Path, args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    let seconds = args
        .first()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(5);
    println!("🎙️  Grabando {seconds} segundos...");
    let data = send_ok(socket, json!({"request": "listen", "seconds": seconds}))?;
    let text = data
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if text.is_empty() {
        return Err("no se reconoció voz".into());
    }
    voice_turn(socket, &text)?;
    Ok(true)
}

fn hablar_command(socket: &Path, args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    let text = args.join(" ");
    if text.trim().is_empty() {
        return Err("uso: /hablar <texto>".into());
    }
    let data = send_ok(socket, json!({"request": "speak", "text": text}))?;
    if let Some(audio) = data.get("audio").and_then(Value::as_str) {
        println!("🔊 Audio generado: {audio}");
    } else {
        println!("🔊 Texto reproducido.");
    }
    Ok(true)
}

fn voice_turn(socket: &Path, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Tú > {text}");
    let answer = ask(socket, text)?;
    println!("Hyrulor > {answer}\n");
    if let Err(error) = hablar_command(socket, &[answer]) {
        eprintln!("(voz) {error}");
    }
    Ok(())
}

fn ask(socket: &Path, query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let reply = request(socket, json!({"request": "chat", "query": query}))?;
    handle_reply(socket, reply)
}

fn handle_reply(socket: &Path, reply: Reply) -> Result<String, Box<dyn std::error::Error>> {
    match reply {
        Reply::Ok(data) => Ok(data
            .get("answer")
            .and_then(Value::as_str)
            .unwrap_or("(respuesta vacía)")
            .to_string()),
        Reply::Error(message) => Err(message.into()),
        Reply::RequiresAuth {
            mode,
            request_id,
            form,
        } => {
            let decision = if mode == "secure" {
                collect_secure_form(&form)?
            } else {
                collect_simple_confirmation(&form)
            };

            let authorization = match decision {
                Decision::Approve(Some(values)) => json!({
                    "request": "authorize",
                    "id": request_id,
                    "approved": true,
                    "form": values,
                }),
                Decision::Approve(None) => json!({
                    "request": "authorize",
                    "id": request_id,
                    "approved": true,
                }),
                Decision::Cancel => json!({
                    "request": "authorize",
                    "id": request_id,
                    "approved": false,
                }),
            };

            match request(socket, authorization)? {
                Reply::Ok(data) => Ok(data
                    .get("answer")
                    .and_then(Value::as_str)
                    .unwrap_or("(sin respuesta)")
                    .to_string()),
                Reply::Error(message) => Err(message.into()),
                Reply::RequiresAuth { .. } => {
                    Err("respuesta de autorización inesperada del daemon".into())
                }
            }
        }
    }
}

fn collect_simple_confirmation(form: &Value) -> Decision {
    let question = form
        .get("question")
        .and_then(Value::as_str)
        .unwrap_or("¿Seguro que quieres hacer esto? [y/n]");

    println!("\n⚠️  {question}");
    if let Some(action) = form.get("proposed_action") {
        let pretty = serde_json::to_string(action).unwrap_or_default();
        println!("   Acción propuesta: {pretty}");
    }
    print!("> ");
    let _ = io::stdout().flush();

    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return Decision::Cancel;
    }

    match line.trim().to_lowercase().as_str() {
        "y" | "yes" | "s" | "si" | "sí" => Decision::Approve(None),
        _ => Decision::Cancel,
    }
}

fn collect_secure_form(form: &Value) -> Result<Decision, Box<dyn std::error::Error>> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║   FORMULARIO DE AUTORIZACIÓN DE ACCIÓN OFENSIVA          ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    if let Some(title) = form.get("title").and_then(Value::as_str) {
        println!("{title}");
    }
    if let Some(notice) = form.get("notice").and_then(Value::as_str) {
        println!("\n{notice}");
    }
    if let Some(action) = form.get("proposed_action") {
        println!(
            "\nAcción propuesta:\n{}",
            serde_json::to_string_pretty(action).unwrap_or_default()
        );
    }
    if let Some(request_id) = form.get("request_id").and_then(Value::as_str) {
        println!("\nID de solicitud: {request_id}");
    }

    let expected_code = form
        .get("authorization_code")
        .and_then(Value::as_str)
        .map(str::to_string);

    if let Some(code) = &expected_code {
        println!("\n🔐 CÓDIGO DE AUTORIZACIÓN: {code}");
        println!("   Escríbelo en el campo «authorization_code» para confirmar.");
    }

    let mut values = serde_json::Map::new();
    if let Some(fields) = form.get("fields").and_then(Value::as_array) {
        for field in fields {
            let name = field.get("name").and_then(Value::as_str).unwrap_or("campo");
            let label = field.get("label").and_then(Value::as_str).unwrap_or(name);
            let required = field
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(true);

            print!("  {label}{} > ", if required { " *" } else { "" });
            io::stdout().flush()?;

            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            let value = line.trim().to_string();

            if value.is_empty() && required {
                println!("  ❌ Campo obligatorio vacío. Autorización cancelada.");
                return Ok(Decision::Cancel);
            }
            values.insert(name.to_string(), Value::String(value));
        }
    }

    if let Some(expected) = &expected_code {
        let provided = values
            .get("authorization_code")
            .and_then(Value::as_str)
            .unwrap_or("");
        if provided != expected {
            println!("  ❌ Código de autorización incorrecto. Operación cancelada.");
            return Ok(Decision::Cancel);
        }
    }

    print!("\n¿Confirmas la autorización? (y/n) > ");
    io::stdout().flush()?;
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;
    if !matches!(
        confirm.trim().to_lowercase().as_str(),
        "y" | "yes" | "s" | "si" | "sí"
    ) {
        return Ok(Decision::Cancel);
    }

    Ok(Decision::Approve(Some(Value::Object(values))))
}

fn mode_command(socket: &Path, value: &str) -> Result<(), Box<dyn std::error::Error>> {
    match value.trim().to_lowercase().as_str() {
        "on" | "secure" | "activar" => {
            send_ok(socket, json!({"request": "set_mode", "secure": true}))?;
            println!("🔐 Modo seguro ACTIVADO: formulario completo habilitado.");
        }
        "off" | "simple" | "desactivar" => {
            send_ok(socket, json!({"request": "set_mode", "secure": false}))?;
            println!("🔓 Modo seguro DESACTIVADO: solo confirmación simple (y/n).");
        }
        "status" | "estado" | "" => {
            println!("{}", mode_value(socket)?);
        }
        other => {
            return Err(format!("valor no válido: {other} (usa on|off|status)").into());
        }
    }
    Ok(())
}

fn mode_value(socket: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let data = send_ok(socket, json!({"request": "get_mode"}))?;
    Ok(data
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("desconocido")
        .to_string())
}

fn send_ok(socket: &Path, request_value: Value) -> Result<Value, Box<dyn std::error::Error>> {
    match request(socket, request_value)? {
        Reply::Ok(data) => Ok(data),
        Reply::Error(message) => Err(message.into()),
        Reply::RequiresAuth { .. } => Err("respuesta de autorización inesperada".into()),
    }
}

fn request(socket: &Path, request_value: Value) -> Result<Reply, Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(socket)?;
    writeln!(stream, "{}", serde_json::to_string(&request_value)?)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    if response.trim().is_empty() {
        return Err("el daemon cerró la conexión sin responder".into());
    }

    let value: Value = serde_json::from_str(&response)?;
    let status = value.get("status").and_then(Value::as_str).unwrap_or("");

    Ok(match status {
        "ok" => Reply::Ok(value.get("data").cloned().unwrap_or(Value::Null)),
        "requires_auth" => Reply::RequiresAuth {
            mode: value
                .get("mode")
                .and_then(Value::as_str)
                .unwrap_or("simple")
                .to_string(),
            request_id: value
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            form: value.get("form").cloned().unwrap_or(Value::Null),
        },
        "error" => Reply::Error(
            value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("error desconocido")
                .to_string(),
        ),
        other => Reply::Error(format!(
            "respuesta inesperada ({other}): {}",
            response.trim()
        )),
    })
}
