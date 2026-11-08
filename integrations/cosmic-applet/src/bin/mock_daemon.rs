//! Small local daemon used only to test the applet UI and JSON-lines protocol.

use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

const SOCKET_PATH: &str = "/tmp/omni.sock";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Request {
    request: String,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    secure: Option<bool>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    approved: Option<bool>,
    #[serde(default)]
    form: Option<Value>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    seconds: Option<u32>,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Clone)]
struct State {
    secure: Arc<AtomicBool>,
    counter: Arc<AtomicU64>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let socket_path = std::env::var_os("HYORY_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(SOCKET_PATH));

    remove_stale_socket(&socket_path)?;
    let listener = UnixListener::bind(&socket_path)?;
    std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;

    let state = State {
        secure: Arc::new(AtomicBool::new(false)),
        counter: Arc::new(AtomicU64::new(1)),
    };

    println!(
        "Hyrulor mock daemon escuchando en {} (0600)",
        socket_path.display()
    );
    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(error) = handle(stream, state).await {
                eprintln!("mock daemon: {error}");
            }
        });
    }
}

fn remove_stale_socket(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            // SAFETY: geteuid has no preconditions and only reads the effective UID.
            let uid = unsafe { libc::geteuid() } as u32;
            if metadata.file_type().is_socket() && metadata.uid() == uid {
                std::fs::remove_file(path)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "no se sobrescribe una ruta que no sea un socket propio",
                ))
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

async fn handle(stream: UnixStream, state: State) -> io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let request: Request = serde_json::from_str(line.trim_end())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    match request.request.as_str() {
        "chat" => {
            let query = request.query.unwrap_or_default();
            if looks_offensive(&query) {
                let mode = if state.secure.load(Ordering::Relaxed) {
                    "secure"
                } else {
                    "simple"
                };
                let id = format!(
                    "MOCK-AUTH-{}",
                    state.counter.fetch_add(1, Ordering::Relaxed)
                );
                let form = if mode == "secure" {
                    json!({
                        "title": "AUTORIZACIÓN DE ACCIÓN OFENSIVA",
                        "question": "Complete el formulario para autorizar la acción.",
                        "request_id": id,
                        "authorization_code": "123456",
                        "proposed_action": {
                            "tool": "offensive_security_action",
                            "arguments": {"action": "port_scan", "target": "127.0.0.1"}
                        },
                        "fields": [
                            {"name": "target", "label": "Objetivo (IP / dominio / host)", "required": true},
                            {"name": "scope", "label": "Alcance autorizado", "required": true},
                            {"name": "justification", "label": "Justificación", "required": true},
                            {"name": "start_time", "label": "Inicio (ISO-8601)", "required": true},
                            {"name": "end_time", "label": "Fin (ISO-8601)", "required": true},
                            {"name": "authorization_code", "label": "Código de autorización", "required": true}
                        ]
                    })
                } else {
                    json!({
                        "title": "¿Seguro que quieres hacer esto?",
                        "question": "¿Seguro que quieres hacer esto? [y/n]",
                        "proposed_action": {
                            "tool": "offensive_security_action",
                            "arguments": {"action": "port_scan", "target": "127.0.0.1"}
                        }
                    })
                };
                let frame = json!({
                    "status": "requires_auth",
                    "mode": mode,
                    "request_id": id,
                    "form": form
                });
                write_frame(&mut write_half, &frame).await?;
                return Ok(());
            }

            let answer = format!(
                "He recibido su consulta: {query}. Esta respuesta procede del daemon de prueba. \
                 Sustituya este proceso por el daemon Hyrulor para conectar el modelo y las herramientas reales."
            );
            for chunk in answer.split_inclusive(' ') {
                let frame = json!({"status": "streaming", "data": {"delta": chunk}});
                write_frame(&mut write_half, &frame).await?;
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
            let frame = json!({"status": "ok", "data": {"answer": answer}});
            write_frame(&mut write_half, &frame).await?;
        }
        "get_mode" => {
            let mode = if state.secure.load(Ordering::Relaxed) {
                "secure"
            } else {
                "simple"
            };
            write_frame(
                &mut write_half,
                &json!({"status": "ok", "data": {"mode": mode}}),
            )
            .await?;
        }
        "set_mode" => {
            state
                .secure
                .store(request.secure.unwrap_or(false), Ordering::Relaxed);
            let mode = if state.secure.load(Ordering::Relaxed) {
                "secure"
            } else {
                "simple"
            };
            write_frame(
                &mut write_half,
                &json!({"status": "ok", "data": {"mode": mode}}),
            )
            .await?;
        }
        "authorize" => {
            let approved = request.approved.unwrap_or(false);
            let answer = if approved {
                "✅ Acción autorizada y ejecutada en el sandbox de prueba (dry-run).".to_owned()
            } else {
                "🛑 Acción cancelada por el usuario. No se ejecutó nada.".to_owned()
            };
            write_frame(
                &mut write_half,
                &json!({"status": "ok", "data": {"answer": answer, "authorized": approved}}),
            )
            .await?;
        }
        "describe_image" => {
            let path = request.path.unwrap_or_default();
            let prompt = request.prompt.unwrap_or_default();
            write_frame(
                &mut write_half,
                &json!({
                    "status": "ok",
                    "data": {"answer": format!("Imagen de prueba «{path}» analizada. Pregunta: {prompt}")}
                }),
            )
            .await?;
        }
        "transcribe" | "listen" => {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            write_frame(
                &mut write_half,
                &json!({"status": "ok", "data": {"text": "Hola desde el audio de prueba."}}),
            )
            .await?;
        }
        "speak" => {
            write_frame(
                &mut write_half,
                &json!({"status": "ok", "data": {"played": true, "audio": null}}),
            )
            .await?;
        }
        other => {
            write_frame(
                &mut write_half,
                &json!({"status": "error", "message": format!("método no soportado: {other}")}),
            )
            .await?;
        }
    }

    Ok(())
}

fn looks_offensive(query: &str) -> bool {
    let query = query.to_lowercase();
    [
        "escanea", "escaneo", "exploit", "pentest", "ataque", "ofensiva",
    ]
    .iter()
    .any(|needle| query.contains(needle))
}

async fn write_frame(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    frame: &Value,
) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(frame)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}
