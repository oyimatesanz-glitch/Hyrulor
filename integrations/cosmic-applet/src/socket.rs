use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::SinkExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::{sleep, timeout};

const DEFAULT_SOCKET: &str = "/tmp/omni.sock";
const MAX_CONNECT_ATTEMPTS: usize = 7;
const MAX_BACKOFF: Duration = Duration::from_secs(30);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(90);
const MAX_FRAME_BYTES: usize = 1_048_576;
const MAX_RESPONSE_BYTES: usize = 4 * 1_048_576;

#[derive(Debug, Clone)]
pub enum SocketEvent {
    ToolCall(String),
    Chunk(String),
    Finished(String),
    Failed(String),
}

#[derive(Serialize)]
struct RpcRequest<'a> {
    request: &'static str,
    query: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RpcResponse {
    status: String,
    data: Option<RpcData>,
    message: Option<String>,
}

impl Default for RpcResponse {
    fn default() -> Self {
        Self {
            status: String::new(),
            data: None,
            message: None,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct RpcData {
    answer: Option<String>,
    delta: Option<String>,
    chunk: Option<String>,
    tool: Option<String>,
    label: Option<String>,
}

/// Sends a JSON-RPC-style line and forwards every streaming frame to the UI.
/// The daemon is expected to create a user-owned Unix socket with mode 0600.
pub async fn request_stream(
    query: String,
    sender: &mut futures::channel::mpsc::Sender<SocketEvent>,
) -> Result<(), String> {
    let path = socket_path();
    let mut stream = connect_with_backoff(&path)
        .await
        .map_err(|error| format!("No se pudo abrir {}: {error}", path.display()))?;

    let request = RpcRequest {
        request: "chat",
        query: &query,
    };
    let mut payload = serde_json::to_vec(&request)
        .map_err(|error| format!("No se pudo serializar la petición: {error}"))?;
    payload.push(b'\n');

    stream
        .write_all(&payload)
        .await
        .map_err(|error| format!("No se pudo enviar la petición: {error}"))?;
    stream
        .flush()
        .await
        .map_err(|error| format!("No se pudo vaciar el socket: {error}"))?;

    let mut reader = BufReader::new(stream);
    let mut accumulated = String::new();

    loop {
        let line = timeout(
            RESPONSE_TIMEOUT,
            read_bounded_line(&mut reader, MAX_FRAME_BYTES),
        )
        .await
        .map_err(|_| "El daemon ha superado el tiempo máximo de respuesta.".to_owned())?
        .map_err(|error| format!("Error leyendo la respuesta del daemon: {error}"))?;

        let Some(line) = line else {
            if !accumulated.is_empty() {
                let _ = sender.send(SocketEvent::Finished(accumulated)).await;
                return Ok(());
            }
            return Err("El daemon cerró el socket antes de enviar una respuesta.".to_owned());
        };

        let frame: RpcResponse = serde_json::from_str(line.trim_end())
            .map_err(|error| format!("El daemon envió JSON no válido: {error}"))?;
        let status = frame.status.to_ascii_lowercase();
        let data = frame.data.unwrap_or_default();

        if status == "error" {
            return Err(frame
                .message
                .unwrap_or_else(|| "El daemon rechazó la petición.".to_owned()));
        }

        if status == "tool" || status == "tool_call" || status == "executing" {
            let label = data
                .tool
                .or(data.label)
                .unwrap_or_else(|| "Consultando herramientas…".to_owned());
            let _ = sender.send(SocketEvent::ToolCall(label)).await;
            continue;
        }

        if status == "chunk" || status == "streaming" || status == "token" || status == "delta" {
            if let Some(chunk) = data.delta.or(data.chunk).or(data.answer) {
                if accumulated.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                    return Err("La respuesta supera el límite total de 4 MiB.".to_owned());
                }
                accumulated.push_str(&chunk);
                let _ = sender.send(SocketEvent::Chunk(chunk)).await;
            }
            continue;
        }

        if status == "ok" || status == "done" || status == "complete" {
            let answer = data.answer.unwrap_or(accumulated);
            if answer.len() > MAX_RESPONSE_BYTES {
                return Err("La respuesta supera el límite total de 4 MiB.".to_owned());
            }
            let _ = sender.send(SocketEvent::Finished(answer)).await;
            return Ok(());
        }

        // Forward a content frame even when the daemon uses a custom status name.
        if let Some(chunk) = data.delta.or(data.chunk) {
            if accumulated.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err("La respuesta supera el límite total de 4 MiB.".to_owned());
            }
            accumulated.push_str(&chunk);
            let _ = sender.send(SocketEvent::Chunk(chunk)).await;
        } else if let Some(answer) = data.answer {
            if answer.len() > MAX_RESPONSE_BYTES {
                return Err("La respuesta supera el límite total de 4 MiB.".to_owned());
            }
            accumulated = answer.clone();
            let _ = sender.send(SocketEvent::Chunk(answer)).await;
        }
    }
}

fn socket_path() -> PathBuf {
    std::env::var_os("HYORY_SOCKET")
        .or_else(|| std::env::var_os("HYRULOR_SOCKET"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET))
}

async fn connect_with_backoff(path: &Path) -> io::Result<UnixStream> {
    let mut delay = Duration::from_millis(250);
    let mut last_error = None;

    for attempt in 0..MAX_CONNECT_ATTEMPTS {
        match verify_socket(path) {
            Ok(()) => match UnixStream::connect(path).await {
                Ok(stream) => return Ok(stream),
                Err(error) => last_error = Some(error),
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }

        if attempt + 1 < MAX_CONNECT_ATTEMPTS {
            sleep(delay).await;
            delay = delay.saturating_mul(2).min(MAX_BACKOFF);
        }
    }

    Err(last_error.unwrap_or_else(|| {
        io::Error::new(io::ErrorKind::NotConnected, "no se pudo conectar al daemon")
    }))
}

/// Refuses symlinks, sockets owned by another user, and group/world-accessible
/// sockets. This protects the applet from connecting to an unexpected endpoint.
fn verify_socket(path: &Path) -> io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;

    if !metadata.file_type().is_socket() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "la ruta existe, pero no es un socket Unix",
        ));
    }

    // SAFETY: geteuid has no preconditions and only reads the effective UID.
    let current_uid = unsafe { libc::geteuid() } as u32;
    if metadata.uid() != current_uid {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "el socket no pertenece al usuario actual",
        ));
    }

    if metadata.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "el socket debe tener permisos 0600 (sin acceso de grupo ni otros)",
        ));
    }

    Ok(())
}

/// Reads one newline-delimited frame without ever allocating more than `max_bytes`.
/// A final unterminated frame is accepted when the peer closes the stream.
async fn read_bounded_line<R>(reader: &mut R, max_bytes: usize) -> io::Result<Option<String>>
where
    R: AsyncBufRead + Unpin,
{
    let mut bytes = Vec::with_capacity(max_bytes);

    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            return String::from_utf8(bytes)
                .map(Some)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
        }

        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |position| position + 1);
        if bytes.len().saturating_add(take) > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "la respuesta supera el límite de 1 MiB por línea",
            ));
        }

        bytes.extend_from_slice(&available[..take]);
        reader.consume(take);

        if newline.is_some() {
            return String::from_utf8(bytes)
                .map(Some)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::read_bounded_line;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn reads_multiple_newline_delimited_frames() {
        let input: &[u8] = b"first\nsecond";
        let mut reader = BufReader::new(input);

        assert_eq!(
            read_bounded_line(&mut reader, 16).await.unwrap(),
            Some("first\n".to_owned())
        );
        assert_eq!(
            read_bounded_line(&mut reader, 16).await.unwrap(),
            Some("second".to_owned())
        );
        assert_eq!(read_bounded_line(&mut reader, 16).await.unwrap(), None);
    }

    #[tokio::test]
    async fn rejects_a_frame_before_exceeding_the_limit() {
        let input: &[u8] = b"12345\n";
        let mut reader = BufReader::new(input);

        let error = read_bounded_line(&mut reader, 5).await.unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
