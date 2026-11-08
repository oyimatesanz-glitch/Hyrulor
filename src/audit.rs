use std::{
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("audit io: {0}")]
    Io(#[from] std::io::Error),
    #[error("audit json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("gpg failed ({status}): {stderr}")]
    Gpg { status: String, stderr: String },
    #[error("audit worker failed: {0}")]
    Worker(String),
}

/// Registro de auditoría cifrado con GPG (AES-256).
///
/// Cada evento se guarda como un mensaje OpenPGP independiente en
/// `audit/events/<timestamp>-<seq>.json.gpg`. Así el descifrado es
/// secuencial y una escritura corrupta nunca invalida el resto del log.
pub struct AuditLog {
    key_path: PathBuf,
    events_dir: PathBuf,
    counter: AtomicU64,
}

impl AuditLog {
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, AuditError> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;

        let events_dir = dir.join("events");
        std::fs::create_dir_all(&events_dir)?;
        std::fs::set_permissions(&events_dir, std::fs::Permissions::from_mode(0o700))?;

        let key_path = dir.join("audit.key");
        if !key_path.exists() {
            let mut key = [0u8; 32];
            std::fs::File::open("/dev/urandom")?.read_exact(&mut key)?;
            let hex = hex_encode(&key);
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&key_path)?;
            file.write_all(hex.as_bytes())?;
            file.write_all(b"\n")?;
        }
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;

        Ok(Self {
            key_path,
            events_dir,
            counter: AtomicU64::new(0),
        })
    }

    /// Cifra y persiste un evento. Devuelve la ruta del archivo cifrado.
    pub async fn log(&self, event: serde_json::Value) -> Result<PathBuf, AuditError> {
        let key_path = self.key_path.clone();
        let events_dir = self.events_dir.clone();
        let seq = self.counter.fetch_add(1, Ordering::Relaxed);
        let mut line = serde_json::to_vec(&event)?;
        line.push(b'\n');

        tokio::task::spawn_blocking(move || encrypt_event(&key_path, &events_dir, &line, seq))
            .await
            .map_err(|error| AuditError::Worker(error.to_string()))?
    }

    #[allow(dead_code)]
    pub fn key_path(&self) -> &Path {
        &self.key_path
    }
}

fn encrypt_event(
    key_path: &Path,
    events_dir: &Path,
    line: &[u8],
    seq: u64,
) -> Result<PathBuf, AuditError> {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let output = events_dir.join(format!("{millis:013}-{seq:06}.json.gpg"));

    let mut child = Command::new("gpg")
        .args(["--batch", "--yes", "--quiet", "--passphrase-file"])
        .arg(key_path)
        .args(["--symmetric", "--cipher-algo", "AES256", "--output"])
        .arg(&output)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(line)?;
    }

    let result = child.wait_with_output()?;
    if !result.status.success() {
        let _ = std::fs::remove_file(&output);
        return Err(AuditError::Gpg {
            status: result.status.to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).trim().to_string(),
        });
    }

    std::fs::set_permissions(&output, std::fs::Permissions::from_mode(0o600))?;
    Ok(output)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::hex_encode;

    #[test]
    fn encodes_bytes_as_lowercase_hex() {
        assert_eq!(hex_encode(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
    }
}
