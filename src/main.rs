pub mod foundation;
mod inference_engine;
mod memory_vault;
mod protocol;
mod tool_sandbox;

use std::{env, path::PathBuf, sync::Arc};

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
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
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
    let llm_socket = env::var_os("HYORY_LLM_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/llm.sock"));
    let llm_addr = env::var("HYORY_LLM_ADDR").ok();
    let llm_model =
        env::var("HYORY_LLM_MODEL").unwrap_or_else(|_| "Qwen2.5-3B-Instruct-Q5_K_M".to_string());

    if socket_path.exists() {
        tokio::fs::remove_file(&socket_path).await?;
    }

    let vault = MemoryVault::open(database_path).await?;
    let sandbox = tool_sandbox::ToolSandbox::new(tools_dir)?;
    let inference = match llm_addr {
        Some(address) => InferenceEngine::new(std::path::PathBuf::from(address), llm_model),
        None => InferenceEngine::new(llm_socket, llm_model),
    };
    let state = Arc::new(AppState {
        vault,
        sandbox,
        inference,
    });
    let listener = UnixListener::bind(&socket_path)?;

    info!(socket = %socket_path.display(), "Hyory daemon listening");

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
