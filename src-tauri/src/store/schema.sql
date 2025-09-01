-- §9 Data Model from Strategic Spec v0.1
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    pubkey TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    mime TEXT,
    added_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS runs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    dag_json TEXT,
    policy_digest TEXT,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS checkpoints (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    run_id TEXT,
    parent_id TEXT,
    json_data TEXT NOT NULL, -- The full canonical JSON of the checkpoint
    sha256 TEXT NOT NULL,
    chain_hash TEXT NOT NULL, -- The hash of (parent_chain_hash || current_sha256)
    signature TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE,
    FOREIGN KEY (run_id) REFERENCES runs (id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS policies (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL UNIQUE, -- Each project has one policy
    json_policy TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS metrics (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    tokens_in INTEGER,
    tokens_out INTEGER,
    usd_cost REAL,
    gco2e REAL,
    latency_ms INTEGER,
    FOREIGN KEY (run_id) REFERENCES runs (id) ON DELETE CASCADE
);