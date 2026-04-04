ALTER TABLE projects
ADD COLUMN default_glossary_id TEXT
REFERENCES glossaries(id) ON DELETE SET NULL;

ALTER TABLE projects
ADD COLUMN default_style_profile_id TEXT
REFERENCES style_profiles(id) ON DELETE SET NULL;

ALTER TABLE projects
ADD COLUMN default_rule_set_id TEXT
REFERENCES rule_sets(id) ON DELETE SET NULL;
