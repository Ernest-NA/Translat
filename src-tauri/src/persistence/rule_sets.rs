use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::persistence::error::PersistenceError;
use crate::rule_sets::{
    NewRule, NewRuleSet, RuleChanges, RuleSetChanges, RuleSetRulesOverview, RuleSetSummary,
    RuleSetsOverview, RuleSummary, ACTIVE_RULE_SET_METADATA_KEY,
};

pub struct RuleSetRepository<'connection> {
    connection: &'connection mut Connection,
}

pub struct RuleRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> RuleSetRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn create(
        &mut self,
        new_rule_set: &NewRuleSet,
    ) -> Result<RuleSetSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The rule-set repository could not start the rule-set creation transaction.",
                error,
            )
        })?;

        transaction
            .execute(
                r#"
                INSERT INTO rule_sets (
                  id,
                  name,
                  description,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    new_rule_set.id,
                    new_rule_set.name,
                    new_rule_set.description,
                    new_rule_set.status,
                    new_rule_set.created_at,
                    new_rule_set.updated_at,
                    new_rule_set.last_opened_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The rule-set repository could not persist the new rule set.",
                    error,
                )
            })?;

        upsert_active_rule_set(&transaction, &new_rule_set.id, new_rule_set.updated_at)?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The rule-set repository could not commit the rule-set creation transaction.",
                error,
            )
        })?;

        Ok(map_rule_set_summary_from_new(new_rule_set))
    }

    pub fn list(&mut self) -> Result<Vec<RuleSetSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM rule_sets
                ORDER BY
                  CASE status WHEN 'active' THEN 0 ELSE 1 END ASC,
                  last_opened_at DESC,
                  created_at DESC,
                  name COLLATE NOCASE ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The rule-set repository could not prepare the rule-set listing query.",
                    error,
                )
            })?;

        let rows = statement
            .query_map([], map_rule_set_summary)
            .map_err(|error| {
                PersistenceError::with_details(
                    "The rule-set repository could not read the rule-set list.",
                    error,
                )
            })?;

        let mut rule_sets = Vec::new();

        for row in rows {
            rule_sets.push(row.map_err(|error| {
                PersistenceError::with_details(
                    "The rule-set repository could not decode a rule-set row.",
                    error,
                )
            })?);
        }

        Ok(rule_sets)
    }

    pub fn open_rule_set(
        &mut self,
        rule_set_id: &str,
        opened_at: i64,
    ) -> Result<RuleSetSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The rule-set repository could not start the rule-set opening transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                "UPDATE rule_sets SET last_opened_at = ?2 WHERE id = ?1",
                params![rule_set_id, opened_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The rule-set repository could not mark rule set {rule_set_id} as opened."
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested rule set {rule_set_id} does not exist."
            )));
        }

        upsert_active_rule_set(&transaction, rule_set_id, opened_at)?;

        let rule_set = transaction
            .query_row(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM rule_sets
                WHERE id = ?1
                "#,
                [rule_set_id],
                map_rule_set_summary,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The rule-set repository could not reload rule set {rule_set_id}."),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The rule-set repository could not commit the rule-set opening transaction.",
                error,
            )
        })?;

        Ok(rule_set)
    }

    pub fn update(&mut self, changes: &RuleSetChanges) -> Result<RuleSetSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The rule-set repository could not start the rule-set update transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                r#"
                UPDATE rule_sets
                SET
                  name = ?2,
                  description = ?3,
                  status = ?4,
                  updated_at = ?5
                WHERE id = ?1
                "#,
                params![
                    changes.rule_set_id,
                    changes.name,
                    changes.description,
                    changes.status,
                    changes.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The rule-set repository could not update rule set {}.",
                        changes.rule_set_id
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested rule set {} does not exist.",
                changes.rule_set_id
            )));
        }

        upsert_active_rule_set(&transaction, &changes.rule_set_id, changes.updated_at)?;

        let rule_set = transaction
            .query_row(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM rule_sets
                WHERE id = ?1
                "#,
                [&changes.rule_set_id],
                map_rule_set_summary,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The rule-set repository could not reload rule set {}.",
                        changes.rule_set_id
                    ),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The rule-set repository could not commit the rule-set update transaction.",
                error,
            )
        })?;

        Ok(rule_set)
    }

    pub fn exists(&mut self, rule_set_id: &str) -> Result<bool, PersistenceError> {
        let row = self
            .connection
            .query_row(
                "SELECT 1 FROM rule_sets WHERE id = ?1 LIMIT 1",
                [rule_set_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The rule-set repository could not inspect rule set {rule_set_id}."),
                    error,
                )
            })?;

        Ok(row.is_some())
    }

    pub fn load_overview(&mut self) -> Result<RuleSetsOverview, PersistenceError> {
        Ok(RuleSetsOverview {
            active_rule_set_id: self.active_rule_set_id()?,
            rule_sets: self.list()?,
        })
    }

    fn active_rule_set_id(&mut self) -> Result<Option<String>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                [ACTIVE_RULE_SET_METADATA_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    "The rule-set repository could not load the active rule-set id.",
                    error,
                )
            })
    }
}

impl<'connection> RuleRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn create(&mut self, new_rule: &NewRule) -> Result<RuleSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The rule repository could not start the rule creation transaction.",
                error,
            )
        })?;

        transaction
            .execute(
                r#"
                INSERT INTO rules (
                  id,
                  rule_set_id,
                  action_scope,
                  rule_type,
                  severity,
                  name,
                  description,
                  guidance,
                  is_enabled,
                  created_at,
                  updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    new_rule.id,
                    new_rule.rule_set_id,
                    new_rule.action_scope,
                    new_rule.rule_type,
                    new_rule.severity,
                    new_rule.name,
                    new_rule.description,
                    new_rule.guidance,
                    if new_rule.is_enabled { 1_i64 } else { 0_i64 },
                    new_rule.created_at,
                    new_rule.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The rule repository could not persist the new rule.",
                    error,
                )
            })?;

        touch_rule_set(&transaction, &new_rule.rule_set_id, new_rule.updated_at)?;
        let created_rule = load_rule(&transaction, &new_rule.rule_set_id, &new_rule.id)?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The rule repository could not commit the rule creation transaction.",
                error,
            )
        })?;

        Ok(created_rule)
    }

    pub fn list_by_rule_set(
        &mut self,
        rule_set_id: &str,
    ) -> Result<Vec<RuleSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  rule_set_id,
                  action_scope,
                  rule_type,
                  severity,
                  name,
                  description,
                  guidance,
                  is_enabled,
                  created_at,
                  updated_at
                FROM rules
                WHERE rule_set_id = ?1
                ORDER BY
                  is_enabled DESC,
                  CASE severity
                    WHEN 'high' THEN 0
                    WHEN 'medium' THEN 1
                    ELSE 2
                  END ASC,
                  CASE action_scope
                    WHEN 'translation' THEN 0
                    WHEN 'retranslation' THEN 1
                    WHEN 'qa' THEN 2
                    WHEN 'export' THEN 3
                    ELSE 4
                  END ASC,
                  name COLLATE NOCASE ASC,
                  updated_at DESC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The rule repository could not prepare the listing query for rule set {rule_set_id}."),
                    error,
                )
            })?;

        let rows = statement
            .query_map([rule_set_id], map_rule_summary)
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The rule repository could not read rules for rule set {rule_set_id}."),
                    error,
                )
            })?;

        let mut rules = Vec::new();

        for row in rows {
            rules.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!("The rule repository could not decode a rule row for rule set {rule_set_id}."),
                    error,
                )
            })?);
        }

        Ok(rules)
    }

    pub fn update(&mut self, changes: &RuleChanges) -> Result<RuleSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The rule repository could not start the rule update transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                r#"
                UPDATE rules
                SET
                  action_scope = ?3,
                  rule_type = ?4,
                  severity = ?5,
                  name = ?6,
                  description = ?7,
                  guidance = ?8,
                  is_enabled = ?9,
                  updated_at = ?10
                WHERE id = ?1 AND rule_set_id = ?2
                "#,
                params![
                    changes.rule_id,
                    changes.rule_set_id,
                    changes.action_scope,
                    changes.rule_type,
                    changes.severity,
                    changes.name,
                    changes.description,
                    changes.guidance,
                    if changes.is_enabled { 1_i64 } else { 0_i64 },
                    changes.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The rule repository could not update rule {}.",
                        changes.rule_id
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested rule {} does not exist in rule set {}.",
                changes.rule_id, changes.rule_set_id
            )));
        }

        touch_rule_set(&transaction, &changes.rule_set_id, changes.updated_at)?;
        let updated_rule = load_rule(&transaction, &changes.rule_set_id, &changes.rule_id)?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The rule repository could not commit the rule update transaction.",
                error,
            )
        })?;

        Ok(updated_rule)
    }

    pub fn load_overview(
        &mut self,
        rule_set_id: &str,
    ) -> Result<RuleSetRulesOverview, PersistenceError> {
        Ok(RuleSetRulesOverview {
            rule_set_id: rule_set_id.to_owned(),
            rules: self.list_by_rule_set(rule_set_id)?,
        })
    }
}

fn touch_rule_set(
    connection: &Connection,
    rule_set_id: &str,
    updated_at: i64,
) -> Result<(), PersistenceError> {
    connection
        .execute(
            "UPDATE rule_sets SET updated_at = ?2 WHERE id = ?1",
            params![rule_set_id, updated_at],
        )
        .map_err(|error| {
            PersistenceError::with_details(
                format!("The rule repository could not touch rule set {rule_set_id}."),
                error,
            )
        })?;

    Ok(())
}

fn load_rule(
    connection: &Connection,
    rule_set_id: &str,
    rule_id: &str,
) -> Result<RuleSummary, PersistenceError> {
    connection
        .query_row(
            r#"
            SELECT
              id,
              rule_set_id,
              action_scope,
              rule_type,
              severity,
              name,
              description,
              guidance,
              is_enabled,
              created_at,
              updated_at
            FROM rules
            WHERE rule_set_id = ?1 AND id = ?2
            "#,
            params![rule_set_id, rule_id],
            map_rule_summary,
        )
        .map_err(|error| {
            PersistenceError::with_details(
                format!("The rule repository could not reload rule {rule_id}."),
                error,
            )
        })
}

fn upsert_active_rule_set(
    connection: &Connection,
    rule_set_id: &str,
    timestamp: i64,
) -> Result<(), PersistenceError> {
    connection
        .execute(
            r#"
            INSERT INTO app_metadata (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
              value = excluded.value,
              updated_at = excluded.updated_at
            "#,
            params![ACTIVE_RULE_SET_METADATA_KEY, rule_set_id, timestamp],
        )
        .map_err(|error| {
            PersistenceError::with_details(
                "The rule-set repository could not persist the active rule-set selection.",
                error,
            )
        })?;

    Ok(())
}

fn map_rule_set_summary_from_new(new_rule_set: &NewRuleSet) -> RuleSetSummary {
    RuleSetSummary {
        id: new_rule_set.id.clone(),
        name: new_rule_set.name.clone(),
        description: new_rule_set.description.clone(),
        status: new_rule_set.status.clone(),
        created_at: new_rule_set.created_at,
        updated_at: new_rule_set.updated_at,
        last_opened_at: new_rule_set.last_opened_at,
    }
}

fn map_rule_set_summary(row: &Row<'_>) -> rusqlite::Result<RuleSetSummary> {
    Ok(RuleSetSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        last_opened_at: row.get(6)?,
    })
}

fn map_rule_summary(row: &Row<'_>) -> rusqlite::Result<RuleSummary> {
    Ok(RuleSummary {
        id: row.get(0)?,
        rule_set_id: row.get(1)?,
        action_scope: row.get(2)?,
        rule_type: row.get(3)?,
        severity: row.get(4)?,
        name: row.get(5)?,
        description: row.get(6)?,
        guidance: row.get(7)?,
        is_enabled: row.get::<_, i64>(8)? == 1,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{RuleRepository, RuleSetRepository};
    use tempfile::tempdir;

    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::rule_sets::{
        NewRule, NewRuleSet, RuleChanges, RuleSetChanges, RULE_ACTION_SCOPE_QA,
        RULE_ACTION_SCOPE_TRANSLATION, RULE_SET_STATUS_ACTIVE, RULE_SET_STATUS_ARCHIVED,
        RULE_SEVERITY_HIGH, RULE_SEVERITY_LOW, RULE_SEVERITY_MEDIUM, RULE_TYPE_CONSISTENCY,
        RULE_TYPE_PREFERENCE, RULE_TYPE_RESTRICTION,
    };

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-d4";

    fn sample_rule_set(now: i64) -> NewRuleSet {
        NewRuleSet {
            id: "rset_test_001".to_owned(),
            name: "Medical safeguards".to_owned(),
            description: Some("Reusable editorial restrictions for regulated copy.".to_owned()),
            status: RULE_SET_STATUS_ACTIVE.to_owned(),
            created_at: now,
            updated_at: now,
            last_opened_at: now,
        }
    }

    fn sample_rule(now: i64) -> NewRule {
        NewRule {
            id: "rul_test_001".to_owned(),
            rule_set_id: "rset_test_001".to_owned(),
            action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
            rule_type: RULE_TYPE_RESTRICTION.to_owned(),
            severity: RULE_SEVERITY_HIGH.to_owned(),
            name: "Do not soften contraindications".to_owned(),
            description: Some("Keep hard safety warnings explicit.".to_owned()),
            guidance: "Never replace contraindication language with milder wording.".to_owned(),
            is_enabled: true,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn create_and_list_rule_sets_round_trip() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");
        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        let mut repository = RuleSetRepository::new(&mut connection);

        repository
            .create(&sample_rule_set(1_775_488_400))
            .expect("rule set should be created");

        let overview = repository
            .load_overview()
            .expect("rule-set overview should load");

        assert_eq!(
            overview.active_rule_set_id.as_deref(),
            Some("rset_test_001")
        );
        assert_eq!(overview.rule_sets.len(), 1);
        assert_eq!(overview.rule_sets[0].name, "Medical safeguards");
    }

    #[test]
    fn rule_and_rule_set_updates_survive_reopen() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");
        {
            let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
                .expect("database connection should open");
            let mut rule_set_repository = RuleSetRepository::new(&mut connection);

            rule_set_repository
                .create(&sample_rule_set(1_775_488_500))
                .expect("rule set should be created");
            rule_set_repository
                .update(&RuleSetChanges {
                    rule_set_id: "rset_test_001".to_owned(),
                    name: "Medical safeguards revised".to_owned(),
                    description: Some("Updated guidance baseline.".to_owned()),
                    status: RULE_SET_STATUS_ARCHIVED.to_owned(),
                    updated_at: 1_775_488_560,
                })
                .expect("rule set should update");
            let mut rule_repository = RuleRepository::new(&mut connection);

            rule_repository
                .create(&sample_rule(1_775_488_520))
                .expect("rule should be created");
            rule_repository
                .update(&RuleChanges {
                    rule_id: "rul_test_001".to_owned(),
                    rule_set_id: "rset_test_001".to_owned(),
                    action_scope: RULE_ACTION_SCOPE_QA.to_owned(),
                    rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                    severity: RULE_SEVERITY_MEDIUM.to_owned(),
                    name: "Keep warning terminology stable".to_owned(),
                    description: Some("Updated consistency guidance.".to_owned()),
                    guidance: "Reuse the same warning label throughout the document.".to_owned(),
                    is_enabled: false,
                    updated_at: 1_775_488_580,
                })
                .expect("rule should update");
        }

        let mut reopened_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should reopen");
        let rule_set_overview = RuleSetRepository::new(&mut reopened_connection)
            .load_overview()
            .expect("rule-set overview should reload");
        let rules_overview = RuleRepository::new(&mut reopened_connection)
            .load_overview("rset_test_001")
            .expect("rule overview should reload");

        assert_eq!(rule_set_overview.rule_sets.len(), 1);
        assert_eq!(
            rule_set_overview.rule_sets[0].status,
            RULE_SET_STATUS_ARCHIVED
        );
        assert_eq!(
            rule_set_overview.rule_sets[0].name,
            "Medical safeguards revised"
        );
        assert_eq!(rules_overview.rules.len(), 1);
        assert_eq!(rules_overview.rules[0].action_scope, RULE_ACTION_SCOPE_QA);
        assert_eq!(rules_overview.rules[0].rule_type, RULE_TYPE_CONSISTENCY);
        assert_eq!(rules_overview.rules[0].severity, RULE_SEVERITY_MEDIUM);
        assert_eq!(
            rules_overview.rules[0].name,
            "Keep warning terminology stable"
        );
        assert!(!rules_overview.rules[0].is_enabled);
    }

    #[test]
    fn rules_are_listed_in_enabled_then_severity_order() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");
        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        RuleSetRepository::new(&mut connection)
            .create(&sample_rule_set(1_775_488_600))
            .expect("rule set should be created");
        let mut repository = RuleRepository::new(&mut connection);

        repository
            .create(&sample_rule(1_775_488_610))
            .expect("high severity rule should be created");
        repository
            .create(&NewRule {
                id: "rul_test_002".to_owned(),
                rule_set_id: "rset_test_001".to_owned(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
                rule_type: RULE_TYPE_PREFERENCE.to_owned(),
                severity: RULE_SEVERITY_LOW.to_owned(),
                name: "Prefer stable label pairs".to_owned(),
                description: None,
                guidance: "Favor the same noun phrase when repeated in headings.".to_owned(),
                is_enabled: true,
                created_at: 1_775_488_611,
                updated_at: 1_775_488_611,
            })
            .expect("low severity rule should be created");
        repository
            .create(&NewRule {
                id: "rul_test_003".to_owned(),
                rule_set_id: "rset_test_001".to_owned(),
                action_scope: RULE_ACTION_SCOPE_QA.to_owned(),
                rule_type: RULE_TYPE_RESTRICTION.to_owned(),
                severity: RULE_SEVERITY_HIGH.to_owned(),
                name: "Disabled hard rule".to_owned(),
                description: None,
                guidance: "Disabled guidance.".to_owned(),
                is_enabled: false,
                created_at: 1_775_488_612,
                updated_at: 1_775_488_612,
            })
            .expect("disabled rule should be created");

        let rules = repository
            .list_by_rule_set("rset_test_001")
            .expect("rules should list");

        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0].id, "rul_test_001");
        assert_eq!(rules[1].id, "rul_test_002");
        assert_eq!(rules[2].id, "rul_test_003");
    }
}
