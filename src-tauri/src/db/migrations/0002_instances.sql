CREATE TABLE IF NOT EXISTS instances (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    loader TEXT NOT NULL DEFAULT 'vanilla',
    mc_version TEXT NOT NULL,
    memory_mb INTEGER,
    jvm_args TEXT,
    java_path TEXT,
    icon_biome TEXT NOT NULL DEFAULT 'ore',
    group_name TEXT,
    created_at TEXT NOT NULL,
    last_played_at TEXT
);
