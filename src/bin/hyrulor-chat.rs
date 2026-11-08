use std::{
    env,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

use serde_json::{json, Value};

const DEFAULT_SOCKET: &str = "/tmp/omni.sock";

fn main() {
    let socket: PathBuf = env::var_os("HYORY_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET));

    println!("Hyrulor REPL · escribe tu pregunta (/salir para terminar)");
    println!("Socket: {}", socket.display());
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

        match ask(&socket, query) {
            Ok(answer) => println!("Hyrulor > {answer}\n"),
            Err(error) => eprintln!("Error: {error}\n"),
        }
    }
}

fn ask(socket: &Path, query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(socket)?;
    let request = json!({"request": "chat", "query": query});
    writeln!(stream, "{}", serde_json::to_string(&request)?)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    if response.trim().is_empty() {
        return Err("el daemon cerró la conexión sin responder".into());
    }

    let value: Value = serde_json::from_str(&response)?;
    match value.get("status").and_then(Value::as_str) {
        Some("ok") => Ok(value
            .get("data")
            .and_then(|data| data.get("answer"))
            .and_then(Value::as_str)
            .unwrap_or("(respuesta vacía)")
            .to_string()),
        Some("error") => Err(value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("error desconocido")
            .to_string()
            .into()),
        _ => Err(format!("respuesta inesperada: {}", response.trim()).into()),
    }
}
