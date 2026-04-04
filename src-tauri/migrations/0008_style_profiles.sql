CREATE TABLE IF NOT EXISTS style_profiles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  tone TEXT NOT NULL,
  formality TEXT NOT NULL,
  treatment_preference TEXT NOT NULL,
  consistency_instructions TEXT,
  editorial_notes TEXT,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  last_opened_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_style_profiles_status_opened
  ON style_profiles (status, last_opened_at DESC, created_at DESC);
