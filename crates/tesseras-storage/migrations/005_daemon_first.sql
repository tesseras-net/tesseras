-- Tombstone records for retracted tesseras
CREATE TABLE IF NOT EXISTS tombstones (
    hash TEXT PRIMARY KEY,
    retracted_at TEXT NOT NULL,
    creator_pubkey TEXT NOT NULL,
    ed25519_signature BLOB NOT NULL,
    mldsa_signature BLOB NOT NULL
);

-- Named circles for Trusted Wheel visibility
CREATE TABLE IF NOT EXISTS circles (
    name TEXT PRIMARY KEY,
    symmetric_key BLOB NOT NULL,
    created_at TEXT NOT NULL
);

-- Circle membership: which contacts belong to which circle
CREATE TABLE IF NOT EXISTS circle_members (
    circle_name TEXT NOT NULL REFERENCES circles(name) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    pubkey TEXT NOT NULL,
    wrapped_key BLOB NOT NULL,
    added_at TEXT NOT NULL,
    PRIMARY KEY (circle_name, alias)
);

-- Persistent operation queue for offline push/pull/delete
CREATE TABLE IF NOT EXISTS operation_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    op_type TEXT NOT NULL,
    payload BLOB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    completed_at TEXT,
    error TEXT,
    retries INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_operation_queue_status ON operation_queue(status);
