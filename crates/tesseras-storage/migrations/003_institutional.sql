-- Institutional node support: search index and reciprocity extension.

-- Add institutional flag to reciprocity table (idempotent via pragma check).
-- SQLite ALTER TABLE ADD COLUMN does not support IF NOT EXISTS,
-- so we check pragma table_info first via the migration runner.

-- FTS5 virtual table for title/description text search.
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
    tessera_hash,
    title,
    description,
    memory_type,
    language,
    tags,
    created_at UNINDEXED,
    visibility UNINDEXED
);

-- R-tree for geo queries (bounding box).
CREATE VIRTUAL TABLE IF NOT EXISTS search_index_geo USING rtree(
    id,
    min_lat, max_lat,
    min_lon, max_lon
);

-- Map R-tree row IDs to tessera hashes.
CREATE TABLE IF NOT EXISTS search_index_geo_map (
    id          INTEGER PRIMARY KEY,
    tessera_hash TEXT NOT NULL UNIQUE
);
