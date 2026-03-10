-- Fear Engine: Initial Schema
-- All timestamps are UTC, stored as TEXT in SQLite's CURRENT_TIMESTAMP format.

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    player_name TEXT,
    current_scene_id TEXT NOT NULL,
    game_phase TEXT NOT NULL DEFAULT 'calibrating',
    game_state_json TEXT NOT NULL DEFAULT '{}',
    completed BOOLEAN DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS fear_profiles (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id),
    claustrophobia REAL DEFAULT 0.5,
    isolation REAL DEFAULT 0.5,
    body_horror REAL DEFAULT 0.5,
    stalking REAL DEFAULT 0.5,
    loss_of_control REAL DEFAULT 0.5,
    uncanny_valley REAL DEFAULT 0.5,
    darkness REAL DEFAULT 0.5,
    sound_based REAL DEFAULT 0.5,
    doppelganger REAL DEFAULT 0.5,
    abandonment REAL DEFAULT 0.5,
    anxiety_threshold REAL DEFAULT 0.5,
    recovery_speed REAL DEFAULT 0.5,
    curiosity_vs_avoidance REAL DEFAULT 0.5,
    confidence_json TEXT DEFAULT '{}',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS behavior_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT REFERENCES sessions(id),
    event_type TEXT NOT NULL,
    event_data_json TEXT NOT NULL,
    scene_id TEXT,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS content_cache (
    cache_key TEXT PRIMARY KEY,
    content_type TEXT NOT NULL,
    content_json TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    ttl_seconds INTEGER DEFAULT 3600
);

CREATE TABLE IF NOT EXISTS scene_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT REFERENCES sessions(id),
    scene_id TEXT NOT NULL,
    narrative_text TEXT,
    player_choice TEXT,
    fear_profile_snapshot_json TEXT,
    adaptation_strategy TEXT,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_behavior_session ON behavior_events(session_id);
CREATE INDEX IF NOT EXISTS idx_behavior_timestamp ON behavior_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_scene_history_session ON scene_history(session_id);
CREATE INDEX IF NOT EXISTS idx_content_cache_ttl ON content_cache(created_at, ttl_seconds);
