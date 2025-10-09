-- V15__project_usage_ledgers.sql
-- Track cumulative project usage for policy enforcement

CREATE TABLE IF NOT EXISTS project_usage_ledgers (
    project_id TEXT NOT NULL,
    policy_version INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    total_usd REAL NOT NULL DEFAULT 0,
    total_nature_cost REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (project_id, policy_version),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_project_usage_ledgers_project
    ON project_usage_ledgers(project_id);
