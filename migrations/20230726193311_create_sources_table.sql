CREATE TABLE IF NOT EXISTS sources (
    id TEXT NOT NULL PRIMARY KEY,
    owner TEXT NOT NULL,
    repo TEXT NOT NULL,
    branch TEXT NOT NULL,
    allowed_ext TEXT NOT NULL,
    allowed_dirs TEXT NOT NULL,
    ignored_dirs TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
