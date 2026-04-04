CREATE TABLE IF NOT EXISTS glossaries (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 120),
  description TEXT CHECK (description IS NULL OR length(description) <= 1000),
  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
  status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  last_opened_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_glossaries_status_last_opened_at
ON glossaries (
  status,
  last_opened_at DESC,
  created_at DESC,
  name COLLATE NOCASE ASC
);

CREATE INDEX IF NOT EXISTS idx_glossaries_project_status_name
ON glossaries (project_id, status, name COLLATE NOCASE ASC);
