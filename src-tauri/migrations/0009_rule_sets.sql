CREATE TABLE IF NOT EXISTS rule_sets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived')),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  last_opened_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS rules (
  id TEXT PRIMARY KEY,
  rule_set_id TEXT NOT NULL,
  rule_type TEXT NOT NULL,
  severity TEXT NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  guidance TEXT NOT NULL,
  is_enabled INTEGER NOT NULL DEFAULT 1 CHECK (is_enabled IN (0, 1)),
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  FOREIGN KEY (rule_set_id) REFERENCES rule_sets(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_rule_sets_status_opened
  ON rule_sets (status, last_opened_at DESC, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_rules_rule_set_listing
  ON rules (rule_set_id, is_enabled DESC, severity ASC, name COLLATE NOCASE);
