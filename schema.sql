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

CREATE VIRTUAL TABLE IF NOT EXISTS conv_fts USING fts5(
    user, assistant,
    tokenize = "unicode61 remove_diacritics 2"
);

CREATE TRIGGER IF NOT EXISTS convs_ai AFTER INSERT ON convs BEGIN
    INSERT INTO conv_fts(rowid, user, assistant) VALUES (new.id, new.user, new.assistant);
END;
CREATE TRIGGER IF NOT EXISTS convs_ad AFTER DELETE ON convs BEGIN
    DELETE FROM conv_fts WHERE rowid = old.id;
END;
CREATE TRIGGER IF NOT EXISTS convs_au AFTER UPDATE ON convs BEGIN
    DELETE FROM conv_fts WHERE rowid = old.id;
    INSERT INTO conv_fts(rowid, user, assistant) VALUES (new.id, new.user, new.assistant);
END;
