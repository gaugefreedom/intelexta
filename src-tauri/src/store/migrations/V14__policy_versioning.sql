-- V14__policy_versioning.sql
-- Add policy versioning support

-- Create new table for policy history
CREATE TABLE IF NOT EXISTS policy_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    policy_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT, -- Optional: user/system identifier
    change_notes TEXT, -- Optional: description of changes
    FOREIGN KEY (project_id) REFERENCES projects(id),
    UNIQUE(project_id, version)
);

-- Add current_policy_version to policies table
ALTER TABLE policies ADD COLUMN current_version INTEGER NOT NULL DEFAULT 1;

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_policy_versions_project ON policy_versions(project_id, version DESC);

-- Add policy_version to runs table to track which policy was active
ALTER TABLE runs ADD COLUMN policy_version INTEGER;

-- Migrate existing policies to version 1
-- For each existing policy, create a version 1 entry
INSERT INTO policy_versions (project_id, version, policy_json, created_at, created_by, change_notes)
SELECT
    project_id,
    1 as version,
    policy_json,
    CURRENT_TIMESTAMP as created_at,
    'system' as created_by,
    'Initial policy version (migrated from legacy policies table)' as change_notes
FROM policies;
