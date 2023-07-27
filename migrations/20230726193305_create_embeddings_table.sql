CREATE TABLE IF NOT EXISTS embeddings (
    source_id TEXT NOT NULL,
    doc_path TEXT NOT NULL,
    chunk INTEGER NOT NULL,
    blob TEXT NOT NULL,
    vector BLOB NOT NULL,
    PRIMARY KEY (source_id, doc_path)
);
