PRAGMA foreign_keys = OFF;

ALTER TABLE translation_chunk_segments RENAME TO translation_chunk_segments_legacy_tr15;
ALTER TABLE segments RENAME TO segments_legacy_tr15;

CREATE TABLE segments (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  sequence INTEGER NOT NULL CHECK (sequence >= 1),
  source_text TEXT NOT NULL CHECK (length(trim(source_text)) >= 1),
  target_text TEXT,
  source_word_count INTEGER NOT NULL CHECK (source_word_count >= 0),
  source_character_count INTEGER NOT NULL CHECK (source_character_count >= 0),
  status TEXT NOT NULL CHECK (status IN ('pending_translation', 'translated')),
  last_task_run_id TEXT REFERENCES task_runs(id) ON DELETE SET NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(document_id, sequence)
);

INSERT INTO segments (
  id,
  document_id,
  sequence,
  source_text,
  target_text,
  source_word_count,
  source_character_count,
  status,
  last_task_run_id,
  created_at,
  updated_at
)
SELECT
  id,
  document_id,
  sequence,
  source_text,
  NULL,
  source_word_count,
  source_character_count,
  status,
  NULL,
  created_at,
  updated_at
FROM segments_legacy_tr15;

DROP TABLE segments_legacy_tr15;

CREATE TABLE translation_chunk_segments (
  chunk_id TEXT NOT NULL REFERENCES translation_chunks(id) ON DELETE CASCADE,
  segment_id TEXT NOT NULL REFERENCES segments(id) ON DELETE CASCADE,
  segment_sequence INTEGER NOT NULL CHECK (segment_sequence >= 1),
  position INTEGER NOT NULL CHECK (position >= 1),
  role TEXT NOT NULL CHECK (role IN ('core', 'context_before', 'context_after')),
  PRIMARY KEY (chunk_id, segment_id, role)
);

INSERT INTO translation_chunk_segments (
  chunk_id,
  segment_id,
  segment_sequence,
  position,
  role
)
SELECT
  chunk_id,
  segment_id,
  segment_sequence,
  position,
  role
FROM translation_chunk_segments_legacy_tr15;

DROP TABLE translation_chunk_segments_legacy_tr15;

CREATE INDEX IF NOT EXISTS idx_segments_document_sequence
ON segments (document_id, sequence ASC);

CREATE INDEX IF NOT EXISTS idx_segments_last_task_run
ON segments (last_task_run_id);

CREATE INDEX IF NOT EXISTS idx_translation_chunk_segments_chunk_position
ON translation_chunk_segments (chunk_id, position ASC);

CREATE INDEX IF NOT EXISTS idx_translation_chunk_segments_segment
ON translation_chunk_segments (segment_id);

PRAGMA foreign_keys = ON;
