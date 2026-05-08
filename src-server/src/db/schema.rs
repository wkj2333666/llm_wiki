pub const SCHEMA: &str = r#"
-- Users
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user',
    created_at INTEGER NOT NULL
);

-- App configuration (key-value store)
CREATE TABLE IF NOT EXISTS app_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL DEFAULT '{}'
);

-- Projects registry
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    last_opened_at INTEGER NOT NULL
);

-- Initialize default config keys
INSERT OR IGNORE INTO app_config (key, value) VALUES
    ('recent_projects', '[]'),
    ('last_project', 'null'),
    ('llm_config', '{}'),
    ('provider_configs', '{}'),
    ('active_preset_id', 'null'),
    ('search_api_config', '{}'),
    ('embedding_config', '{}'),
    ('multimodal_config', '{}'),
    ('language', 'zh'),
    ('output_language', 'Chinese'),
    ('update_check_state', '{"enabled":true,"lastCheckedAt":null,"dismissedVersion":null}');
"#;