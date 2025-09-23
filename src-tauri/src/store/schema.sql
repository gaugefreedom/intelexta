-- In src-tauri/src/store/schema.sql

-- For managing schema evolution idempotently
CREATE TABLE IF NOT EXISTS migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    pubkey TEXT NOT NULL -- Ed25519 public key in base64
);

CREATE TABLE IF NOT EXISTS policies (
    project_id TEXT PRIMARY KEY,
    policy_json TEXT NOT NULL, -- The full Policy struct as JSON
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'exact', -- 'exact' | 'concordant'
    spec_json TEXT NOT NULL, -- Serialized RunSpec (deprecated; retained for compatibility)
    sampler_json TEXT, -- Optional sampler config
    seed INTEGER NOT NULL DEFAULT 0,
    epsilon REAL,
    token_budget INTEGER NOT NULL DEFAULT 0,
    default_model TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    checkpoint_config_id TEXT,
    parent_checkpoint_id TEXT, -- For chaining interactive turns
    turn_index INTEGER,        -- Strict ordering for interactive mode
    kind TEXT NOT NULL DEFAULT 'Step', -- 'Step' | 'Incident'
    incident_json TEXT,        -- Details if kind = 'Incident'
    timestamp TEXT NOT NULL,
    inputs_sha256 TEXT,
    outputs_sha256 TEXT,
    prev_chain TEXT,
    curr_chain TEXT NOT NULL UNIQUE,
    signature TEXT NOT NULL,
    usage_tokens INTEGER NOT NULL DEFAULT 0,
    prompt_tokens INTEGER NOT NULL DEFAULT 0,
    completion_tokens INTEGER NOT NULL DEFAULT 0,
    semantic_digest TEXT,
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (parent_checkpoint_id) REFERENCES checkpoints(id),
    FOREIGN KEY (checkpoint_config_id) REFERENCES run_checkpoints(id)
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_config_id
    ON checkpoints(checkpoint_config_id);

CREATE TABLE IF NOT EXISTS checkpoint_messages (
    checkpoint_id TEXT PRIMARY KEY,
    role TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS checkpoint_payloads (
    checkpoint_id TEXT PRIMARY KEY,
    prompt_payload TEXT,
    output_payload TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS receipts (
    id TEXT PRIMARY KEY, -- The CAR ID (sha256 of canonical body)
    run_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    file_path TEXT NOT NULL,
    match_kind TEXT,     -- Result of replay: 'exact'|'semantic'|'process'
    epsilon REAL,        -- Tolerance for concordant match
    s_grade INTEGER,     -- Provenance score (0-100)
    FOREIGN KEY (run_id) REFERENCES runs(id)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_runs_project ON runs(project_id);
CREATE INDEX IF NOT EXISTS idx_ckpt_run ON checkpoints(run_id);

CREATE TABLE IF NOT EXISTS run_checkpoints (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    order_index INTEGER NOT NULL,
    checkpoint_type TEXT NOT NULL DEFAULT 'Step',
    model TEXT NOT NULL,
    prompt TEXT NOT NULL,
    token_budget INTEGER NOT NULL,
    proof_mode TEXT NOT NULL DEFAULT 'exact',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (run_id) REFERENCES runs(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_run_checkpoints_order
    ON run_checkpoints(run_id, order_index);

CREATE INDEX IF NOT EXISTS idx_run_checkpoints_run_id
    ON run_checkpoints(run_id);
