CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('microsoft', 'offline')),
    username TEXT NOT NULL,
    mc_uuid TEXT,
    skin_url TEXT,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    last_used_at TEXT
);

CREATE TABLE IF NOT EXISTS kv_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
