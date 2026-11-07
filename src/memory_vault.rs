use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use rusqlite::{params, Connection};
use serde::Serialize;
use tokio::task::JoinError;

pub type Db = Arc<Mutex<Connection>>;

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("database mutex is poisoned")]
    Poisoned,
    #[error("blocking task failed: {0}")]
    Join(#[from] JoinError),
    #[error("invalid database path: {0}")]
    Path(PathBuf),
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryHit {
    pub id: i64,
    pub timestamp: String,
    pub user: String,
    pub assistant: String,
}

#[derive(Clone)]
pub struct MemoryVault {
    db: Db,
}

impl MemoryVault {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let path = path.as_ref().to_path_buf();
        if path.as_os_str().is_empty() {
            return Err(MemoryError::Path(path));
        }

        let db = tokio::task::spawn_blocking(move || -> Result<Db, MemoryError> {
            let connection = Connection::open(path)?;
            configure_connection(&connection)?;
            initialize_schema(&connection)?;
            Ok(Arc::new(Mutex::new(connection)))
        })
        .await??;

        Ok(Self { db })
    }

    pub async fn save_conversation(
        &self,
        user: String,
        assistant: String,
    ) -> Result<i64, MemoryError> {
        let db = Arc::clone(&self.db);
        Ok(tokio::task::spawn_blocking(move || {
            let connection = lock_db(&db)?;
            connection.execute(
                "INSERT INTO convs (user, assistant) VALUES (?1, ?2)",
                params![user, assistant],
            )?;
            Ok::<i64, MemoryError>(connection.last_insert_rowid())
        })
        .await??)
    }

    pub async fn search(&self, query: String, limit: u32) -> Result<Vec<MemoryHit>, MemoryError> {
        let db = Arc::clone(&self.db);
        Ok(tokio::task::spawn_blocking(move || {
            let match_query = build_fts_query(&query);
            if match_query.is_empty() {
                return Ok(Vec::new());
            }

            let connection = lock_db(&db)?;
            let mut statement = connection.prepare(
                "SELECT c.id, c.ts, c.user, c.assistant
                 FROM conv_fts AS f
                 JOIN convs AS c ON c.id = f.rowid
                 WHERE conv_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows =
                statement.query_map(params![match_query, i64::from(limit.min(50))], |row| {
                    Ok(MemoryHit {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        user: row.get(2)?,
                        assistant: row.get(3)?,
                    })
                })?;

            rows.collect::<Result<Vec<_>, _>>()
                .map_err(MemoryError::from)
        })
        .await??)
    }
}

fn lock_db(db: &Db) -> Result<MutexGuard<'_, Connection>, MemoryError> {
    db.lock().map_err(|_| MemoryError::Poisoned)
}

fn configure_connection(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA foreign_keys = ON;",
    )
}

fn initialize_schema(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS convs (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             ts TEXT NOT NULL DEFAULT (datetime('now')),
             user TEXT NOT NULL,
             assistant TEXT NOT NULL
         );

         CREATE VIRTUAL TABLE IF NOT EXISTS conv_fts USING fts5(
             user, assistant, content='convs', content_rowid='id',
             tokenize='unicode61 remove_diacritics 2'
         );

         CREATE TRIGGER IF NOT EXISTS convs_ai AFTER INSERT ON convs BEGIN
             INSERT INTO conv_fts(rowid, user, assistant)
             VALUES (new.id, new.user, new.assistant);
         END;

         CREATE TRIGGER IF NOT EXISTS convs_ad AFTER DELETE ON convs BEGIN
             INSERT INTO conv_fts(conv_fts, rowid, user, assistant)
             VALUES ('delete', old.id, old.user, old.assistant);
         END;

         CREATE TRIGGER IF NOT EXISTS convs_au AFTER UPDATE ON convs BEGIN
             INSERT INTO conv_fts(conv_fts, rowid, user, assistant)
             VALUES ('delete', old.id, old.user, old.assistant);
             INSERT INTO conv_fts(rowid, user, assistant)
             VALUES (new.id, new.user, new.assistant);
         END;",
    )
}

fn build_fts_query(input: &str) -> String {
    input
        .split_whitespace()
        .map(|term| {
            term.chars()
                .filter(|character| character.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|term| term.chars().count() >= 3)
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

#[cfg(test)]
mod tests {
    use super::build_fts_query;

    #[test]
    fn fts_query_discards_short_and_punctuation_only_terms() {
        assert_eq!(
            build_fts_query("¿GPU? ok batería"),
            "\"GPU\" OR \"batería\""
        );
    }
}
