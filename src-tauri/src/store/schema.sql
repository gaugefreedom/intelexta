CREATE TABLE IF NOT EXISTS project (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  settings_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS item (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  source_path TEXT,
  content TEXT,
  type TEXT NOT NULL, -- 'NOTE', 'PDF', 'MD', 'URL'
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  FOREIGN KEY (project_id) REFERENCES project(id)
);

CREATE TABLE IF NOT EXISTS chunk (
  id TEXT PRIMARY KEY,
  item_id TEXT NOT NULL,
  chunk_index INTEGER NOT NULL,
  content TEXT NOT NULL,
  token_count INTEGER NOT NULL,
  FOREIGN KEY (item_id) REFERENCES item(id)
);

CREATE TABLE IF NOT EXISTS checkpoint (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  summary TEXT NOT NULL,
  decisions_json TEXT NOT NULL, -- JSON array of strings
  todos_json TEXT NOT NULL,     -- JSON array of strings
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  FOREIGN KEY (project_id) REFERENCES project(id)
);

CREATE TABLE IF NOT EXISTS checkpoint_keypoint (
  id TEXT PRIMARY KEY,
  checkpoint_id TEXT NOT NULL,
  text TEXT NOT NULL,
  citations_json TEXT NOT NULL, -- JSON array of chunk_ids
  FOREIGN KEY (checkpoint_id) REFERENCES checkpoint(id)
);