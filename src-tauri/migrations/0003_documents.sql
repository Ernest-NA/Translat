CREATE TABLE IF NOT EXISTS documents (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 255),
  source_kind TEXT NOT NULL CHECK (source_kind IN ('local_file')),
  format TEXT NOT NULL CHECK (length(trim(format)) BETWEEN 1 AND 40),
  mime_type TEXT CHECK (mime_type IS NULL OR length(mime_type) <= 255),
  stored_path TEXT NOT NULL,
  file_size_bytes INTEGER NOT NULL CHECK (file_size_bytes >= 0),
  status TEXT NOT NULL CHECK (status IN ('imported')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_documents_project_created_at
ON documents (project_id, created_at DESC);
