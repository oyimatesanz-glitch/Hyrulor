//! Small local daemon used only to test the applet UI and JSON-lines protocol.

use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

const SOCKET_PATH: &str = "/tmp/omni.sock";

#[derive(Debug, Deserialize)]
struct Request {
    request: String,
    query: String,
}

#[derive(Serialize)]
struct Data<'a> {
    delta: Option<&'a str>,
    answer: Option<&'a str>,
}

#[derive(Serialize)]
struct Frame<'a> {
    status: &'a str,
    data: Data<'a>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    remove_stale_socket(Path::new(SOCKET_PATH))?;
    let listener = UnixListener::bind(SOCKET_PATH)?;
    std::fs::set_permissions(SOCKET_PATH, std::fs::Permissions::from_mode(0o600))?;

    println!("Hyrulor mock daemon escuchando en {SOCKET_PATH} (0600)");
    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(error) = handle(stream).await {
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

async fn handle(stream: UnixStream) -> io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    let request: Request = serde_json::from_str(line.trim_end())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    if request.request != "chat" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "método no soportado",
        ));
    }

    let answer = format!(
        "He recibido su consulta: {}. Esta respuesta procede del daemon de prueba. Sustituya este proceso por el daemon Hyrulor para conectar el modelo y las herramientas reales.",
        request.query
    );

    // Send several JSON-lines frames to exercise the applet's streaming renderer.
    for chunk in answer.split_inclusive(' ') {
        let frame = Frame {
            status: "streaming",
            data: Data {
                delta: Some(chunk),
                answer: None,
            },
        };
        write_frame(&mut write_half, &frame).await?;
        tokio::time::sleep(std::time::Duration::from_millis(35)).await;
    }

    let final_frame = Frame {
        status: "ok",
        data: Data {
            delta: None,
            answer: Some(&answer),
        },
    };
    write_frame(&mut write_half, &final_frame).await?;
    Ok(())
}

async fn write_frame(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    frame: &Frame<'_>,
) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(frame)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await?;
    writer.flush().await
}
