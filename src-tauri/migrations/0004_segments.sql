ALTER TABLE documents RENAME TO documents_legacy_c3;

CREATE TABLE documents (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 255),
  source_kind TEXT NOT NULL CHECK (source_kind IN ('local_file')),
  format TEXT NOT NULL CHECK (length(trim(format)) BETWEEN 1 AND 40),
  mime_type TEXT CHECK (mime_type IS NULL OR length(mime_type) <= 255),
  stored_path TEXT NOT NULL,
  file_size_bytes INTEGER NOT NULL CHECK (file_size_bytes >= 0),
  status TEXT NOT NULL CHECK (status IN ('imported', 'segmented')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

INSERT INTO documents (
  id,
  project_id,
  name,
  source_kind,
  format,
  mime_type,
  stored_path,
  file_size_bytes,
  status,
  created_at,
  updated_at
)
SELECT
  id,
  project_id,
  name,
  source_kind,
  format,
  mime_type,
  stored_path,
  file_size_bytes,
  status,
  created_at,
  updated_at
FROM documents_legacy_c3;

DROP TABLE documents_legacy_c3;

CREATE INDEX IF NOT EXISTS idx_documents_project_created_at
ON documents (project_id, created_at DESC);

CREATE TABLE IF NOT EXISTS segments (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL CHECK (sequence >= 1),
  source_text TEXT NOT NULL CHECK (length(trim(source_text)) >= 1),
  source_word_count INTEGER NOT NULL CHECK (source_word_count >= 0),
  source_character_count INTEGER NOT NULL CHECK (source_character_count >= 0),
  status TEXT NOT NULL CHECK (status IN ('pending_translation')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(document_id, sequence)
);

CREATE INDEX IF NOT EXISTS idx_segments_document_sequence
ON segments (document_id, sequence ASC);
