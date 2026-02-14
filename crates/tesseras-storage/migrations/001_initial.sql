CREATE TABLE IF NOT EXISTS tesseras (
    hash TEXT PRIMARY KEY,
    creator_pubkey TEXT NOT NULL,
    created_at TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    memory_count INTEGER NOT NULL,
    visibility TEXT NOT NULL,
    sealed_until TEXT,
    is_mine BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS memories (
    hash TEXT PRIMARY KEY,
    tessera_hash TEXT NOT NULL REFERENCES tesseras(hash) ON DELETE CASCADE,
    memory_type TEXT NOT NULL,
    media_path TEXT NOT NULL,
    context_path TEXT,
    meta_json TEXT,
    created_at TEXT NOT NULL
);
