//! Gestión de la clave de cifrado de SQLCipher.
//!
//! La clave es un valor de 32 bytes codificado como 64 caracteres
//! hexadecimales. Se entrega a SQLCipher como clave cruda
//! (`PRAGMA key = "x'…'"`), lo que evita el KDF de passphrase y cualquier
//! problema de comillas.
//!
//! Orden de resolución:
//! 1. Variable de entorno `HYORY_DB_KEY`.
//! 2. Archivo indicado por `HYORY_DB_KEY_FILE` o `~/.hyrulor_run/db.key`.
//! 3. Si la base de datos no existe o está vacía, se genera una clave nueva
//!    con permisos `0600`. Si la base ya contiene datos y no hay clave, se
//!    devuelve un error: generar una clave nueva dejaría la base ilegible.

use std::{
    fs::{self, DirBuilder, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

use rusqlite::Connection;

pub const KEY_ENV: &str = "HYORY_DB_KEY";
pub const KEY_FILE_ENV: &str = "HYORY_DB_KEY_FILE";
const KEY_BYTES: usize = 32;
const KEY_HEX_LEN: usize = KEY_BYTES * 2;
const KEY_FILE_NAME: &str = "db.key";

#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    #[error("HYORY_DB_KEY debe contener exactamente 64 caracteres hexadecimales (32 bytes)")]
    InvalidEnvKey,
    #[error("el archivo de clave {0} no contiene 64 caracteres hexadecimales válidos")]
    InvalidKeyFile(PathBuf),
    #[error("el archivo de clave {0} ya existe con una clave distinta de HYORY_DB_KEY")]
    KeyMismatch(PathBuf),
    #[error(
        "la base de datos {db} ya contiene datos y no existe la clave {key}; \
         se niega a generar una clave nueva para no dejar la base ilegible"
    )]
    MissingKey { db: PathBuf, key: PathBuf },
    #[error("no se pudo leer /dev/urandom: {0}")]
    Entropy(std::io::Error),
    #[error("error de E/S gestionando la clave en {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

/// Ruta efectiva del archivo de clave.
pub fn key_path() -> PathBuf {
    if let Some(explicit) = std::env::var_os(KEY_FILE_ENV) {
        return PathBuf::from(explicit);
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".hyrulor_run").join(KEY_FILE_NAME)
}

/// Resuelve la clave para abrir `db_path`, generándola solo si la base es nueva.
pub fn resolve_for_database(db_path: &Path) -> Result<String, KeyError> {
    if let Some(value) = std::env::var_os(KEY_ENV) {
        let raw = value.to_string_lossy();
        return parse_key(&raw).ok_or(KeyError::InvalidEnvKey);
    }

    let path = key_path();
    match fs::read_to_string(&path) {
        Ok(raw) => parse_key(&raw).ok_or(KeyError::InvalidKeyFile(path)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if database_has_content(db_path) {
                return Err(KeyError::MissingKey {
                    db: db_path.to_path_buf(),
                    key: path,
                });
            }
            create_key(&path)
        }
        Err(source) => Err(KeyError::Io { path, source }),
    }
}

/// Valida y normaliza una clave de 64 caracteres hexadecimales.
pub fn parse_key(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() == KEY_HEX_LEN && trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(trimmed.to_ascii_lowercase())
    } else {
        None
    }
}

/// Persiste `key` en `path` con permisos `0600` si el archivo aún no existe.
///
/// Si ya existe, exige que contenga exactamente la misma clave. Se usa al
/// migrar con `HYORY_DB_KEY` para que el daemon pueda abrir la base después
/// sin depender de esa variable de entorno.
pub fn ensure_key_file(path: &Path, key: &str) -> Result<(), KeyError> {
    let key = parse_key(key).ok_or(KeyError::InvalidEnvKey)?;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(parent)
                .map_err(|source| KeyError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        Ok(mut file) => file
            .write_all(key.as_bytes())
            .map_err(|source| KeyError::Io {
                path: path.to_path_buf(),
                source,
            }),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let raw = fs::read_to_string(path).map_err(|source| KeyError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            match parse_key(&raw) {
                Some(existing) if existing == key => Ok(()),
                Some(_) => Err(KeyError::KeyMismatch(path.to_path_buf())),
                None => Err(KeyError::InvalidKeyFile(path.to_path_buf())),
            }
        }
        Err(source) => Err(KeyError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Aplica la clave a una conexión y comprueba que la base se puede leer.
///
/// Debe llamarse antes de cualquier otro `PRAGMA` que toque el archivo.
pub fn apply(connection: &Connection, key: &str) -> Result<(), rusqlite::Error> {
    debug_assert!(parse_key(key).is_some(), "clave no validada antes de apply");
    connection.execute_batch(&format!("PRAGMA key = \"x'{key}'\";"))?;
    // Con una clave incorrecta SQLCipher devuelve "file is not a database" en
    // la primera lectura; así el fallo es inmediato y visible.
    connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))?;
    Ok(())
}

fn database_has_content(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path != Path::new(":memory:")
        && fs::metadata(path)
            .map(|metadata| metadata.len() > 0)
            .unwrap_or(false)
}

fn create_key(path: &Path) -> Result<String, KeyError> {
    let mut bytes = [0u8; KEY_BYTES];
    fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(KeyError::Entropy)?;
    let key: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(parent)
                .map_err(|source| KeyError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            // Si el directorio ya existía, asegura también 0700.
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        Ok(mut file) => {
            file.write_all(key.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|source| KeyError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
            Ok(key)
        }
        // Otro proceso creó la clave primero: usa la suya.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let raw = fs::read_to_string(path).map_err(|source| KeyError::Io {
                path: path.to_path_buf(),
                source,
            })?;
            parse_key(&raw).ok_or_else(|| KeyError::InvalidKeyFile(path.to_path_buf()))
        }
        Err(source) => Err(KeyError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{create_key, database_has_content, parse_key};
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("reloj")
            .as_nanos();
        std::env::temp_dir().join(format!("hyrulor-key-{name}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn accepts_only_32_byte_hex_keys() {
        let key = "a".repeat(64);
        assert_eq!(parse_key(&format!("  {key}\n")), Some(key.clone()));
        assert_eq!(parse_key(&key.to_uppercase()), Some(key));
        assert_eq!(parse_key("a".repeat(63).as_str()), None);
        assert_eq!(parse_key(&"g".repeat(64)), None);
        assert_eq!(parse_key(""), None);
    }

    #[test]
    fn generated_key_is_private_and_valid() {
        let path = temp_path("create");
        let key = create_key(&path).expect("clave creada");

        assert_eq!(key.len(), 64);
        assert!(parse_key(&key).is_some());
        let mode = fs::metadata(&path).expect("metadata").permissions().mode();
        assert_eq!(mode & 0o777, 0o600);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn detects_existing_database_and_ignores_memory() {
        let path = temp_path("db");
        assert!(!database_has_content(&path));
        fs::write(&path, b"data").expect("db temporal");
        assert!(database_has_content(&path));
        assert!(!database_has_content(std::path::Path::new(":memory:")));
        let _ = fs::remove_file(&path);
    }
}
