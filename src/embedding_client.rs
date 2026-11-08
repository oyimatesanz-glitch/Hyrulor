use serde::Deserialize;
use std::path::PathBuf;
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

#[derive(Debug, Deserialize)]
struct RerankResult {
    index: usize,
    #[serde(default)]
    relevance_score: f64,
}

#[derive(Debug, Deserialize)]
struct RerankResponse {
    results: Vec<RerankResult>,
}

#[derive(Clone)]
pub struct EmbeddingClient {
    socket_path: PathBuf,
    reranker_socket: Option<PathBuf>,
}

impl EmbeddingClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            reranker_socket: None,
        }
    }

    /// El Reranker vive en su propio `llama-server` (`--reranking`),
    /// por lo que necesita un socket distinto al de embeddings.
    pub fn with_reranker(mut self, socket: Option<PathBuf>) -> Self {
        self.reranker_socket = socket;
        self
    }

    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let body = serde_json::to_vec(&serde_json::json!({
            "model": "Qwen3-Embedding-0.6B",
            "input": text
        }))?;

        let response = post_json(&self.socket_path, "/v1/embeddings", &body).await?;
        let parsed: EmbedResponse = serde_json::from_slice(&response)?;

        parsed
            .data
            .into_iter()
            .next()
            .map(|data| data.embedding)
            .ok_or(EmbedError::InvalidResponse)
    }

    /// Reordena los documentos por relevancia real usando `/v1/rerank`.
    /// Si el reranker no está disponible, devuelve el orden original
    /// (degradación segura: nunca rompe el RAG).
    pub async fn rerank(
        &self,
        query: &str,
        documents: Vec<String>,
    ) -> Result<Vec<String>, EmbedError> {
        if documents.len() < 2 {
            return Ok(documents);
        }
        let Some(socket) = self.reranker_socket.as_ref() else {
            return Ok(documents);
        };

        let body = serde_json::to_vec(&serde_json::json!({
            "model": "Qwen3-Reranker-0.6B",
            "query": query,
            "documents": documents
        }))?;

        let response = match post_json(socket, "/v1/rerank", &body).await {
            Ok(response) => response,
            Err(error) => {
                eprintln!(
                    "⚠️  Reranker no disponible ({error}); se mantiene el orden del embedding."
                );
                return Ok(documents);
            }
        };

        let Ok(parsed) = serde_json::from_slice::<RerankResponse>(&response) else {
            return Ok(documents);
        };

        let mut scored: Vec<(usize, f64)> = parsed
            .results
            .iter()
            .map(|result| (result.index, result.relevance_score))
            .collect();
        scored.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut reordered = Vec::with_capacity(documents.len());
        for (index, _) in scored {
            if let Some(document) = documents.get(index) {
                reordered.push(document.clone());
            }
        }

        if reordered.len() != documents.len() {
            return Ok(documents);
        }
        Ok(reordered)
    }
}

async fn post_json(socket: &PathBuf, endpoint: &str, body: &[u8]) -> Result<Vec<u8>, EmbedError> {
    let request = format!(
        "POST {endpoint} HTTP/1.1\r\nHost: localhost\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );

    let mut stream = UnixStream::connect(socket).await?;
    stream.write_all(request.as_bytes()).await?;
    stream.write_all(body).await?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;

    let separator = b"\r\n\r\n";
    let header_end = response
        .windows(separator.len())
        .position(|window| window == separator)
        .ok_or(EmbedError::InvalidResponse)?;

    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status = headers.split_whitespace().nth(1).unwrap_or("500");
    if status != "200" {
        return Err(EmbedError::Http {
            status: status.to_string(),
        });
    }

    Ok(response[header_end + separator.len()..].to_vec())
}
