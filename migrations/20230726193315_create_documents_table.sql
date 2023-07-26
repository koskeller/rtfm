CREATE TABLE IF NOT EXISTS documents (
    id TEXT NOT NULL PRIMARY KEY,
    path TEXT NOT NULL,
    checksum INTEGER NOT NULL,
    tokens INTEGER NOT NULL,
    blob TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
