CREATE TABLE IF NOT EXISTS document_sections (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL CHECK (sequence >= 1),
  title TEXT NOT NULL CHECK (length(trim(title)) BETWEEN 1 AND 255),
  section_type TEXT NOT NULL CHECK (section_type IN ('document', 'chapter', 'section')),
  level INTEGER NOT NULL CHECK (level BETWEEN 1 AND 3),
  start_segment_sequence INTEGER NOT NULL CHECK (start_segment_sequence >= 1),
  end_segment_sequence INTEGER NOT NULL CHECK (end_segment_sequence >= start_segment_sequence),
  segment_count INTEGER NOT NULL CHECK (segment_count >= 1),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(document_id, sequence)
);

CREATE INDEX IF NOT EXISTS idx_document_sections_document_sequence
ON document_sections (document_id, sequence ASC);
