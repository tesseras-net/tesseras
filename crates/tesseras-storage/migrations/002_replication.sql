CREATE TABLE IF NOT EXISTS fragments (
    tessera_hash     TEXT    NOT NULL,
    fragment_index   INTEGER NOT NULL,
    is_parity        INTEGER NOT NULL,
    checksum         TEXT    NOT NULL,
    size_bytes       INTEGER NOT NULL,
    blob_path        TEXT    NOT NULL UNIQUE,
    stored_at        TEXT    NOT NULL,
    last_verified    TEXT    NOT NULL,
    origin_peer      TEXT,
    PRIMARY KEY (tessera_hash, fragment_index)
);

CREATE INDEX IF NOT EXISTS idx_fragments_origin_peer
    ON fragments(origin_peer)
    WHERE origin_peer IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_fragments_last_verified
    ON fragments(last_verified);

CREATE TABLE IF NOT EXISTS fragment_plans (
    tessera_hash          TEXT    PRIMARY KEY,
    tier                  TEXT    NOT NULL,
    original_tessera_size INTEGER NOT NULL,
    data_shards           INTEGER,
    parity_shards         INTEGER,
    fragment_size         INTEGER,
    replication_factor    INTEGER NOT NULL,
    created_at            TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS holders (
    tessera_hash    TEXT    NOT NULL,
    node_id         TEXT    NOT NULL,
    addr            TEXT    NOT NULL,
    subnet          TEXT    NOT NULL,
    last_attested   TEXT,
    last_seen       TEXT    NOT NULL,
    PRIMARY KEY (tessera_hash, node_id)
);

CREATE INDEX IF NOT EXISTS idx_holders_last_seen ON holders(last_seen);

CREATE TABLE IF NOT EXISTS holder_fragments (
    tessera_hash   TEXT    NOT NULL,
    node_id        TEXT    NOT NULL,
    fragment_index INTEGER NOT NULL,
    PRIMARY KEY (tessera_hash, node_id, fragment_index),
    FOREIGN KEY (tessera_hash, node_id) REFERENCES holders(tessera_hash, node_id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS reciprocity (
    peer_id                  TEXT    PRIMARY KEY,
    bytes_stored_for_them    INTEGER NOT NULL DEFAULT 0,
    bytes_they_store_for_us  INTEGER NOT NULL DEFAULT 0,
    balance                  INTEGER GENERATED ALWAYS AS (
        bytes_they_store_for_us - bytes_stored_for_them
    ) STORED,
    last_updated             TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reciprocity_balance ON reciprocity(balance);
