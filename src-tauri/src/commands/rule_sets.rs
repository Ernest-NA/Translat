use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::rule_sets::{RuleRepository, RuleSetRepository};
use crate::rule_sets::{
    CreateRuleInput, CreateRuleSetInput, ListRuleSetRulesInput, NewRule, NewRuleSet, RuleChanges,
    RuleSetChanges, RuleSetRulesOverview, RuleSetSummary, RuleSetsOverview, RuleSummary,
    UpdateRuleInput, UpdateRuleSetInput, RULE_ACTION_SCOPE_CONSISTENCY_REVIEW,
    RULE_ACTION_SCOPE_EXPORT, RULE_ACTION_SCOPE_QA, RULE_ACTION_SCOPE_RETRANSLATION,
    RULE_ACTION_SCOPE_TRANSLATION, RULE_SET_STATUS_ACTIVE, RULE_SET_STATUS_ARCHIVED,
    RULE_SEVERITY_HIGH, RULE_SEVERITY_LOW, RULE_SEVERITY_MEDIUM, RULE_TYPE_CONSISTENCY,
    RULE_TYPE_PREFERENCE, RULE_TYPE_RESTRICTION,
};

const MAX_RULE_SET_NAME_LENGTH: usize = 120;
const MAX_RULE_SET_DESCRIPTION_LENGTH: usize = 1000;
const MAX_RULE_NAME_LENGTH: usize = 160;
const MAX_RULE_DESCRIPTION_LENGTH: usize = 1000;
const MAX_RULE_GUIDANCE_LENGTH: usize = 2000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenRuleSetInput {
    pub rule_set_id: String,
}

#[tauri::command]
pub fn list_rule_sets(
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<RuleSetsOverview, DesktopCommandError> {
    list_rule_sets_with_runtime(database_runtime.inner())
}

fn list_rule_sets_with_runtime(
    database_runtime: &DatabaseRuntime,
) -> Result<RuleSetsOverview, DesktopCommandError> {
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for rule-set listing.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = RuleSetRepository::new(&mut connection);

    repository.load_overview().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not load the persisted rule sets.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn create_rule_set(
    input: CreateRuleSetInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<RuleSetSummary, DesktopCommandError> {
    create_rule_set_with_runtime(input, database_runtime.inner())
}

fn create_rule_set_with_runtime(
    input: CreateRuleSetInput,
    database_runtime: &DatabaseRuntime,
) -> Result<RuleSetSummary, DesktopCommandError> {
    let timestamp = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for rule-set creation.",
            Some(error.to_string()),
        )
    })?;
    let new_rule_set = validate_new_rule_set(input, timestamp)?;
    let mut repository = RuleSetRepository::new(&mut connection);

    repository.create(&new_rule_set).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not create the requested rule set.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn open_rule_set(
    input: OpenRuleSetInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<RuleSetSummary, DesktopCommandError> {
    open_rule_set_with_runtime(input, database_runtime.inner())
}

fn open_rule_set_with_runtime(
    input: OpenRuleSetInput,
    database_runtime: &DatabaseRuntime,
) -> Result<RuleSetSummary, DesktopCommandError> {
    let rule_set_id = validate_rule_set_id(&input.rule_set_id)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for rule-set selection.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = RuleSetRepository::new(&mut connection);

    repository
        .open_rule_set(&rule_set_id, current_timestamp()?)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not open the requested rule set.",
                Some(error.to_string()),
            )
        })
}

#[tauri::command]
pub fn update_rule_set(
    input: UpdateRuleSetInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<RuleSetSummary, DesktopCommandError> {
    update_rule_set_with_runtime(input, database_runtime.inner())
}

fn update_rule_set_with_runtime(
    input: UpdateRuleSetInput,
    database_runtime: &DatabaseRuntime,
) -> Result<RuleSetSummary, DesktopCommandError> {
    let updated_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for rule-set updates.",
            Some(error.to_string()),
        )
    })?;
    let changes = validate_rule_set_changes(input, updated_at)?;
    let mut repository = RuleSetRepository::new(&mut connection);

    repository.update(&changes).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not update the requested rule set.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn list_rule_set_rules(
    input: ListRuleSetRulesInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<RuleSetRulesOverview, DesktopCommandError> {
    list_rule_set_rules_with_runtime(input, database_runtime.inner())
}

fn list_rule_set_rules_with_runtime(
    input: ListRuleSetRulesInput,
    database_runtime: &DatabaseRuntime,
) -> Result<RuleSetRulesOverview, DesktopCommandError> {
    let rule_set_id = validate_rule_set_id(&input.rule_set_id)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for rule listing.",
            Some(error.to_string()),
        )
    })?;
    validate_rule_set_exists(&rule_set_id, &mut connection)?;
    let mut repository = RuleRepository::new(&mut connection);

    repository.load_overview(&rule_set_id).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not load the persisted rules for the selected rule set.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn create_rule(
    input: CreateRuleInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<RuleSummary, DesktopCommandError> {
    create_rule_with_runtime(input, database_runtime.inner())
}

fn create_rule_with_runtime(
    input: CreateRuleInput,
    database_runtime: &DatabaseRuntime,
) -> Result<RuleSummary, DesktopCommandError> {
    let timestamp = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for rule creation.",
            Some(error.to_string()),
        )
    })?;
    let new_rule = validate_new_rule(input, &mut connection, timestamp)?;
    let mut repository = RuleRepository::new(&mut connection);

    repository.create(&new_rule).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not create the requested rule.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn update_rule(
    input: UpdateRuleInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<RuleSummary, DesktopCommandError> {
    update_rule_with_runtime(input, database_runtime.inner())
}

fn update_rule_with_runtime(
    input: UpdateRuleInput,
    database_runtime: &DatabaseRuntime,
) -> Result<RuleSummary, DesktopCommandError> {
    let updated_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for rule updates.",
            Some(error.to_string()),
        )
    })?;
    let changes = validate_rule_changes(input, &mut connection, updated_at)?;
    let mut repository = RuleRepository::new(&mut connection);

    repository.update(&changes).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not update the requested rule.",
            Some(error.to_string()),
        )
    })
}

fn validate_new_rule_set(
    input: CreateRuleSetInput,
    timestamp: i64,
) -> Result<NewRuleSet, DesktopCommandError> {
    Ok(NewRuleSet {
        id: generate_rule_set_id(timestamp),
        name: validate_rule_set_name(&input.name)?,
        description: normalize_optional_text(
            input.description,
            MAX_RULE_SET_DESCRIPTION_LENGTH,
            "The rule-set description must stay within 1000 characters.",
        )?,
        status: RULE_SET_STATUS_ACTIVE.to_owned(),
        created_at: timestamp,
        updated_at: timestamp,
        last_opened_at: timestamp,
    })
}

fn validate_rule_set_changes(
    input: UpdateRuleSetInput,
    updated_at: i64,
) -> Result<RuleSetChanges, DesktopCommandError> {
    Ok(RuleSetChanges {
        rule_set_id: validate_rule_set_id(&input.rule_set_id)?,
        name: validate_rule_set_name(&input.name)?,
        description: normalize_optional_text(
            input.description,
            MAX_RULE_SET_DESCRIPTION_LENGTH,
            "The rule-set description must stay within 1000 characters.",
        )?,
        status: validate_rule_set_status(&input.status)?,
        updated_at,
    })
}

fn validate_new_rule(
    input: CreateRuleInput,
    connection: &mut rusqlite::Connection,
    timestamp: i64,
) -> Result<NewRule, DesktopCommandError> {
    let rule_set_id = validate_rule_set_id(&input.rule_set_id)?;
    validate_rule_set_exists(&rule_set_id, connection)?;

    Ok(NewRule {
        id: generate_rule_id(timestamp),
        rule_set_id,
        action_scope: validate_rule_action_scope(&input.action_scope)?,
        rule_type: validate_rule_type(&input.rule_type)?,
        severity: validate_rule_severity(&input.severity)?,
        name: validate_rule_name(&input.name)?,
        description: normalize_optional_text(
            input.description,
            MAX_RULE_DESCRIPTION_LENGTH,
            "The rule description must stay within 1000 characters.",
        )?,
        guidance: validate_rule_guidance(&input.guidance)?,
        is_enabled: input.is_enabled,
        created_at: timestamp,
        updated_at: timestamp,
    })
}

fn validate_rule_changes(
    input: UpdateRuleInput,
    connection: &mut rusqlite::Connection,
    updated_at: i64,
) -> Result<RuleChanges, DesktopCommandError> {
    let rule_set_id = validate_rule_set_id(&input.rule_set_id)?;
    validate_rule_set_exists(&rule_set_id, connection)?;

    Ok(RuleChanges {
        rule_id: validate_rule_id(&input.rule_id)?,
        rule_set_id,
        action_scope: validate_rule_action_scope(&input.action_scope)?,
        rule_type: validate_rule_type(&input.rule_type)?,
        severity: validate_rule_severity(&input.severity)?,
        name: validate_rule_name(&input.name)?,
        description: normalize_optional_text(
            input.description,
            MAX_RULE_DESCRIPTION_LENGTH,
            "The rule description must stay within 1000 characters.",
        )?,
        guidance: validate_rule_guidance(&input.guidance)?,
        is_enabled: input.is_enabled,
        updated_at,
    })
}

fn validate_rule_set_exists(
    rule_set_id: &str,
    connection: &mut rusqlite::Connection,
) -> Result<(), DesktopCommandError> {
    let rule_set_exists = RuleSetRepository::new(connection)
        .exists(rule_set_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not validate the selected rule set.",
                Some(error.to_string()),
            )
        })?;

    if !rule_set_exists {
        return Err(DesktopCommandError::validation(
            "The selected rule set does not exist.",
            None,
        ));
    }

    Ok(())
}

fn validate_rule_set_name(name: &str) -> Result<String, DesktopCommandError> {
    let trimmed_name = name.trim();

    if trimmed_name.is_empty() {
        return Err(DesktopCommandError::validation(
            "The rule-set name is required.",
            None,
        ));
    }

    if trimmed_name.chars().count() > MAX_RULE_SET_NAME_LENGTH {
        return Err(DesktopCommandError::validation(
            "The rule-set name must stay within 120 characters.",
            None,
        ));
    }

    Ok(trimmed_name.to_owned())
}

fn validate_rule_name(name: &str) -> Result<String, DesktopCommandError> {
    let trimmed_name = name.trim();

    if trimmed_name.is_empty() {
        return Err(DesktopCommandError::validation(
            "The rule name is required.",
            None,
        ));
    }

    if trimmed_name.chars().count() > MAX_RULE_NAME_LENGTH {
        return Err(DesktopCommandError::validation(
            "The rule name must stay within 160 characters.",
            None,
        ));
    }

    Ok(trimmed_name.to_owned())
}

fn validate_rule_guidance(guidance: &str) -> Result<String, DesktopCommandError> {
    let trimmed_guidance = guidance.trim();

    if trimmed_guidance.is_empty() {
        return Err(DesktopCommandError::validation(
            "Each rule requires editorial guidance.",
            None,
        ));
    }

    if trimmed_guidance.chars().count() > MAX_RULE_GUIDANCE_LENGTH {
        return Err(DesktopCommandError::validation(
            "The rule guidance must stay within 2000 characters.",
            None,
        ));
    }

    Ok(trimmed_guidance.to_owned())
}

fn validate_rule_set_id(rule_set_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_rule_set_id = rule_set_id.trim();

    if trimmed_rule_set_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The rule-set request is missing a valid rule-set id.",
            None,
        ));
    }

    validate_safe_identifier(
        trimmed_rule_set_id,
        "The rule-set request requires a safe persisted rule-set id.",
    )?;

    Ok(trimmed_rule_set_id.to_owned())
}

fn validate_rule_id(rule_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_rule_id = rule_id.trim();

    if trimmed_rule_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The rule request is missing a valid rule id.",
            None,
        ));
    }

    validate_safe_identifier(
        trimmed_rule_id,
        "The rule request requires a safe persisted rule id.",
    )?;

    Ok(trimmed_rule_id.to_owned())
}

fn validate_rule_set_status(status: &str) -> Result<String, DesktopCommandError> {
    let normalized_status = status.trim().to_ascii_lowercase();

    match normalized_status.as_str() {
        RULE_SET_STATUS_ACTIVE | RULE_SET_STATUS_ARCHIVED => Ok(normalized_status),
        _ => Err(DesktopCommandError::validation(
            "The rule-set status must be active or archived.",
            None,
        )),
    }
}

fn validate_rule_type(rule_type: &str) -> Result<String, DesktopCommandError> {
    let normalized_rule_type = rule_type.trim().to_ascii_lowercase();

    match normalized_rule_type.as_str() {
        RULE_TYPE_CONSISTENCY | RULE_TYPE_PREFERENCE | RULE_TYPE_RESTRICTION => {
            Ok(normalized_rule_type)
        }
        _ => Err(DesktopCommandError::validation(
            "The rule type must be consistency, preference, or restriction.",
            None,
        )),
    }
}

fn validate_rule_action_scope(action_scope: &str) -> Result<String, DesktopCommandError> {
    let normalized_action_scope = action_scope.trim().to_ascii_lowercase();

    match normalized_action_scope.as_str() {
        RULE_ACTION_SCOPE_TRANSLATION
        | RULE_ACTION_SCOPE_RETRANSLATION
        | RULE_ACTION_SCOPE_QA
        | RULE_ACTION_SCOPE_EXPORT
        | RULE_ACTION_SCOPE_CONSISTENCY_REVIEW => Ok(normalized_action_scope),
        _ => Err(DesktopCommandError::validation(
            "The rule action scope must be translation, retranslation, qa, export, or consistency_review.",
            None,
        )),
    }
}

fn validate_rule_severity(severity: &str) -> Result<String, DesktopCommandError> {
    let normalized_severity = severity.trim().to_ascii_lowercase();

    match normalized_severity.as_str() {
        RULE_SEVERITY_LOW | RULE_SEVERITY_MEDIUM | RULE_SEVERITY_HIGH => Ok(normalized_severity),
        _ => Err(DesktopCommandError::validation(
            "The rule severity must be low, medium, or high.",
            None,
        )),
    }
}

fn normalize_optional_text(
    value: Option<String>,
    max_length: usize,
    message: &str,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_value = value
        .as_deref()
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(ToOwned::to_owned);

    if let Some(text) = &normalized_value {
        if text.chars().count() > max_length {
            return Err(DesktopCommandError::validation(message, None));
        }
    }

    Ok(normalized_value)
}

fn validate_safe_identifier(value: &str, message: &str) -> Result<(), DesktopCommandError> {
    if !value
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(message, None));
    }

    Ok(())
}

fn generate_rule_set_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "rset_{}_{}",
        timestamp,
        URL_SAFE_NO_PAD.encode(random_part.to_le_bytes())
    )
}

fn generate_rule_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "rul_{}_{}",
        timestamp,
        URL_SAFE_NO_PAD.encode(random_part.to_le_bytes())
    )
}

fn current_timestamp() -> Result<i64, DesktopCommandError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not compute the current rule timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid rule timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::rule_sets::{
        CreateRuleInput, CreateRuleSetInput, ListRuleSetRulesInput, UpdateRuleInput,
        UpdateRuleSetInput, RULE_ACTION_SCOPE_QA, RULE_ACTION_SCOPE_TRANSLATION,
        RULE_SET_STATUS_ARCHIVED, RULE_SEVERITY_HIGH, RULE_SEVERITY_MEDIUM, RULE_TYPE_CONSISTENCY,
        RULE_TYPE_RESTRICTION,
    };

    use super::{
        create_rule_set_with_runtime, create_rule_with_runtime, list_rule_set_rules_with_runtime,
        list_rule_sets_with_runtime, open_rule_set_with_runtime, update_rule_set_with_runtime,
        update_rule_with_runtime, OpenRuleSetInput,
    };

    #[test]
    fn create_open_and_update_rule_set_end_to_end() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let created_rule_set = create_rule_set_with_runtime(
            CreateRuleSetInput {
                name: "Regulatory warnings".to_owned(),
                description: Some("Rule set for sensitive medical copy.".to_owned()),
            },
            &runtime,
        )
        .expect("rule set should be created");

        let opened_rule_set = open_rule_set_with_runtime(
            OpenRuleSetInput {
                rule_set_id: created_rule_set.id.clone(),
            },
            &runtime,
        )
        .expect("rule set should open");

        let updated_rule_set = update_rule_set_with_runtime(
            UpdateRuleSetInput {
                rule_set_id: created_rule_set.id.clone(),
                name: "Regulatory warnings revised".to_owned(),
                description: Some("Updated rule-set baseline.".to_owned()),
                status: RULE_SET_STATUS_ARCHIVED.to_owned(),
            },
            &runtime,
        )
        .expect("rule set should update");

        let overview = list_rule_sets_with_runtime(&runtime).expect("overview should load");

        assert_eq!(opened_rule_set.id, created_rule_set.id);
        assert_eq!(updated_rule_set.status, RULE_SET_STATUS_ARCHIVED);
        assert_eq!(
            overview.active_rule_set_id.as_deref(),
            Some(created_rule_set.id.as_str())
        );
        assert_eq!(overview.rule_sets.len(), 1);
        assert_eq!(overview.rule_sets[0].name, "Regulatory warnings revised");
    }

    #[test]
    fn create_list_and_update_rules_end_to_end() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let created_rule_set = create_rule_set_with_runtime(
            CreateRuleSetInput {
                name: "Medical guidance".to_owned(),
                description: None,
            },
            &runtime,
        )
        .expect("rule set should be created");

        let created_rule = create_rule_with_runtime(
            CreateRuleInput {
                rule_set_id: created_rule_set.id.clone(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
                rule_type: RULE_TYPE_RESTRICTION.to_owned(),
                severity: RULE_SEVERITY_HIGH.to_owned(),
                name: "Do not euphemize risk".to_owned(),
                description: Some("Keep adverse events explicit.".to_owned()),
                guidance: "Avoid softened alternatives for risk statements.".to_owned(),
                is_enabled: true,
            },
            &runtime,
        )
        .expect("rule should be created");

        let updated_rule = update_rule_with_runtime(
            UpdateRuleInput {
                rule_id: created_rule.id.clone(),
                rule_set_id: created_rule_set.id.clone(),
                action_scope: RULE_ACTION_SCOPE_QA.to_owned(),
                rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                severity: RULE_SEVERITY_MEDIUM.to_owned(),
                name: "Keep warning labels stable".to_owned(),
                description: Some("Prefer the same warning label throughout.".to_owned()),
                guidance: "Reuse the same warning noun phrase across segments.".to_owned(),
                is_enabled: false,
            },
            &runtime,
        )
        .expect("rule should update");

        let overview = list_rule_set_rules_with_runtime(
            ListRuleSetRulesInput {
                rule_set_id: created_rule_set.id.clone(),
            },
            &runtime,
        )
        .expect("rules overview should load");

        assert_eq!(overview.rule_set_id, created_rule_set.id);
        assert_eq!(overview.rules.len(), 1);
        assert_eq!(updated_rule.action_scope, RULE_ACTION_SCOPE_QA);
        assert_eq!(updated_rule.rule_type, RULE_TYPE_CONSISTENCY);
        assert_eq!(updated_rule.severity, RULE_SEVERITY_MEDIUM);
        assert!(!updated_rule.is_enabled);
    }
}
