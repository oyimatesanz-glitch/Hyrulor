use std::{
    env,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use tokio::{io::AsyncWriteExt, process::Command, time::timeout};

const RECORD_SECONDS_MIN: u32 = 1;
const RECORD_SECONDS_MAX: u32 = 60;

#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("voice io: {0}")]
    Io(#[from] std::io::Error),
    #[error("voice command timed out")]
    Timeout,
    #[error("voice command failed ({command}): {stderr}")]
    Failed { command: String, stderr: String },
    #[error("whisper produced no text")]
    Empty,
}

/// Motor de voz local: `arecord` -> `whisper.cpp` (ASR) y `piper` -> reproductor (TTS).
pub struct VoiceEngine {
    whisper_bin: PathBuf,
    whisper_model: PathBuf,
    piper_bin: PathBuf,
    piper_voice: PathBuf,
    player: Option<PathBuf>,
}

impl VoiceEngine {
    pub fn from_env() -> Option<Self> {
        let whisper_bin = PathBuf::from(env::var_os("HYORY_WHISPER_BIN")?);
        let whisper_model = PathBuf::from(env::var_os("HYORY_WHISPER_MODEL")?);
        let piper_bin = PathBuf::from(env::var_os("HYORY_PIPER_BIN")?);
        let piper_voice = PathBuf::from(env::var_os("HYORY_PIPER_VOICE")?);

        if !whisper_bin.is_file()
            || !whisper_model.is_file()
            || !piper_bin.is_file()
            || !piper_voice.is_file()
        {
            return None;
        }

        let player = env::var_os("HYORY_AUDIO_PLAYER")
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .or_else(|| find_in_path("pw-play"))
            .or_else(|| find_in_path("paplay"))
            .or_else(|| find_in_path("aplay"));

        Some(Self {
            whisper_bin,
            whisper_model,
            piper_bin,
            piper_voice,
            player,
        })
    }

    pub async fn transcribe(&self, audio: &Path) -> Result<String, VoiceError> {
        if !audio.is_file() {
            return Err(VoiceError::Failed {
                command: "whisper".to_string(),
                stderr: format!("no existe el archivo de audio: {}", audio.display()),
            });
        }

        let mut command = Command::new(&self.whisper_bin);
        command
            .arg("-m")
            .arg(&self.whisper_model)
            .arg("-f")
            .arg(audio)
            .arg("-l")
            .arg("es")
            .arg("-nt")
            .arg("-t")
            .arg("4")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = timeout(Duration::from_secs(180), command.output())
            .await
            .map_err(|_| VoiceError::Timeout)??;

        if !output.status.success() {
            return Err(VoiceError::Failed {
                command: "whisper".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() {
            return Err(VoiceError::Empty);
        }
        Ok(text)
    }

    pub async fn record(&self, seconds: u32) -> Result<PathBuf, VoiceError> {
        let seconds = seconds.clamp(RECORD_SECONDS_MIN, RECORD_SECONDS_MAX);
        let path = temp_wav("listen");

        let mut command = Command::new("arecord");
        command
            .args([
                "-q", "-t", "wav", "-f", "S16_LE", "-r", "16000", "-c", "1", "-d",
            ])
            .arg(seconds.to_string())
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        let output = timeout(
            Duration::from_secs(u64::from(seconds) + 15),
            command.output(),
        )
        .await
        .map_err(|_| VoiceError::Timeout)??;

        if !output.status.success() {
            let _ = tokio::fs::remove_file(&path).await;
            return Err(VoiceError::Failed {
                command: "arecord".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(path)
    }

    pub async fn listen(&self, seconds: u32) -> Result<String, VoiceError> {
        let path = self.record(seconds).await?;
        let result = self.transcribe(&path).await;
        let _ = tokio::fs::remove_file(&path).await;
        result
    }

    /// Genera voz con Piper y la reproduce si hay reproductor.
    /// Devuelve `Some(ruta)` solo cuando no se pudo reproducir y el WAV queda disponible.
    pub async fn speak(&self, text: &str) -> Result<Option<PathBuf>, VoiceError> {
        let output_path = temp_wav("speak");

        let mut command = Command::new(&self.piper_bin);
        command
            .arg("--model")
            .arg(&self.piper_voice)
            .arg("--output_file")
            .arg(&output_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        if let Some(piper_dir) = self.piper_bin.parent() {
            command.current_dir(piper_dir);
            let existing = env::var("LD_LIBRARY_PATH").unwrap_or_default();
            let joined = if existing.is_empty() {
                piper_dir.display().to_string()
            } else {
                format!("{}:{}", piper_dir.display(), existing)
            };
            command.env("LD_LIBRARY_PATH", joined);
            // Algunas distribuciones del binario buscan espeak-ng-data relativo al cwd.
            command.env("ESPEAK_DATA_PATH", piper_dir.join("espeak-ng-data"));
        }

        let mut child = command.spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
        }
        let result = timeout(Duration::from_secs(90), child.wait_with_output())
            .await
            .map_err(|_| VoiceError::Timeout)??;

        if !result.status.success() {
            let _ = tokio::fs::remove_file(&output_path).await;
            return Err(VoiceError::Failed {
                command: "piper".to_string(),
                stderr: String::from_utf8_lossy(&result.stderr).trim().to_string(),
            });
        }

        if let Some(player) = &self.player {
            let mut play = Command::new(player);
            play.arg(&output_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped());
            let played = timeout(Duration::from_secs(60), play.output())
                .await
                .map_err(|_| VoiceError::Timeout)??;
            if !played.status.success() {
                let _ = tokio::fs::remove_file(&output_path).await;
                return Err(VoiceError::Failed {
                    command: player.display().to_string(),
                    stderr: String::from_utf8_lossy(&played.stderr).trim().to_string(),
                });
            }
            let _ = tokio::fs::remove_file(&output_path).await;
            Ok(None)
        } else {
            Ok(Some(output_path))
        }
    }
}

fn temp_wav(prefix: &str) -> PathBuf {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    env::temp_dir().join(format!("hyrulor-{prefix}-{millis}.wav"))
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}
