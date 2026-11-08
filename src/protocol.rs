use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use crate::inference_engine::{tool_definitions, ChatMessage, InferenceEngine};
use crate::{memory_vault::MemoryVault, tool_sandbox::ToolSandbox};

const MAX_REQUEST_BYTES: usize = 8192;

pub struct AppState {
    pub vault: MemoryVault,
    pub sandbox: ToolSandbox,
    pub inference: InferenceEngine,
    pub coder: Option<InferenceEngine>,
    pub embedding_client: crate::embedding_client::EmbeddingClient,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case")]
enum Request {
    Remember { user: String, assistant: String },
    Search { query: String, limit: Option<u32> },
    Tool { name: String },
    Chat { query: String },
    Index { path: String },
    Health,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Response {
    Ok { data: Value },
    Error { message: String },
}

pub async fn handle_connection(
    stream: UnixStream,
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        if line.len() > MAX_REQUEST_BYTES {
            let response = Response::Error {
                message: format!("request payload exceeds {MAX_REQUEST_BYTES} bytes"),
            };
            writer.write_all(&serde_json::to_vec(&response)?).await?;
            writer.write_all(b"\n").await?;
            continue;
        }

        let response = match serde_json::from_str::<Request>(&line) {
            Ok(request) => dispatch(request, &state).await,
            Err(error) => Response::Error {
                message: format!("invalid request: {error}"),
            },
        };
        let encoded = serde_json::to_vec(&response)?;
        writer.write_all(&encoded).await?;
        writer.write_all(b"\n").await?;
    }
    Ok(())
}

async fn dispatch(request: Request, state: &AppState) -> Response {
    match request {
        Request::Health => Response::Ok {
            data: serde_json::json!({"service": "hyory", "version": env!("CARGO_PKG_VERSION")}),
        },
        Request::Remember { user, assistant } => {
            match state.vault.save_conversation(user, assistant).await {
                Ok(id) => Response::Ok {
                    data: serde_json::json!({"id": id}),
                },
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Request::Search { query, limit } => {
            match state.vault.search(query, limit.unwrap_or(3)).await {
                Ok(hits) => Response::Ok {
                    data: match serde_json::to_value(hits) {
                        Ok(value) => value,
                        Err(error) => {
                            return Response::Error {
                                message: format!("serialization error: {error}"),
                            };
                        }
                    },
                },
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Request::Tool { name } => match state.sandbox.exec_tool(&name, "{}").await {
            Ok(result) => Response::Ok {
                data: match serde_json::to_value(result) {
                    Ok(value) => value,
                    Err(error) => {
                        return Response::Error {
                            message: format!("serialization error: {error}"),
                        };
                    }
                },
            },
            Err(error) => Response::Error {
                message: error.to_string(),
            },
        },

        Request::Index { path } => {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    return Response::Error {
                        message: format!("Error leyendo archivo: {}", e),
                    }
                }
            };

            let chunks = content
                .chars()
                .collect::<Vec<_>>()
                .chunks(500)
                .map(|chunk| chunk.iter().collect::<String>())
                .filter(|chunk| !chunk.trim().is_empty())
                .collect::<Vec<_>>();
            let mut indexed = 0usize;

            for chunk in chunks {
                let embedding = match state.embedding_client.get_embedding(&chunk).await {
                    Ok(embedding) => embedding,
                    Err(error) => {
                        return Response::Error {
                            message: format!("Error embedding: {error}"),
                        }
                    }
                };
                if let Err(error) = state.vault.insert_document(chunk, embedding).await {
                    return Response::Error {
                        message: error.to_string(),
                    };
                }
                indexed += 1;
            }

            Response::Ok {
                data: serde_json::json!({"status": "indexed", "path": path, "chunks": indexed}),
            }
        }

        Request::Chat { query } => match run_agent(&query, state).await {
            Ok(answer) => Response::Ok {
                data: serde_json::json!({"answer": answer}),
            },
            Err(error) => Response::Error {
                message: error.to_string(),
            },
        },
    }
}

#[derive(Debug, thiserror::Error)]
enum AgentError {
    #[error("memory: {0}")]
    Memory(#[from] crate::memory_vault::MemoryError),
    #[error("inference: {0}")]
    Inference(#[from] crate::inference_engine::InferenceError),
    #[error("tool: {0}")]
    Tool(#[from] crate::tool_sandbox::ToolError),
    #[error("agent produced no final answer")]
    EmptyAnswer,
}

async fn run_agent(query: &str, state: &AppState) -> Result<String, AgentError> {
    let q_lower = query.to_lowercase();

    // 1. CLASIFICADOR DE INTENCIÓN
    let es_charla_casual = q_lower.starts_with("hola")
        || q_lower.starts_with("buenos")
        || q_lower.starts_with("qué tal")
        || q_lower.contains("cómo estás")
        || q_lower.contains("funcionando bien");

    // BYPASS DE CORTESÍA: Si es un saludo, cero herramientas y respuesta directa.
    if es_charla_casual {
        eprintln!("👋 Modo charla casual (Cero herramientas)");
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some("Eres Hyrulor, un asistente de IA local. Responde de forma amigable, breve y natural al saludo del usuario.".to_string()),
                tool_calls: Vec::new(),
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: Some(query.to_string()),
                tool_calls: Vec::new(),
                tool_call_id: None,
            },
        ];
        let engine = state.coder.as_ref().unwrap_or(&state.inference);
        let response = engine.chat(&messages, &[], false).await?;
        return Ok(response.content.unwrap_or_default().trim().to_string());
    }

    let es_pregunta_codigo = q_lower.contains("main")
        || q_lower.contains("fn ")
        || q_lower.contains("código")
        || q_lower.contains("archivo")
        || q_lower.contains("rust")
        || q_lower.contains("función");

    // 2. FASE RAG (Solo si pregunta de código)
    if es_pregunta_codigo {
        let rag_docs = match state.embedding_client.get_embedding(query).await {
            Ok(emb) => state
                .vault
                .search_documents(emb, 3)
                .await
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        if !rag_docs.is_empty() {
            eprintln!("🔍 RAG activado: {} fragmentos", rag_docs.len());
            let context = rag_docs.join(
                "

---

",
            );
            let messages = vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: Some(format!(
                        "Eres Hyrulor, experto en código. Responde la pregunta usando ÚNICAMENTE este código.

== CÓDIGO ==
{}
== FIN ==",
                        context
                    )),
                    tool_calls: Vec::new(),
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: Some(query.to_string()),
                    tool_calls: Vec::new(),
                    tool_call_id: None,
                },
            ];
            let engine = state.coder.as_ref().unwrap_or(&state.inference);
            let response = engine.chat(&messages, &[], false).await?;
            let answer = response.content.unwrap_or_default().trim().to_string();
            if !answer.is_empty() {
                let _ = state
                    .vault
                    .save_conversation(query.to_string(), answer.clone())
                    .await;
                return Ok(answer);
            }
        }
    }

    // 3. FASE HERRAMIENTAS / CHARLA (Flujo normal)
    eprintln!("🔧 Modo herramientas (sin RAG)");
    let memories = state
        .vault
        .search(query.to_string(), 2)
        .await
        .unwrap_or_default();
    let memory_context = if memories.is_empty() {
        String::new()
    } else {
        format!(
            "Memoria:
{}",
            serde_json::to_string(&memories).unwrap_or_default()
        )
    };

    let tools = tool_definitions();

    let mut messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: Some(format!(
                "You are Hyory, a local Linux assistant. CRITICAL RULE: If the user asks about hardware, system status, or disk space, you MUST call the relevant tool to get the real data. NEVER tell the user to run a command themselves. Execute the tool and give them the result. Memory: {memory_context}"
            )),
            tool_calls: Vec::new(),
            tool_call_id: None,
        },
        ChatMessage {
            role: "user".to_string(),
            content: Some(query.to_string()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        },
    ];

    for round in 0..5 {
        let force_tool = round == 0;
        let response = state.inference.chat(&messages, &tools, force_tool).await?;

        if response.tool_calls.is_empty() {
            let answer = response.content.unwrap_or_default().trim().to_string();
            if answer.is_empty() {
                return Err(AgentError::EmptyAnswer);
            }
            let _ = state
                .vault
                .save_conversation(query.to_string(), answer.clone())
                .await;
            return Ok(answer);
        }

        let assistant_calls = response.tool_calls.clone();
        messages.push(response);
        for call in assistant_calls {
            if !is_allowed_tool(&call.function.name) {
                messages.push(tool_message(
                    call.id,
                    format!("Error: herramienta no permitida: {}.", call.function.name),
                ));
                continue;
            }
            let output = match state
                .sandbox
                .exec_tool(&call.function.name, &call.function.arguments)
                .await
            {
                Ok(result) => result.output,
                Err(error) => format!("Error del sandbox: {error}"),
            };
            messages.push(ChatMessage {
                role: "tool".to_string(),
                content: Some(output),
                tool_calls: Vec::new(),
                tool_call_id: Some(call.id),
            });
        }
    }

    Err(AgentError::EmptyAnswer)
}

fn is_allowed_tool(name: &str) -> bool {
    matches!(
        name,
        "check_disk_space"
            | "check_memory"
            | "list_processes"
            | "battery_status"
            | "gpu_diag"
            | "read_file"
            | "search_code"
            | "system_info"
            | "date_time"
            | "ping_host"
    )
}

#[allow(dead_code)]
fn validate_tool_arguments(arguments: &str) -> Result<(), &'static str> {
    let value: Value = serde_json::from_str(arguments).map_err(|_| "deben ser un objeto JSON")?;
    let object = value.as_object().ok_or("deben ser un objeto JSON")?;
    if object.is_empty() {
        Ok(())
    } else {
        Err("las herramientas actuales no aceptan argumentos")
    }
}

fn tool_message(tool_call_id: String, content: String) -> ChatMessage {
    ChatMessage {
        role: "tool".to_string(),
        content: Some(content),
        tool_calls: Vec::new(),
        tool_call_id: Some(tool_call_id),
    }
}

#[cfg(test)]
mod tests {
    use super::{is_allowed_tool, validate_tool_arguments};

    #[test]
    fn allows_only_declared_tools() {
        assert!(is_allowed_tool("check_disk_space"));
        assert!(is_allowed_tool("check_memory"));
        assert!(is_allowed_tool("list_processes"));
        assert!(is_allowed_tool("battery_status"));
        assert!(is_allowed_tool("gpu_diag"));
        assert!(!is_allowed_tool("net_privacy"));
        assert!(!is_allowed_tool("rm_rf_root"));
    }

    #[test]
    fn accepts_only_json_objects_as_tool_arguments() {
        assert!(validate_tool_arguments("{}").is_ok());
        assert!(validate_tool_arguments("{\"level\":80}").is_err());
        assert!(validate_tool_arguments("nivel 80").is_err());
        assert!(validate_tool_arguments("[]").is_err());
    }

    #[test]
    fn rejects_non_empty_arguments_for_current_tools() {
        assert_eq!(
            validate_tool_arguments("{\"unexpected\":true}"),
            Err("las herramientas actuales no aceptan argumentos")
        );
    }
}
