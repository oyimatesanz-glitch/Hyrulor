-- Esquema de referencia que usa el prototipo `legacy-go/orchestrator_secure.go`.
--
-- Debe coincidir con `initialize_schema()` en `src/memory_vault.rs` (salvo
-- `docs`/`docs_vec`, que requieren la extensión sqlite-vec y solo crea el
-- daemon Rust). Todas las escrituras pasan por `convs`; los triggers mantienen
-- el índice FTS5 en sincronía.

PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS convs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts TEXT NOT NULL DEFAULT (datetime('now')),
    user TEXT NOT NULL,
    assistant TEXT NOT NULL
);

-- FTS5 con contenido externo (content='convs') + tokenizer español.
CREATE VIRTUAL TABLE IF NOT EXISTS conv_fts USING fts5(
    user, assistant, content='convs', content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

-- TRIGGERS AUTOMÁTICOS (la clave de la consistencia del índice).
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
END;
