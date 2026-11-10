-- Esquema de referencia para la memoria conversacional.
--
-- La fuente de verdad es `initialize_schema()` en `src/memory_vault.rs`; este
-- archivo debe mantenerse sincronizado con ella. La tabla `docs`/`docs_vec`
-- (sqlite-vec) solo la crea el daemon, porque requiere cargar la extensión.

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

-- FTS5 con contenido externo: el índice no duplica el texto de `convs`.
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
END;
