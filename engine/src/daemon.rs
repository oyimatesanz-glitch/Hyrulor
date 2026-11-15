//! Cliente JSON-Lines del daemon Hyrulor.
//!
//! Habla con el mismo socket Unix que el cliente de terminal (`hyory`):
//! cada petición es una línea JSON y cada respuesta es otra línea JSON.
//! La conexión se abre por petición, igual que `src/bin/hyrulor-chat.rs`.

use std::{
    env,
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::{json, Value};

pub const DEFAULT_SOCKET: &str = "/tmp/omni.sock";

/// Ruta del socket del daemon. Precedencia:
/// `HYRULOR_DAEMON_SOCKET` > `HYORY_SOCKET` > `/tmp/omni.sock`.
pub fn socket_path() -> PathBuf {
    env::var_os("HYRULOR_DAEMON_SOCKET")
        .or_else(|| env::var_os("HYORY_SOCKET"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET))
}

/// Respuesta del daemon, espejo del protocolo `status` + `data`/`message`.
#[derive(Debug, Clone)]
pub enum Reply {
    Ok(Value),
    Error(String),
    RequiresAuth {
        mode: String,
        request_id: String,
        form: Value,
    },
}

/// Envía una petición JSON y devuelve la respuesta.
///
/// `Err` se reserva para fallos de transporte/protocolo (daemon caído,
/// JSON inválido, respuesta vacía). Un `status: "error"` del daemon se
/// devuelve como `Reply::Error`, no como `Err`.
pub fn request(socket: &Path, request_value: Value) -> Result<Reply, String> {
    request_with_timeout(socket, request_value, None)
}

/// Igual que [`request`], pero con un timeout opcional de lectura/escritura.
/// Las sondas de estado lo usan para que un daemon colgado no deje hilos
/// bloqueados indefinidamente.
fn request_with_timeout(
    socket: &Path,
    request_value: Value,
    timeout: Option<Duration>,
) -> Result<Reply, String> {
    let mut stream = UnixStream::connect(socket).map_err(|error| error.to_string())?;
    if let Some(timeout) = timeout {
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|error| error.to_string())?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|error| error.to_string())?;
    }

    let line = serde_json::to_string(&request_value).map_err(|error| error.to_string())?;
    writeln!(stream, "{line}").map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .map_err(|error| error.to_string())?;
    if response.trim().is_empty() {
        return Err("el daemon cerró la conexión sin responder".to_string());
    }

    let value: Value = serde_json::from_str(&response).map_err(|error| error.to_string())?;
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

/// Pregunta al asistente. Devuelve la misma `Reply` que el daemon produce.
pub fn chat(socket: &Path, query: &str) -> Result<Reply, String> {
    request(socket, json!({"request": "chat", "query": query}))
}

/// Consulta de salud: `{"request":"health"}`. Timeout corto: es una sonda.
pub fn health(socket: &Path) -> Result<Reply, String> {
    request_with_timeout(
        socket,
        json!({"request": "health"}),
        Some(Duration::from_secs(3)),
    )
}

/// Modo actual del daemon (`secure` o `simple`). Timeout corto: es una sonda.
pub fn get_mode(socket: &Path) -> Result<Reply, String> {
    request_with_timeout(
        socket,
        json!({"request": "get_mode"}),
        Some(Duration::from_secs(3)),
    )
}

/// Aprueba o cancela una acción pendiente de autorización humana.
pub fn authorize(
    socket: &Path,
    id: &str,
    approved: bool,
    form: Option<Value>,
) -> Result<Reply, String> {
    let mut value = json!({"request": "authorize", "id": id, "approved": approved});
    if let Some(form) = form {
        value["form"] = form;
    }
    request(socket, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::net::UnixListener,
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_socket() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("reloj")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "hyrulor-engine-{}-{}.sock",
            std::process::id(),
            nanos
        ))
    }

    /// Levanta un daemon de pruebas que responde `response` a la primera
    /// petición y devuelve el handle del hilo para poder esperarlo.
    fn mock_daemon(response: &'static str) -> (PathBuf, thread::JoinHandle<()>) {
        let path = temp_socket();
        let listener = UnixListener::bind(&path).expect("bind del socket de prueba");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("aceptar conexión");
            let mut reader = BufReader::new(stream.try_clone().expect("clonar stream"));
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            writeln!(stream, "{response}").expect("responder");
            stream.flush().expect("flush");
        });
        (path, handle)
    }

    #[test]
    fn parses_ok_reply_and_sends_chat_request() {
        let (path, server) = mock_daemon(r#"{"status":"ok","data":{"answer":"hola"}}"#);
        let reply = chat(&path, "hola").expect("petición ok");
        match reply {
            Reply::Ok(data) => assert_eq!(data["answer"], "hola"),
            other => panic!("esperaba Ok, recibí {other:?}"),
        }
        server.join().expect("servidor termina");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn parses_error_reply() {
        let (path, server) = mock_daemon(r#"{"status":"error","message":"rechazado"}"#);
        match chat(&path, "x").expect("petición ok") {
            Reply::Error(message) => assert_eq!(message, "rechazado"),
            other => panic!("esperaba Error, recibí {other:?}"),
        }
        server.join().expect("servidor termina");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn parses_requires_auth_reply() {
        let (path, server) = mock_daemon(
            r#"{"status":"requires_auth","mode":"secure","request_id":"AUTH-1","form":{"title":"F"}}"#,
        );
        match chat(&path, "escanea").expect("petición ok") {
            Reply::RequiresAuth {
                mode,
                request_id,
                form,
            } => {
                assert_eq!(mode, "secure");
                assert_eq!(request_id, "AUTH-1");
                assert_eq!(form["title"], "F");
            }
            other => panic!("esperaba RequiresAuth, recibí {other:?}"),
        }
        server.join().expect("servidor termina");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn authorize_builds_the_expected_payload() {
        // Un daemon que devuelve ok independientemente del payload.
        let (path, server) = mock_daemon(r#"{"status":"ok","data":{"authorized":true}}"#);
        let reply = authorize(&path, "AUTH-9", true, Some(json!({"target": "10.0.0.1"})))
            .expect("petición ok");
        assert!(matches!(reply, Reply::Ok(_)));
        server.join().expect("servidor termina");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unreachable_daemon_is_a_transport_error() {
        let path = temp_socket();
        let reply = chat(&path, "hola");
        assert!(reply.is_err(), "debe fallar sin daemon: {reply:?}");
    }
}
