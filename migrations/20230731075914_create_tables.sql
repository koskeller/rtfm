CREATE TABLE IF NOT EXISTS collection (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_collection_name ON collection(name);

CREATE TABLE IF NOT EXISTS source (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    collection_id INTEGER NOT NULL,
    owner TEXT NOT NULL,
    repo TEXT NOT NULL,
    branch TEXT NOT NULL,
    allowed_ext TEXT NOT NULL,
    allowed_dirs TEXT NOT NULL,
    ignored_dirs TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (collection_id) REFERENCES collection(id)
);

CREATE INDEX IF NOT EXISTS idx_source_collection ON source(collection_id);

CREATE TABLE IF NOT EXISTS document (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL,
    collection_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    checksum INTEGER NOT NULL,
    tokens_len INTEGER NOT NULL,
    data TEXT NOT NULl,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_id) REFERENCES source(id),
    FOREIGN KEY (collection_id) REFERENCES collection(id)
);

CREATE INDEX IF NOT EXISTS idx_document_source ON document(source_id);
CREATE INDEX IF NOT EXISTS idx_document_collection ON document(collection_id);

CREATE TABLE IF NOT EXISTS chunk (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    document_id INTEGER NOT NULL,
    source_id INTEGER NOT NULL,
    collection_id INTEGER NOT NULL,
    chunk_index INTEGER NOT NULL,
    context TEXT NOT NULL,
    data TEXT NOT NULL,
    vector BLOB NOT NULL,
    FOREIGN KEY (document_id) REFERENCES document(id),
    FOREIGN KEY (source_id) REFERENCES source(id),
    FOREIGN KEY (collection_id) REFERENCES collection(id)
);

CREATE INDEX IF NOT EXISTS idx_chunk_document ON chunk(document_id);
CREATE INDEX IF NOT EXISTS idx_chunk_source ON chunk(source_id);
CREATE INDEX IF NOT EXISTS idx_chunk_collection ON chunk(collection_id);
