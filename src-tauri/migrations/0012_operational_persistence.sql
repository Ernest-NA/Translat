CREATE TABLE IF NOT EXISTS task_runs (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  chunk_id TEXT REFERENCES translation_chunks(id) ON DELETE SET NULL,
  job_id TEXT,
  action_type TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
  input_payload TEXT,
  output_payload TEXT,
  error_message TEXT,
  started_at INTEGER NOT NULL,
  completed_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_task_runs_document_created
ON task_runs (document_id, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_task_runs_chunk
ON task_runs (chunk_id);

CREATE INDEX IF NOT EXISTS idx_task_runs_job
ON task_runs (job_id);

CREATE TABLE IF NOT EXISTS chapter_contexts (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  section_id TEXT REFERENCES document_sections(id) ON DELETE SET NULL,
  task_run_id TEXT REFERENCES task_runs(id) ON DELETE SET NULL,
  scope_type TEXT NOT NULL CHECK (scope_type IN ('document', 'chapter', 'section', 'range')),
  start_segment_sequence INTEGER NOT NULL CHECK (start_segment_sequence >= 1),
  end_segment_sequence INTEGER NOT NULL CHECK (end_segment_sequence >= start_segment_sequence),
  context_text TEXT NOT NULL,
  source_summary TEXT,
  context_word_count INTEGER NOT NULL CHECK (context_word_count >= 0),
  context_character_count INTEGER NOT NULL CHECK (context_character_count >= 0),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chapter_contexts_document_scope
ON chapter_contexts (document_id, start_segment_sequence ASC, end_segment_sequence ASC);

CREATE INDEX IF NOT EXISTS idx_chapter_contexts_section
ON chapter_contexts (section_id);

CREATE INDEX IF NOT EXISTS idx_chapter_contexts_task_run
ON chapter_contexts (task_run_id);

CREATE TABLE IF NOT EXISTS qa_findings (
  id TEXT PRIMARY KEY,
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  chunk_id TEXT REFERENCES translation_chunks(id) ON DELETE SET NULL,
  task_run_id TEXT REFERENCES task_runs(id) ON DELETE SET NULL,
  job_id TEXT,
  finding_type TEXT NOT NULL,
  severity TEXT NOT NULL CHECK (severity IN ('low', 'medium', 'high')),
  status TEXT NOT NULL CHECK (status IN ('open', 'resolved', 'dismissed')),
  message TEXT NOT NULL,
  details TEXT,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_qa_findings_document_created
ON qa_findings (document_id, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_qa_findings_chunk
ON qa_findings (chunk_id);

CREATE INDEX IF NOT EXISTS idx_qa_findings_task_run
ON qa_findings (task_run_id);

CREATE INDEX IF NOT EXISTS idx_qa_findings_job
ON qa_findings (job_id);
