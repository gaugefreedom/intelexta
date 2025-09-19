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
    kind TEXT NOT NULL DEFAULT 'exact', -- 'exact' | 'concordant' | 'interactive'
    spec_json TEXT NOT NULL, -- Serialized RunSpec
    sampler_json TEXT, -- Optional sampler config
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
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
    FOREIGN KEY (parent_checkpoint_id) REFERENCES checkpoints(id)
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
