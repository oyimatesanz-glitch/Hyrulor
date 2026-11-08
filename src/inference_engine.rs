use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    time::timeout,
};

#[derive(Debug, thiserror::Error)]
pub enum InferenceError {
    #[error("llm connection failed: {0}")]
    Connect(#[source] std::io::Error),
    #[error("llm request failed: {0}")]
    Io(#[source] std::io::Error),
    #[error("llm request timed out")]
    Timeout,
    #[error("llm returned HTTP {status}: {body}")]
    Http { status: String, body: String },
    #[error("invalid llm response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("llm response contained no choices")]
    EmptyChoices,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Clone)]
pub struct InferenceEngine {
    address: String,
    model: String,
}

impl InferenceEngine {
    pub fn new(address: String, model: String) -> Self {
        Self { address, model }
    }

    pub async fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        force_tool: bool,
    ) -> Result<ChatMessage, InferenceError> {
        let body = serde_json::to_vec(&serde_json::json!({
            "model": self.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": if force_tool { "required" } else { "auto" },
            "temperature": 0.1,
        }))?;

        let request = format!(
            "POST /v1/chat/completions HTTP/1.1\r\nHost: localhost\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );

        let address = self.address.clone();
        let result = timeout(Duration::from_secs(120), async move {
            let mut stream = UnixStream::connect(address)
                .await
                .map_err(InferenceError::Connect)?;

            stream
                .write_all(request.as_bytes())
                .await
                .map_err(InferenceError::Io)?;
            stream.write_all(&body).await.map_err(InferenceError::Io)?;

            let mut response = Vec::new();
            stream
                .read_to_end(&mut response)
                .await
                .map_err(InferenceError::Io)?;

            Ok::<Vec<u8>, InferenceError>(response)
        })
        .await
        .map_err(|_| InferenceError::Timeout)??;

        parse_response(&result)
    }
}

fn parse_response(response: &[u8]) -> Result<ChatMessage, InferenceError> {
    let separator = b"\r\n\r\n";
    let header_end = response
        .windows(separator.len())
        .position(|window| window == separator)
        .ok_or_else(|| InferenceError::Http {
            status: "malformed".to_string(),
            body: "missing HTTP header separator".to_string(),
        })?;
    let headers = String::from_utf8_lossy(&response[..header_end]);
    let status = headers
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| InferenceError::Http {
            status: "malformed".to_string(),
            body: "missing HTTP status".to_string(),
        })?;
    let body = &response[header_end + separator.len()..];
    if status != "200" {
        return Err(InferenceError::Http {
            status: status.to_string(),
            body: String::from_utf8_lossy(body).into_owned(),
        });
    }
    let parsed: ChatResponse = serde_json::from_slice(body)?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message)
        .ok_or(InferenceError::EmptyChoices)
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            kind: "function",
            function: FunctionDefinition {
                name: "check_disk_space",
                description: "Get the free and used disk space of the system.",
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
        },
        ToolDefinition {
            kind: "function",
            function: FunctionDefinition {
                name: "check_memory",
                description: "Get the RAM memory usage of the system.",
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
        },
        ToolDefinition {
            kind: "function",
            function: FunctionDefinition {
                name: "list_processes",
                description: "List the top CPU consuming processes.",
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
        },
        ToolDefinition {
            kind: "function",
            function: FunctionDefinition {
                name: "battery_status",
                description: "Get the battery level and health.",
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
        },
        ToolDefinition {
            kind: "function",
            function: FunctionDefinition {
                name: "gpu_diag",
                description: "Get GPU information and diagnostics.",
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_response;

    #[test]
    fn parses_chat_completion_from_unix_http_response() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n\
            {\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"ok\"}}]}";
        let message = match parse_response(response) {
            Ok(message) => message,
            Err(error) => panic!("valid response expected: {error}"),
        };
        assert_eq!(message.role, "assistant");
        assert_eq!(message.content.as_deref(), Some("ok"));
    }

    #[test]
    fn rejects_http_errors() {
        let response = b"HTTP/1.1 400 Bad Request\r\n\r\n{\"error\":\"bad request\"}";
        assert!(parse_response(response).is_err());
    }
}
