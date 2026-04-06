ALTER TABLE rules
ADD COLUMN action_scope TEXT NOT NULL DEFAULT 'translation'
CHECK (action_scope IN ('translation', 'retranslation', 'qa', 'export', 'consistency_review'));

CREATE INDEX IF NOT EXISTS idx_rules_rule_set_scope_listing
  ON rules (rule_set_id, action_scope, is_enabled DESC, severity ASC, name COLLATE NOCASE);
