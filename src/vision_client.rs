use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    time::timeout,
};

const MAX_IMAGE_BYTES: usize = 25 * 1024 * 1024;
const VISION_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, thiserror::Error)]
pub enum VisionError {
    #[error("vision io: {0}")]
    Io(#[from] std::io::Error),
    #[error("vision json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("vision HTTP {status}: {body}")]
    Http { status: String, body: String },
    #[error("vision returned no text")]
    Empty,
    #[error("image too large: {0} bytes (máximo 25 MiB)")]
    TooLarge(usize),
    #[error("unsupported or unreadable image: {0}")]
    Unsupported(String),
}

#[derive(Clone)]
pub struct VisionClient {
    socket: PathBuf,
    model: String,
}

impl VisionClient {
    pub fn new(socket: PathBuf, model: String) -> Self {
        Self { socket, model }
    }

    pub async fn describe(&self, image: &Path, prompt: &str) -> Result<String, VisionError> {
        let bytes = tokio::fs::read(image)
            .await
            .map_err(|error| VisionError::Unsupported(format!("{}: {error}", image.display())))?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(VisionError::TooLarge(bytes.len()));
        }

        let mime = detect_mime(&bytes, image);
        let data_url = format!("data:{mime};base64,{}", base64_encode(&bytes));

        let body = serde_json::to_vec(&json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": prompt},
                        {"type": "image_url", "image_url": {"url": data_url}}
                    ]
                }
            ],
            "temperature": 0.2,
            "max_tokens": 1024,
            "chat_template_kwargs": {"enable_thinking": false}
        }))?;

        let request = format!(
            "POST /v1/chat/completions HTTP/1.1\r\nHost: localhost\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );

        let socket = self.socket.clone();
        let response = timeout(VISION_TIMEOUT, async move {
            let mut stream = UnixStream::connect(socket).await?;
            stream.write_all(request.as_bytes()).await?;
            stream.write_all(&body).await?;
            let mut response = Vec::new();
            stream.read_to_end(&mut response).await?;
            Ok::<Vec<u8>, std::io::Error>(response)
        })
        .await
        .map_err(|_| {
            VisionError::Unsupported("el servidor de visión agotó el tiempo".to_string())
        })??;

        let payload = http_body(&response)?;
        let value: Value = serde_json::from_slice(payload)?;
        let content = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();

        if content.is_empty() {
            return Err(VisionError::Empty);
        }
        Ok(content)
    }
}

fn http_body(response: &[u8]) -> Result<&[u8], VisionError> {
    let separator = b"\r\n\r\n";
    let header_end = response
        .windows(separator.len())
        .position(|window| window == separator)
        .ok_or_else(|| VisionError::Http {
            status: "malformed".to_string(),
            body: "missing HTTP header separator".to_string(),
        })?;
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status = headers.split_whitespace().nth(1).unwrap_or("500");
    let body = &response[header_end + separator.len()..];
    if status != "200" {
        return Err(VisionError::Http {
            status: status.to_string(),
            body: String::from_utf8_lossy(body).into_owned(),
        });
    }
    Ok(body)
}

fn detect_mime(bytes: &[u8], path: &Path) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else if bytes.starts_with(b"BM") {
        "image/bmp"
    } else {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            Some("bmp") => "image/bmp",
            _ => "image/png",
        }
    }
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity((input.len() + 2) / 3 * 4);

    for chunk in input.chunks(3) {
        let first = chunk[0] as u32;
        let second = *chunk.get(1).unwrap_or(&0) as u32;
        let third = *chunk.get(2).unwrap_or(&0) as u32;
        let combined = (first << 16) | (second << 8) | third;

        output.push(TABLE[((combined >> 18) & 0x3f) as usize] as char);
        output.push(TABLE[((combined >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[((combined >> 6) & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(combined & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{base64_encode, detect_mime};
    use std::path::Path;

    #[test]
    fn encodes_base64_padding_correctly() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"hola"), "aG9sYQ==");
    }

    #[test]
    fn detects_image_type_from_magic_bytes() {
        let png = b"\x89PNG\r\n\x1a\nrest";
        let jpeg = b"\xff\xd8\xff\xe0rest";
        let gif = b"GIF89arest";
        let webp = b"RIFF\x00\x00\x00\x00WEBPrest";

        assert_eq!(detect_mime(png, Path::new("x.bin")), "image/png");
        assert_eq!(detect_mime(jpeg, Path::new("x.bin")), "image/jpeg");
        assert_eq!(detect_mime(gif, Path::new("x.bin")), "image/gif");
        assert_eq!(detect_mime(webp, Path::new("x.bin")), "image/webp");
    }

    #[test]
    fn falls_back_to_extension_when_magic_is_unknown() {
        assert_eq!(detect_mime(b"plain", Path::new("foto.JPG")), "image/jpeg");
        assert_eq!(detect_mime(b"plain", Path::new("foto.bmp")), "image/bmp");
        assert_eq!(
            detect_mime(b"plain", Path::new("foto.unknown")),
            "image/png"
        );
    }
}
