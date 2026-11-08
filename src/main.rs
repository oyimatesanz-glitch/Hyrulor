mod audit;
mod db_key;
mod embedding_client;
mod guard_client;
mod inference_engine;
mod memory_vault;
mod protocol;
mod tool_sandbox;
mod vision_client;
mod voice;

use std::{
    env,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};

use inference_engine::InferenceEngine;
use memory_vault::MemoryVault;
use protocol::{handle_connection, AppState};
use tokio::{net::UnixListener, signal};
use tracing::{error, info};

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("memory vault: {0}")]
    Memory(#[from] memory_vault::MemoryError),
    #[error("tool sandbox: {0}")]
    Sandbox(#[from] tool_sandbox::ToolError),
    #[error("audit log: {0}")]
    Audit(#[from] audit::AuditError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("refusing to replace a non-socket path: {0}")]
    UnsafeSocketPath(PathBuf),
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Registrar sqlite-vec globalmente (una sola vez al inicio)
    unsafe {
        rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
            sqlite_vec::sqlite3_vec_init as *const (),
        )));
    }

    tracing_subscriber::fmt()
        .with_env_filter(env::var("RUST_LOG").unwrap_or_else(|_| "hyory=info".to_string()))
        .with_target(false)
        .compact()
        .init();

    let socket_path = env::var_os("HYORY_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/omni.sock"));
    let database_path = env::var_os("HYORY_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("hyory.db"));
    let tools_dir = env::var_os("HYORY_TOOLS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tools"));
    let llm_addr = env::var("HYORY_LLM_ADDR")
        .or_else(|_| env::var("HYORY_LLM_SOCKET"))
        .unwrap_or_else(|_| "127.0.0.1:8082".to_string());
    let llm_model =
        env::var("HYORY_LLM_MODEL").unwrap_or_else(|_| "Qwen3.5-4B-Instruct-Q4_K_M".to_string());

    match tokio::fs::symlink_metadata(&socket_path).await {
        Ok(metadata) if metadata.file_type().is_socket() => {
            tokio::fs::remove_file(&socket_path).await?;
        }
        Ok(_) => return Err(AppError::UnsafeSocketPath(socket_path)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(AppError::Io(error)),
    }

    let vault = MemoryVault::open(database_path).await?;
    let sandbox = tool_sandbox::ToolSandbox::new(tools_dir)?;
    let inference = InferenceEngine::new(llm_addr, llm_model);
    let coder = env::var_os("HYORY_CODER_SOCKET")
        .and_then(|s| s.into_string().ok())
        .map(|addr| InferenceEngine::new(addr, "Qwen2.5-Coder-7B".to_string()));

    let embedding_addr = env::var("HYORY_EMBEDDING_ADDR")
        .or_else(|_| env::var("HYORY_EMBEDDING_SOCKET"))
        .unwrap_or_else(|_| "127.0.0.1:8081".to_string());
    let reranker_socket = env::var_os("HYORY_RERANKER_SOCKET").map(PathBuf::from);
    let embedding_client = embedding_client::EmbeddingClient::new(embedding_addr.into())
        .with_reranker(reranker_socket);

    let guard_socket =
        env::var("HYORY_GUARD_SOCKET").unwrap_or_else(|_| default_run_socket("guard.sock"));
    let guard = guard_client::GuardClient::new(InferenceEngine::new(
        guard_socket,
        "Qwen3Guard".to_string(),
    ));

    let audit_dir = env::var_os("HYORY_AUDIT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("audit"));
    let audit = audit::AuditLog::open(audit_dir)?;

    let mode_file = env::var_os("HYORY_MODE_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("hyory.mode"));
    let secure_mode = read_mode(&mode_file).unwrap_or_else(|| env_flag("HYORY_SECURE_MODE"));

    let vision = env::var_os("HYORY_VISION_SOCKET")
        .map(PathBuf::from)
        .filter(|socket| socket.exists())
        .map(|socket| {
            let model = env::var("HYORY_VISION_MODEL")
                .unwrap_or_else(|_| "Qwen2.5-VL-3B-Instruct".to_string());
            vision_client::VisionClient::new(socket, model)
        });
    let voice = voice::VoiceEngine::from_env();
    info!(
        vision = vision.is_some(),
        voice = voice.is_some(),
        "special senses"
    );

    let state = Arc::new(AppState::new(
        vault,
        sandbox,
        inference,
        coder,
        embedding_client,
        guard,
        audit,
        vision,
        voice,
        secure_mode,
        mode_file,
    ));
    let listener = UnixListener::bind(&socket_path)?;
    tokio::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600)).await?;

    info!(
        socket = %socket_path.display(),
        secure_mode,
        "Hyory daemon listening"
    );

    let accept_loop = async {
        loop {
            let (stream, _) = listener.accept().await?;
            let state = Arc::clone(&state);
            tokio::spawn(async move {
                if let Err(err) = handle_connection(stream, state).await {
                    error!(%err, "connection failed");
                }
            });
        }
        #[allow(unreachable_code)]
        Ok::<(), std::io::Error>(())
    };

    tokio::select! {
        result = accept_loop => result?,
        result = signal::ctrl_c() => result?,
    }

    if let Err(err) = tokio::fs::remove_file(&socket_path).await {
        error!(%err, "failed to remove daemon socket");
    }
    Ok(())
}

fn read_mode(path: &Path) -> Option<bool> {
    std::fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().eq_ignore_ascii_case("secure"))
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on" | "secure"
    )
}

fn default_run_socket(name: &str) -> String {
    match env::var_os("HOME") {
        Some(home) => PathBuf::from(home)
            .join(".hyrulor_run")
            .join(name)
            .to_string_lossy()
            .into_owned(),
        None => format!("/tmp/{name}"),
    }
}
