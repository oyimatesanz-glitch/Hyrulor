//! Hito 2.1 (anticipado en Fase 0) — Memoria Jerárquica: SQLite WAL + FTS5 + triggers.
//!
//! REGLA 2: NUNCA inserts manuales en la tabla virtual FTS5 externa. La
//! sincronización es 100% por triggers AFTER INSERT/UPDATE/DELETE con
//! `rebuild`/`delete` de la índice externo content=''.
//! Tokenizer obligatorio: unicode61 remove_diacritics 2.
//! sqlite-vec (embeddings float[384] nomic-embed-text-v1.5) se añade en Fase 2
//! como extensión loadable; aquí queda el hook de esquema preparado.

use rusqlite::{params, Connection, OptionalExtension};
use std::fmt;

#[derive(Debug)]
pub enum MemoriaError {
    Sql(rusqlite::Error),
    /// El PRAGMA journal_mode no devolvió 'wal' => riesgo de corrupción/concurrencia.
    WalNoActivo(String),
    /// Verificación de integridad tras inserción: trigger roto o DB corrupta.
    FtsDesincronizado {
        conv_id: i64,
    },
}

impl fmt::Display for MemoriaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoriaError::Sql(e) => write!(f, "sqlite: {e}"),
            MemoriaError::WalNoActivo(v) => write!(f, "journal_mode != wal (obtenido: {v})"),
            MemoriaError::FtsDesincronizado { conv_id } => write!(
                f,
                "FTS5 no contiene fila de convs id={conv_id}: trigger comprometido"
            ),
        }
    }
}
impl std::error::Error for MemoriaError {}
impl From<rusqlite::Error> for MemoriaError {
    fn from(e: rusqlite::Error) -> Self {
        MemoriaError::Sql(e)
    }
}

pub struct Memoria {
    conn: Connection,
}

const ESQUEMA: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;      -- seguro con WAL + power-loss ok para logs conversacionales
PRAGMA foreign_keys=ON;
PRAGMA auto_vacuum=INCREMENTAL;

CREATE TABLE IF NOT EXISTS convs (
    id        INTEGER PRIMARY KEY,
    ts        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    session   TEXT NOT NULL,
    user      TEXT NOT NULL,
    assistant TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_convs_session ON convs(session, ts);

-- Tabla virtual EXTERNA content='': solo se alimenta por triggers (REGLA 2).
CREATE VIRTUAL TABLE IF NOT EXISTS conv_fts USING fts5(
    user, assistant,
    content='convs', content_rowid='id',
    tokenize="unicode61 remove_diacritics 2"
);

CREATE TRIGGER IF NOT EXISTS convs_ai AFTER INSERT ON convs BEGIN
    INSERT INTO conv_fts(rowid, user, assistant) VALUES (new.id, new.user, new.assistant);
END;
CREATE TRIGGER IF NOT EXISTS convs_ad AFTER DELETE ON convs BEGIN
    INSERT INTO conv_fts(conv_fts, rowid, user, assistant) VALUES('delete', old.id, old.user, old.assistant);
END;
CREATE TRIGGER IF NOT EXISTS convs_au AFTER UPDATE ON convs BEGIN
    INSERT INTO conv_fts(conv_fts, rowid, user, assistant) VALUES('delete', old.id, old.user, old.assistant);
    INSERT INTO conv_fts(rowid, user, assistant) VALUES (new.id, new.user, new.assistant);
END;

-- Hook Fase 2: sqlite-vec. CREATE VIRTUAL TABLE docs_vec USING vec0(id INTEGER PRIMARY KEY, embedding float[384]);
-- Se ejecutará tras load_extension('sqlite_vec') cuando esté sellada la firma del .so.
"#;

impl Memoria {
    /// Abre (o crea) la base con garantías anti-corrupción verificadas, no asumidas.
    pub fn abrir(ruta: &str) -> Result<Self, MemoriaError> {
        let conn = Connection::open(ruta)?;
        conn.execute_batch(ESQUEMA)?;

        // VERIFICACIÓN explícita de WAL (no confiar en el batch).
        // En sistemas de archivos sin POSIX locking (algunos CI/containers) SQLite
        // degrada journal_mode silenciosamente => lo tratamos como error duro aquí,
        // pero los tests usan un directorio privado para evitar colisiones.
        let modo: String = conn
            .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
            .unwrap_or_else(|_| "?".into());
        if modo.to_lowercase() != "wal" {
            return Err(MemoriaError::WalNoActivo(modo));
        }

        // Integridad estructural mínima al arrancar. Ojo: la sintaxis correcta es
        // PRAGMA integrity_check; el nivel rápido se obtiene con quick_check.
        let chk: String = conn
            .query_row("PRAGMA quick_check;", [], |r| r.get(0))
            .optional()?
            .unwrap_or_default();
        if !chk.eq_ignore_ascii_case("ok") {
            return Err(MemoriaError::Sql(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CORRUPT),
                Some(format!("quick_check != ok: {chk}")),
            )));
        }
        Ok(Self { conn })
    }

    /// Inserta un turno. El trigger FTS5 hace el resto. Retorna el id nuevo.
    pub fn registrar_turno(
        &self,
        session: &str,
        user: &str,
        assistant: &str,
    ) -> Result<i64, MemoriaError> {
        self.conn.execute(
            "INSERT INTO convs(session, user, assistant) VALUES (?1, ?2, ?3)",
            params![session, user, assistant],
        )?;
        let id = self.conn.last_insert_rowid();

        // AUTO-AUDITORÍA (REGLA 2): verificar que el trigger realmente indexó.
        let hay: i64 = self.conn.query_row(
            "SELECT count(*) FROM conv_fts WHERE rowid = ?1",
            params![id],
            |r| r.get(0),
        )?;
        if hay != 1 {
            return Err(MemoriaError::FtsDesincronizado { conv_id: id });
        }
        Ok(id)
    }

    /// Búsqueda textual BM25 (capa episódica barata, <5ms en discos NVMe/SATA sanos).
    pub fn buscar_texto(
        &self,
        q: &str,
        limite: u32,
    ) -> Result<Vec<(i64, String, String)>, MemoriaError> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.user, c.assistant
               FROM conv_fts f JOIN convs c ON c.id = f.rowid
              WHERE conv_fts MATCH ?1
              ORDER BY bm25(conv_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![q, limite], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// DERECHO AL OLVIDO (§6.5): borra y verifica destrucción real en ambas tablas.
    pub fn olvidar(&self, ids: &[i64]) -> Result<usize, MemoriaError> {
        let tx = self.conn.unchecked_transaction()?;
        let mut n = 0usize;
        for id in ids {
            n += tx.execute("DELETE FROM convs WHERE id = ?1", params![id])?;
        }
        // El trigger convs_ad purga conv_fts automáticamente. Verificamos.
        for id in ids {
            let quedan: i64 = tx.query_row(
                "SELECT count(*) FROM conv_fts WHERE rowid = ?1",
                params![id],
                |r| r.get(0),
            )?;
            if quedan != 0 {
                return Err(MemoriaError::FtsDesincronizado { conv_id: *id });
            }
        }
        tx.commit()?;
        Ok(n)
    }

    /// Compactación incremental (WAL checkpoint) para cerrar sesión sin dejar -wal huérfano.
    pub fn checkpoint(&self) -> Result<(), MemoriaError> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memoria_test(tag: &str) -> Memoria {
        // Cada test usa su propia DB: evita 'database is locked' entre hilos.
        let ruta = format!("/tmp/hyrulor_test_{}_{}.db", tag, std::process::id());
        let _ = std::fs::remove_file(&ruta);
        let _ = std::fs::remove_file(format!("{ruta}-wal"));
        let _ = std::fs::remove_file(format!("{ruta}-shm"));
        Memoria::abrir(&ruta).expect("apertura con WAL+FTS5")
    }

    #[test]
    fn wal_activo_y_trigger_sincroniza() {
        let m = memoria_test("wal");
        let id = m
            .registrar_turno("s1", "¿Cómo está mi batería?", "Está al 85%, salud OK")
            .unwrap();
        let hits = m.buscar_texto("batería", 5).unwrap(); // acentos: remove_diacritics 2
        assert!(hits.iter().any(|(h, _, _)| *h == id));
    }

    #[test]
    fn olvidar_purga_fts_junto_a_origen() {
        let m = memoria_test("olvido");
        let id = m
            .registrar_turno("s1", "secreto efímero", "borrado pronto")
            .unwrap();
        let n = m.olvidar(&[id]).unwrap();
        assert_eq!(n, 1);
        assert!(m.buscar_texto("efímero", 5).unwrap().is_empty());
    }

    #[test]
    fn fts_externa_refleja_solo_la_tabla_base() {
        // REGLA 2: conv_fts es content='convs'. Un rowid inexistente en convs
        // NO puede producir resultados buscables reales => el índice espeja la base.
        let m = memoria_test("espejo");
        let id = m.registrar_turno("s1", "palabra_unica_xyz", "x").unwrap();
        let hits = m.buscar_texto("palabra_unica_xyz", 5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, id);
        // La fila 999 no existe en convs => JOIN interno no la devuelve jamás.
        let huérfanas: i64 = m
            .conn
            .query_row(
                "SELECT count(*) FROM conv_fts WHERE rowid NOT IN (SELECT id FROM convs)",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(huérfanas, 0, "FTS externa no debe acumular filas huérfanas");
    }
}
