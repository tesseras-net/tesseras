-- Content-addressable storage for deduplication
CREATE TABLE IF NOT EXISTS cas_objects (
    blake3_hash TEXT PRIMARY KEY,
    size_bytes  INTEGER NOT NULL,
    ref_count   INTEGER NOT NULL DEFAULT 1,
    stored_at   TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS blob_refs (
    tessera_hash TEXT NOT NULL,
    memory_hash  TEXT NOT NULL,
    filename     TEXT NOT NULL,
    blake3_hash  TEXT NOT NULL REFERENCES cas_objects(blake3_hash),
    PRIMARY KEY (tessera_hash, memory_hash, filename)
);

CREATE TABLE IF NOT EXISTS fragment_refs (
    tessera_hash   TEXT    NOT NULL,
    fragment_index INTEGER NOT NULL,
    blake3_hash    TEXT    NOT NULL REFERENCES cas_objects(blake3_hash),
    PRIMARY KEY (tessera_hash, fragment_index)
);

CREATE INDEX IF NOT EXISTS idx_blob_refs_hash ON blob_refs(blake3_hash);
CREATE INDEX IF NOT EXISTS idx_fragment_refs_hash ON fragment_refs(blake3_hash);

-- Storage version metadata for migration tracking
CREATE TABLE IF NOT EXISTS storage_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO storage_meta (key, value) VALUES ('storage_version', '1');
