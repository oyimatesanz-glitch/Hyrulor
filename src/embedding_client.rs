use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http error: {status}")]
    Http { status: String },
    #[error("invalid response")]
    InvalidResponse,
}

#[derive(Debug, Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Clone)]
pub struct EmbeddingClient {
    address: String,
}

impl EmbeddingClient {
    pub fn new(address: String) -> Self {
        Self { address }
    }

    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let body = serde_json::to_vec(&serde_json::json!({
            "model": "Qwen3-Embedding-0.6B",
            "input": text
        }))?;

        let request = format!(
            "POST /v1/embeddings HTTP/1.1\r\nHost: localhost\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );

        let mut stream = UnixStream::connect(&self.address).await?;
        stream.write_all(request.as_bytes()).await?;
        stream.write_all(&body).await?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;

        let separator = b"\r\n\r\n";
        let header_end = response
            .windows(separator.len())
            .position(|w| w == separator)
            .ok_or(EmbedError::InvalidResponse)?;

        let headers = String::from_utf8_lossy(&response[..header_end]);
        let status = headers.split_whitespace().nth(1).unwrap_or("500");
        if status != "200" {
            return Err(EmbedError::Http {
                status: status.to_string(),
            });
        }

        let body_bytes = &response[header_end + separator.len()..];
        let parsed: EmbedResponse = serde_json::from_slice(body_bytes)?;

        parsed
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or(EmbedError::InvalidResponse)
    }
}
