use std::{
    collections::HashMap,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use crate::{
    audit::AuditLog,
    embedding_client::EmbeddingClient,
    guard_client::GuardClient,
    inference_engine::{tool_definitions, ChatMessage, InferenceEngine},
    memory_vault::MemoryVault,
    tool_sandbox::ToolSandbox,
    vision_client::VisionClient,
    voice::VoiceEngine,
};

const MAX_REQUEST_BYTES: usize = 8192;
pub const OFFENSIVE_TOOL: &str = "offensive_security_action";

const SECURE_FORM_FIELDS: [&str; 6] = [
    "target",
    "scope",
    "justification",
    "start_time",
    "end_time",
    "authorization_code",
];

pub struct AppState {
    pub vault: MemoryVault,
    pub sandbox: ToolSandbox,
    pub inference: InferenceEngine,
    pub coder: Option<InferenceEngine>,
    pub embedding_client: EmbeddingClient,
    pub guard: GuardClient,
    pub audit: AuditLog,
    pub vision: Option<VisionClient>,
    pub voice: Option<VoiceEngine>,
    secure: AtomicBool,
    pub mode_file: PathBuf,
    pub pending_auth: tokio::sync::Mutex<HashMap<String, PendingAuth>>,
    pub auth_counter: AtomicU64,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vault: MemoryVault,
        sandbox: ToolSandbox,
        inference: InferenceEngine,
        coder: Option<InferenceEngine>,
        embedding_client: EmbeddingClient,
        guard: GuardClient,
        audit: AuditLog,
        vision: Option<VisionClient>,
        voice: Option<VoiceEngine>,
        secure_mode: bool,
        mode_file: PathBuf,
    ) -> Self {
        Self {
            vault,
            sandbox,
            inference,
            coder,
            embedding_client,
            guard,
            audit,
            vision,
            voice,
            secure: AtomicBool::new(secure_mode),
            mode_file,
            pending_auth: tokio::sync::Mutex::new(HashMap::new()),
            auth_counter: AtomicU64::new(1),
        }
    }

    pub fn secure_mode(&self) -> bool {
        self.secure.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub struct PendingAuth {
    pub tool: String,
    pub arguments: String,
    pub code: String,
    pub secure: bool,
    pub created_at: u64,
}

const PENDING_AUTH_TTL_MS: u64 = 15 * 60 * 1000;

#[derive(Debug, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case")]
enum Request {
    Remember {
        user: String,
        assistant: String,
    },
    Search {
        query: String,
        limit: Option<u32>,
    },
    Tool {
        name: String,
    },
    Chat {
        query: String,
    },
    Index {
        path: String,
    },
    DescribeImage {
        path: String,
        #[serde(default)]
        prompt: Option<String>,
    },
    Transcribe {
        path: String,
    },
    Listen {
        #[serde(default)]
        seconds: Option<u32>,
    },
    Speak {
        text: String,
    },
    GetMode,
    SetMode {
        secure: bool,
    },
    Authorize {
        id: String,
        approved: bool,
        #[serde(default)]
        form: Option<Value>,
    },
    Health,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Response {
    Ok {
        data: Value,
    },
    Error {
        message: String,
    },
    RequiresAuth {
        mode: &'static str,
        request_id: String,
        form: Value,
    },
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
            data: json!({"service": "hyory", "version": env!("CARGO_PKG_VERSION")}),
        },
        Request::GetMode => Response::Ok {
            data: mode_payload(state.secure_mode()),
        },
        Request::SetMode { secure } => set_mode(secure, state).await,
        Request::Remember { user, assistant } => {
            match state.vault.save_conversation(user, assistant).await {
                Ok(id) => Response::Ok {
                    data: json!({"id": id}),
                },
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Request::Search { query, limit } => {
            match state.vault.search(query, limit.unwrap_or(3)).await {
                Ok(hits) => match serde_json::to_value(hits) {
                    Ok(value) => Response::Ok { data: value },
                    Err(error) => Response::Error {
                        message: format!("serialization error: {error}"),
                    },
                },
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Request::Tool { name } => {
            if is_offensive_tool(&name) {
                return Response::Error {
                    message: format!(
                        "la herramienta `{name}` exige autorización humana: solicítala desde el chat"
                    ),
                };
            }
            match state.sandbox.exec_tool(&name, "{}").await {
                Ok(result) => match serde_json::to_value(result) {
                    Ok(value) => Response::Ok { data: value },
                    Err(error) => Response::Error {
                        message: format!("serialization error: {error}"),
                    },
                },
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Request::Index { path } => index_path(path, state).await,
        Request::DescribeImage { path, prompt } => describe_image(path, prompt, state).await,
        Request::Transcribe { path } => transcribe_audio(path, state).await,
        Request::Listen { seconds } => listen(seconds, state).await,
        Request::Speak { text } => speak(text, state).await,
        Request::Chat { query } => match run_agent(&query, state).await {
            Ok(AgentOutcome::Answer(answer)) => Response::Ok {
                data: json!({"answer": answer}),
            },
            Ok(AgentOutcome::AuthRequired {
                mode,
                request_id,
                form,
            }) => Response::RequiresAuth {
                mode,
                request_id,
                form,
            },
            Err(error) => Response::Error {
                message: error.to_string(),
            },
        },
        Request::Authorize { id, approved, form } => {
            handle_authorize(id, approved, form, state).await
        }
    }
}

async fn set_mode(secure: bool, state: &AppState) -> Response {
    state.secure.store(secure, Ordering::Relaxed);
    let value = if secure { "secure\n" } else { "simple\n" };
    if let Err(error) = tokio::fs::write(&state.mode_file, value).await {
        return Response::Error {
            message: format!("no se pudo persistir el modo: {error}"),
        };
    }
    let _ =
        tokio::fs::set_permissions(&state.mode_file, std::fs::Permissions::from_mode(0o600)).await;

    let _ = state
        .audit
        .log(json!({
            "event": "mode_changed",
            "secure": secure,
            "mode": if secure { "secure" } else { "simple" },
            "timestamp_ms": now_millis(),
        }))
        .await;

    Response::Ok {
        data: mode_payload(secure),
    }
}

async fn index_path(path: String, state: &AppState) -> Response {
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            return Response::Error {
                message: format!("Error leyendo archivo: {error}"),
            };
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
                };
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
        data: json!({"status": "indexed", "path": path, "chunks": indexed}),
    }
}

async fn describe_image(path: String, prompt: Option<String>, state: &AppState) -> Response {
    let Some(vision) = state.vision.as_ref() else {
        return Response::Error {
            message: "el motor de visión no está configurado (define HYORY_VISION_SOCKET)"
                .to_string(),
        };
    };

    let prompt = prompt.unwrap_or_else(|| {
        "Describe la imagen en detalle. Si contiene texto, transcríbelo. Responde en español."
            .to_string()
    });

    if let Ok(verdict) = state.guard.classify(&prompt).await {
        let _ = state
            .audit
            .log(json!({
                "event": "guard_query",
                "stage": "vision",
                "label": verdict.label_name(),
                "categories": verdict.categories,
                "secure_mode": state.secure_mode(),
                "preview": truncate(&prompt, 240),
                "timestamp_ms": now_millis(),
            }))
            .await;
    }

    match vision.describe(std::path::Path::new(&path), &prompt).await {
        Ok(answer) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "vision_request",
                    "path": path,
                    "prompt": prompt,
                    "answer_preview": truncate(&answer, 240),
                    "timestamp_ms": now_millis(),
                }))
                .await;

            match finalize(&format!("[imagen] {path}"), answer, state).await {
                Ok(answer) => Response::Ok {
                    data: json!({"answer": answer, "path": path}),
                },
                Err(error) => Response::Error {
                    message: error.to_string(),
                },
            }
        }
        Err(error) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "vision_error",
                    "path": path,
                    "error": error.to_string(),
                    "timestamp_ms": now_millis(),
                }))
                .await;
            Response::Error {
                message: error.to_string(),
            }
        }
    }
}

async fn transcribe_audio(path: String, state: &AppState) -> Response {
    let Some(voice) = state.voice.as_ref() else {
        return Response::Error {
            message: "el motor de voz no está configurado (Whisper/Piper)".to_string(),
        };
    };

    match voice.transcribe(std::path::Path::new(&path)).await {
        Ok(text) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "voice_transcribed",
                    "audio": path,
                    "preview": truncate(&text, 240),
                    "timestamp_ms": now_millis(),
                }))
                .await;
            Response::Ok {
                data: json!({"text": text, "audio": path}),
            }
        }
        Err(error) => Response::Error {
            message: error.to_string(),
        },
    }
}

async fn listen(seconds: Option<u32>, state: &AppState) -> Response {
    let Some(voice) = state.voice.as_ref() else {
        return Response::Error {
            message: "el motor de voz no está configurado (Whisper/Piper)".to_string(),
        };
    };

    let seconds = seconds.unwrap_or(5);
    match voice.listen(seconds).await {
        Ok(text) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "voice_listened",
                    "seconds": seconds,
                    "preview": truncate(&text, 240),
                    "timestamp_ms": now_millis(),
                }))
                .await;
            Response::Ok {
                data: json!({"text": text, "seconds": seconds}),
            }
        }
        Err(error) => Response::Error {
            message: error.to_string(),
        },
    }
}

async fn speak(text: String, state: &AppState) -> Response {
    let Some(voice) = state.voice.as_ref() else {
        return Response::Error {
            message: "el motor de voz no está configurado (Whisper/Piper)".to_string(),
        };
    };

    let text = text.trim().to_string();
    if text.is_empty() {
        return Response::Error {
            message: "no hay texto que pronunciar".to_string(),
        };
    }

    match voice.speak(&text).await {
        Ok(audio) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "voice_spoken",
                    "played": audio.is_none(),
                    "preview": truncate(&text, 240),
                    "timestamp_ms": now_millis(),
                }))
                .await;
            Response::Ok {
                data: json!({
                    "played": audio.is_none(),
                    "audio": audio.map(|path| path.display().to_string()),
                }),
            }
        }
        Err(error) => Response::Error {
            message: error.to_string(),
        },
    }
}

fn es_conversacion_casual(query: &str) -> bool {
    let q = query.to_lowercase();
    let tarea_keywords = [
        "batería",
        "bateria",
        "gpu",
        "disco",
        "memoria",
        "proceso",
        "red",
        "ping",
        "kernel",
        "hora",
        "fecha",
        "main",
        "fn ",
        "código",
        "codigo",
        "archivo",
        "rust",
        "función",
        "funcion",
        "imagen",
        "voz",
        "escuchar",
        "hablar",
        "index",
        "search",
        "busca",
        "ejecuta",
        "diagnóstico",
        "cpu",
        "ram",
        "temperatura",
        "sensor",
        "puerto",
        "conexión",
        "conexion",
    ];
    !tarea_keywords.iter().any(|kw| q.contains(kw)) && !q.starts_with('/') && !q.trim().is_empty()
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

enum AgentOutcome {
    Answer(String),
    AuthRequired {
        mode: &'static str,
        request_id: String,
        form: Value,
    },
}

async fn run_agent(query: &str, state: &AppState) -> Result<AgentOutcome, AgentError> {
    let q_lower = query.to_lowercase();

    // 0. GUARD: clasifica la petición del usuario (auditoría, nunca bloquea
    //    la petición: el control de acciones ofensivas es el formulario).
    match state.guard.classify(query).await {
        Ok(verdict) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "guard_query",
                    "label": verdict.label_name(),
                    "categories": verdict.categories,
                    "dangerous": verdict.is_dangerous(),
                    "secure_mode": state.secure_mode(),
                    "preview": truncate(query, 240),
                    "timestamp_ms": now_millis(),
                }))
                .await;
        }
        Err(error) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "guard_unavailable",
                    "error": error.to_string(),
                    "timestamp_ms": now_millis(),
                }))
                .await;
        }
    }

    // 1. CLASIFICADOR DE INTENCIÓN
    let es_charla_casual = es_conversacion_casual(query);

    if es_charla_casual {
        eprintln!("👋 Modo charla casual (cero herramientas)");
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: Some(
                    "Eres Hyrulor, un asistente de IA local. Responde de forma natural, amable y breve a cualquier conversación casual o pregunta general. No uses herramientas."
                        .to_string(),
                ),
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
        let engine = &state.inference;
        let response = engine.chat(&messages, &[], false).await?;
        let answer = response.content.unwrap_or_default().trim().to_string();
        return finalize(query, answer, state)
            .await
            .map(AgentOutcome::Answer);
    }

    let es_pregunta_codigo = q_lower.contains("main")
        || q_lower.contains("fn ")
        || q_lower.contains("código")
        || q_lower.contains("archivo")
        || q_lower.contains("rust")
        || q_lower.contains("función");

    // 2. FASE RAG (embedding + reranking real)
    if es_pregunta_codigo {
        let rag_docs = match state.embedding_client.get_embedding(query).await {
            Ok(embedding) => state
                .vault
                .search_documents(embedding, 8)
                .await
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        if !rag_docs.is_empty() {
            eprintln!("🔍 RAG activado: {} fragmentos crudos", rag_docs.len());
            let reranked = state
                .embedding_client
                .rerank(query, rag_docs)
                .await
                .unwrap_or_default();

            if !reranked.is_empty() {
                let top: Vec<String> = reranked.into_iter().take(3).collect();
                eprintln!("🎯 Reranker: {} fragmentos finales", top.len());
                let context = top.join("\n\n---\n\n");

                let messages = vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: Some(format!(
                            "Eres Hyrulor, experto en código. Responde la pregunta usando ÚNICAMENTE este código.\n\n== CÓDIGO ==\n{}\n== FIN ==",
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
                    return finalize(query, answer, state)
                        .await
                        .map(AgentOutcome::Answer);
                }
            }
        }
    }

    // 3. FASE HERRAMIENTAS / CHARLA GENERAL
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
            "Memoria:\n{}",
            serde_json::to_string(&memories).unwrap_or_default()
        )
    };

    let tools = tool_definitions();

    let mut messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: Some(format!(
                "You are Hyory, a local Linux assistant. Answer directly. If asked about hardware or system status, you MUST call the relevant tool. \
                 If the user asks for any offensive security action (scan, exploit, pentest, attack), you MUST call `offensive_security_action`; \
                 never run it yourself and never suggest illegal activity. The daemon will ask the human for explicit authorization. \
                 Memory: {memory_context}"
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
            return finalize(query, answer, state)
                .await
                .map(AgentOutcome::Answer);
        }

        let assistant_calls = response.tool_calls.clone();
        messages.push(response);

        for call in assistant_calls {
            // Toda acción ofensiva pasa por autorización humana explícita.
            if is_offensive_tool(&call.function.name) {
                let (mode, request_id, form) =
                    create_auth_request(state, &call.function.name, &call.function.arguments).await;
                return Ok(AgentOutcome::AuthRequired {
                    mode,
                    request_id,
                    form,
                });
            }

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

            messages.push(tool_message(call.id, output));
        }
    }

    Err(AgentError::EmptyAnswer)
}

/// Revisa la respuesta final con el Guard. Fuera del modo seguro, una
/// respuesta `Unsafe` se sustituye por un aviso; en modo seguro se permite
/// pero queda auditada.
async fn finalize(query: &str, answer: String, state: &AppState) -> Result<String, AgentError> {
    let es_casual = es_conversacion_casual(query);
    if answer.trim().is_empty() {
        return Err(AgentError::EmptyAnswer);
    }

    let guarded = match state.guard.classify(&answer).await {
        Ok(verdict) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "guard_response",
                    "label": verdict.label_name(),
                    "categories": verdict.categories,
                    "secure_mode": state.secure_mode(),
                    "preview": truncate(&answer, 240),
                    "timestamp_ms": now_millis(),
                }))
                .await;

            if verdict.is_unsafe() && !state.secure_mode() && !es_casual {
                let _ = state
                    .audit
                    .log(json!({
                        "event": "guard_response_blocked",
                        "label": verdict.label_name(),
                        "categories": verdict.categories,
                        "timestamp_ms": now_millis(),
                    }))
                    .await;
                "🛡️ El Guardián bloqueó esta respuesta por considerarla peligrosa. \
                 Si es una operación autorizada, activa el modo seguro con `/secure on`."
                    .to_string()
            } else {
                answer
            }
        }
        Err(error) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "guard_unavailable",
                    "stage": "response",
                    "error": error.to_string(),
                    "timestamp_ms": now_millis(),
                }))
                .await;
            answer
        }
    };

    let _ = state
        .vault
        .save_conversation(query.to_string(), guarded.clone())
        .await;
    Ok(guarded)
}

async fn create_auth_request(
    state: &AppState,
    tool: &str,
    arguments: &str,
) -> (&'static str, String, Value) {
    let seq = state.auth_counter.fetch_add(1, Ordering::Relaxed);
    let millis = now_millis();
    let request_id = format!("AUTH-{millis}-{seq}");
    let code = random_code();
    let secure = state.secure_mode();

    let pending = PendingAuth {
        tool: tool.to_string(),
        arguments: arguments.to_string(),
        code,
        secure,
        created_at: millis,
    };
    {
        let mut pending_requests = state.pending_auth.lock().await;
        // Evita crecimiento ilimitado: descarta solicitudes caducadas.
        pending_requests
            .retain(|_, request| millis.saturating_sub(request.created_at) <= PENDING_AUTH_TTL_MS);
        pending_requests.insert(request_id.clone(), pending.clone());
    }

    let mode = if secure { "secure" } else { "simple" };
    let _ = state
        .audit
        .log(json!({
            "event": "auth_requested",
            "request_id": request_id,
            "tool": tool,
            "mode": mode,
            "arguments": arguments,
            "timestamp_ms": millis,
        }))
        .await;

    let form = build_auth_form(&request_id, &pending);
    (mode, request_id, form)
}

fn build_auth_form(request_id: &str, pending: &PendingAuth) -> Value {
    let proposed = json!({
        "tool": pending.tool,
        "arguments": arguments_value(&pending.arguments),
    });

    if !pending.secure {
        return json!({
            "title": "¿Seguro que quieres hacer esto?",
            "question": "¿Seguro que quieres hacer esto? [y/n]",
            "warning": "No estás en modo seguro: solo se pedirá una confirmación simple. La acción se ejecuta en sandbox y queda auditada.",
            "request_id": request_id,
            "proposed_action": proposed,
        });
    }

    json!({
        "title": "FORMULARIO DE AUTORIZACIÓN DE ACCIÓN OFENSIVA",
        "notice": "Modo seguro activo. Rellena todos los campos. La acción se ejecutará aislada en bubblewrap (sin red, sin capabilities) y cada paso quedará auditado y cifrado con GPG AES-256.",
        "request_id": request_id,
        "proposed_action": proposed,
        "fields": [
            {"name": "target", "label": "Objetivo (IP / dominio / host)", "required": true},
            {"name": "scope", "label": "Alcance autorizado", "required": true},
            {"name": "justification", "label": "Justificación de la prueba", "required": true},
            {"name": "start_time", "label": "Inicio autorizado (ISO-8601)", "required": true},
            {"name": "end_time", "label": "Fin autorizado (ISO-8601)", "required": true},
            {"name": "authorization_code", "label": "Código de autorización (el mostrado arriba)", "required": true}
        ],
        "authorization_code": pending.code,
        "monitoring": {
            "audit": "GPG AES-256, un archivo cifrado por evento",
            "sandbox": "bubblewrap --unshare-user/uts/ipc/pid/net --cap-drop ALL --clearenv",
            "events": ["auth_requested", "auth_approved", "action_start", "action_result", "action_end"],
            "execution": "dry-run: no se ejecuta ningún ataque real"
        }
    })
}

async fn handle_authorize(
    id: String,
    approved: bool,
    form: Option<Value>,
    state: &AppState,
) -> Response {
    let pending = {
        let pending_requests = state.pending_auth.lock().await;
        pending_requests.get(&id).cloned()
    };
    let Some(pending) = pending else {
        return Response::Error {
            message: format!("solicitud de autorización desconocida o expirada: {id}"),
        };
    };

    if now_millis().saturating_sub(pending.created_at) > PENDING_AUTH_TTL_MS {
        state.pending_auth.lock().await.remove(&id);
        let _ = state
            .audit
            .log(json!({
                "event": "auth_expired",
                "request_id": id,
                "tool": pending.tool,
                "timestamp_ms": now_millis(),
            }))
            .await;
        return Response::Error {
            message: "la solicitud de autorización expiró (15 minutos)".to_string(),
        };
    }

    if !approved {
        state.pending_auth.lock().await.remove(&id);
        let _ = state
            .audit
            .log(json!({
                "event": "auth_denied",
                "request_id": id,
                "tool": pending.tool,
                "timestamp_ms": now_millis(),
            }))
            .await;
        return Response::Ok {
            data: json!({
                "answer": "🛑 Acción cancelada por el usuario. No se ejecutó nada.",
                "authorized": false,
            }),
        };
    }

    if pending.secure {
        if let Err(message) = validate_secure_form(&form, &pending) {
            let _ = state
                .audit
                .log(json!({
                    "event": "auth_invalid",
                    "request_id": id,
                    "reason": message,
                    "timestamp_ms": now_millis(),
                }))
                .await;
            return Response::Error { message };
        }
    }

    state.pending_auth.lock().await.remove(&id);

    let _ = state
        .audit
        .log(json!({
            "event": "auth_approved",
            "request_id": id,
            "tool": pending.tool,
            "secure": pending.secure,
            "form": form,
            "timestamp_ms": now_millis(),
        }))
        .await;

    let arguments = effective_arguments(&pending, &form);
    let _ = state
        .audit
        .log(json!({
            "event": "action_start",
            "request_id": id,
            "tool": pending.tool,
            "arguments": arguments,
            "timestamp_ms": now_millis(),
        }))
        .await;

    let result = state.sandbox.exec_tool(&pending.tool, &arguments).await;

    match result {
        Ok(result) => {
            let audit_file = state
                .audit
                .log(json!({
                    "event": "action_result",
                    "request_id": id,
                    "tool": pending.tool,
                    "output": truncate(&result.output, 2000),
                    "timestamp_ms": now_millis(),
                }))
                .await
                .ok();
            let _ = state
                .audit
                .log(json!({
                    "event": "action_end",
                    "request_id": id,
                    "tool": pending.tool,
                    "status": "ok",
                    "timestamp_ms": now_millis(),
                }))
                .await;

            let audit_name = audit_file
                .and_then(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .unwrap_or_else(|| "(sin archivo)".to_string());

            Response::Ok {
                data: json!({
                    "answer": format!(
                        "✅ Acción autorizada y ejecutada en sandbox.\n\
                         🔒 Evento de auditoría cifrado: {audit_name}\n\n{}",
                        result.output
                    ),
                    "authorized": true,
                    "request_id": id,
                }),
            }
        }
        Err(error) => {
            let _ = state
                .audit
                .log(json!({
                    "event": "action_end",
                    "request_id": id,
                    "tool": pending.tool,
                    "status": "error",
                    "error": error.to_string(),
                    "timestamp_ms": now_millis(),
                }))
                .await;
            Response::Error {
                message: format!("la acción autorizada falló en el sandbox: {error}"),
            }
        }
    }
}

fn validate_secure_form(form: &Option<Value>, pending: &PendingAuth) -> Result<(), String> {
    let object = form
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| "falta el formulario de autorización".to_string())?;

    for field in SECURE_FORM_FIELDS {
        let value = object
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        if value.is_empty() {
            return Err(format!("campo obligatorio vacío: {field}"));
        }
    }

    let code = object
        .get("authorization_code")
        .and_then(Value::as_str)
        .unwrap_or("");
    if code != pending.code {
        return Err("código de autorización incorrecto".to_string());
    }

    Ok(())
}

fn effective_arguments(pending: &PendingAuth, form: &Option<Value>) -> String {
    if !pending.secure {
        return pending.arguments.clone();
    }
    let Some(form) = form else {
        return pending.arguments.clone();
    };

    let mut value = serde_json::from_str::<Value>(&pending.arguments)
        .unwrap_or_else(|_| json!({"raw": pending.arguments}));
    if let Some(object) = value.as_object_mut() {
        object.insert("authorized_form".to_string(), form.clone());
    }
    serde_json::to_string(&value).unwrap_or_else(|_| pending.arguments.clone())
}

fn arguments_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({"raw": raw}))
}

fn mode_payload(secure: bool) -> Value {
    json!({
        "mode": if secure { "secure" } else { "simple" },
        "secure": secure,
    })
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn random_code() -> String {
    let mut buffer = [0u8; 4];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut buffer))
        .is_ok()
    {
        format!("{:06}", u32::from_le_bytes(buffer) % 1_000_000)
    } else {
        format!("{:06}", now_millis() % 1_000_000)
    }
}

fn truncate(text: &str, max: usize) -> String {
    let mut truncated: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        truncated.push('…');
    }
    truncated
}

fn is_offensive_tool(name: &str) -> bool {
    name == OFFENSIVE_TOOL
        || matches!(
            name,
            "nmap_scan" | "exploit" | "metasploit" | "sqlmap" | "hydra" | "john" | "hashcat"
        )
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
    use super::{
        build_auth_form, effective_arguments, is_allowed_tool, is_offensive_tool,
        validate_secure_form, validate_tool_arguments, PendingAuth, OFFENSIVE_TOOL,
    };
    use serde_json::json;

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
    fn offensive_tools_are_always_gated() {
        assert!(is_offensive_tool(OFFENSIVE_TOOL));
        assert!(is_offensive_tool("nmap_scan"));
        assert!(!is_offensive_tool("check_disk_space"));
    }

    #[test]
    fn secure_form_requires_every_field_and_the_right_code() {
        let pending = PendingAuth {
            tool: OFFENSIVE_TOOL.to_string(),
            arguments: "{}".to_string(),
            code: "123456".to_string(),
            secure: true,
            created_at: 0,
        };

        assert!(validate_secure_form(&None, &pending).is_err());

        let valid = json!({
            "target": "10.0.0.1",
            "scope": "laboratorio propio",
            "justification": "auditoría",
            "start_time": "2026-11-08T10:00:00Z",
            "end_time": "2026-11-08T11:00:00Z",
            "authorization_code": "123456",
        });
        assert!(validate_secure_form(&Some(valid), &pending).is_ok());

        let wrong_code = json!({
            "target": "10.0.0.1",
            "scope": "laboratorio propio",
            "justification": "auditoría",
            "start_time": "2026-11-08T10:00:00Z",
            "end_time": "2026-11-08T11:00:00Z",
            "authorization_code": "000000",
        });
        assert!(validate_secure_form(&Some(wrong_code), &pending).is_err());
    }

    #[test]
    fn simple_form_has_no_fields_and_asks_yes_no() {
        let pending = PendingAuth {
            tool: OFFENSIVE_TOOL.to_string(),
            arguments: "{}".to_string(),
            code: "123456".to_string(),
            secure: false,
            created_at: 0,
        };

        let form = build_auth_form("AUTH-1", &pending);
        assert_eq!(form["question"], "¿Seguro que quieres hacer esto? [y/n]");
        assert!(form.get("fields").is_none());
        assert_eq!(form["proposed_action"]["tool"], OFFENSIVE_TOOL);
    }

    #[test]
    fn secure_form_has_six_fields_and_the_generated_code() {
        let pending = PendingAuth {
            tool: OFFENSIVE_TOOL.to_string(),
            arguments: "{}".to_string(),
            code: "123456".to_string(),
            secure: true,
            created_at: 0,
        };

        let form = build_auth_form("AUTH-2", &pending);
        assert_eq!(form["fields"].as_array().map(Vec::len), Some(6));
        assert_eq!(form["authorization_code"], "123456");
    }

    #[test]
    fn authorized_form_is_merged_into_tool_arguments() {
        let pending = PendingAuth {
            tool: OFFENSIVE_TOOL.to_string(),
            arguments: "{\"action\":\"port_scan\",\"target\":\"127.0.0.1\"}".to_string(),
            code: "123456".to_string(),
            secure: true,
            created_at: 0,
        };
        let form = json!({"target": "10.0.0.1"});

        let merged: serde_json::Value =
            serde_json::from_str(&effective_arguments(&pending, &Some(form))).unwrap();
        assert_eq!(merged["action"], "port_scan");
        assert_eq!(merged["authorized_form"]["target"], "10.0.0.1");
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
