CREATE TABLE IF NOT EXISTS glossary_entries (
  id TEXT PRIMARY KEY,
  glossary_id TEXT NOT NULL REFERENCES glossaries(id) ON DELETE CASCADE,
  source_term TEXT NOT NULL CHECK (length(trim(source_term)) BETWEEN 1 AND 240),
  target_term TEXT NOT NULL CHECK (length(trim(target_term)) BETWEEN 1 AND 240),
  context_note TEXT CHECK (context_note IS NULL OR length(context_note) <= 2000),
  status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS glossary_entry_variants (
  id TEXT PRIMARY KEY,
  glossary_entry_id TEXT NOT NULL REFERENCES glossary_entries(id) ON DELETE CASCADE,
  variant_text TEXT NOT NULL CHECK (length(trim(variant_text)) BETWEEN 1 AND 240),
  variant_type TEXT NOT NULL CHECK (variant_type IN ('source', 'target', 'forbidden')),
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_glossary_entries_glossary_status_source_term
ON glossary_entries (
  glossary_id,
  status,
  source_term COLLATE NOCASE ASC,
  updated_at DESC
);

CREATE INDEX IF NOT EXISTS idx_glossary_entries_glossary_status_target_term
ON glossary_entries (
  glossary_id,
  status,
  target_term COLLATE NOCASE ASC,
  updated_at DESC
);

CREATE INDEX IF NOT EXISTS idx_glossary_entry_variants_entry_type_text
ON glossary_entry_variants (
  glossary_entry_id,
  variant_type,
  variant_text COLLATE NOCASE ASC
);
