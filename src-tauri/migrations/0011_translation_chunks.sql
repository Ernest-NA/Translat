CREATE TABLE IF NOT EXISTS translation_chunks (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL CHECK (sequence >= 1),
  builder_version TEXT NOT NULL,
  strategy TEXT NOT NULL,
  source_text TEXT NOT NULL,
  context_before_text TEXT,
  context_after_text TEXT,
  start_segment_sequence INTEGER NOT NULL CHECK (start_segment_sequence >= 1),
  end_segment_sequence INTEGER NOT NULL CHECK (end_segment_sequence >= start_segment_sequence),
  segment_count INTEGER NOT NULL CHECK (segment_count >= 1),
  source_word_count INTEGER NOT NULL CHECK (source_word_count >= 0),
  source_character_count INTEGER NOT NULL CHECK (source_character_count >= 0),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(document_id, sequence)
);

CREATE INDEX IF NOT EXISTS idx_translation_chunks_document_sequence
ON translation_chunks (document_id, sequence ASC);

CREATE TABLE IF NOT EXISTS translation_chunk_segments (
  chunk_id TEXT NOT NULL REFERENCES translation_chunks(id) ON DELETE CASCADE,
  segment_id TEXT NOT NULL REFERENCES segments(id) ON DELETE CASCADE,
  segment_sequence INTEGER NOT NULL CHECK (segment_sequence >= 1),
  position INTEGER NOT NULL CHECK (position >= 1),
  role TEXT NOT NULL CHECK (role IN ('core', 'context_before', 'context_after')),
  PRIMARY KEY (chunk_id, segment_id, role)
);

CREATE INDEX IF NOT EXISTS idx_translation_chunk_segments_chunk_position
ON translation_chunk_segments (chunk_id, position ASC);

CREATE INDEX IF NOT EXISTS idx_translation_chunk_segments_segment
ON translation_chunk_segments (segment_id);
