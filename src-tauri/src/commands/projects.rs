use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::glossaries::GlossaryRepository;
use crate::persistence::projects::ProjectRepository;
use crate::persistence::rule_sets::RuleSetRepository;
use crate::persistence::style_profiles::StyleProfileRepository;
use crate::projects::{
    CreateProjectInput, NewProject, ProjectEditorialDefaultsChanges, ProjectSummary,
    ProjectsOverview, UpdateProjectEditorialDefaultsInput,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectInput {
    pub project_id: String,
}

#[tauri::command]
pub fn list_projects(
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<ProjectsOverview, DesktopCommandError> {
    list_projects_with_runtime(database_runtime.inner())
}

fn list_projects_with_runtime(
    database_runtime: &DatabaseRuntime,
) -> Result<ProjectsOverview, DesktopCommandError> {
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for project listing.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = ProjectRepository::new(&mut connection);

    repository.load_overview().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not load the persisted projects.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn create_project(
    input: CreateProjectInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<ProjectSummary, DesktopCommandError> {
    create_project_with_runtime(input, database_runtime.inner())
}

pub(crate) fn create_project_with_runtime(
    input: CreateProjectInput,
    database_runtime: &DatabaseRuntime,
) -> Result<ProjectSummary, DesktopCommandError> {
    let new_project = validate_new_project(input)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for project creation.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = ProjectRepository::new(&mut connection);

    repository.create(&new_project).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not create the requested project.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn open_project(
    input: OpenProjectInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<ProjectSummary, DesktopCommandError> {
    open_project_with_runtime(input, database_runtime.inner())
}

pub(crate) fn open_project_with_runtime(
    input: OpenProjectInput,
    database_runtime: &DatabaseRuntime,
) -> Result<ProjectSummary, DesktopCommandError> {
    let project_id = validate_project_id(&input.project_id)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for project selection.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = ProjectRepository::new(&mut connection);

    repository
        .open_project(&project_id, current_timestamp()?)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not open the requested project.",
                Some(error.to_string()),
            )
        })
}

#[tauri::command]
pub fn update_project_editorial_defaults(
    input: UpdateProjectEditorialDefaultsInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<ProjectSummary, DesktopCommandError> {
    update_project_editorial_defaults_with_runtime(input, database_runtime.inner())
}

fn update_project_editorial_defaults_with_runtime(
    input: UpdateProjectEditorialDefaultsInput,
    database_runtime: &DatabaseRuntime,
) -> Result<ProjectSummary, DesktopCommandError> {
    let updated_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for project default updates.",
            Some(error.to_string()),
        )
    })?;
    let changes = validate_project_editorial_defaults_changes(input, &mut connection, updated_at)?;
    let mut repository = ProjectRepository::new(&mut connection);

    repository.update_editorial_defaults(&changes).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not update the project's editorial defaults.",
            Some(error.to_string()),
        )
    })
}

fn validate_new_project(input: CreateProjectInput) -> Result<NewProject, DesktopCommandError> {
    let trimmed_name = input.name.trim();

    if trimmed_name.is_empty() {
        return Err(DesktopCommandError::validation(
            "The project name is required.",
            None,
        ));
    }

    if trimmed_name.chars().count() > 120 {
        return Err(DesktopCommandError::validation(
            "The project name must stay within 120 characters.",
            None,
        ));
    }

    let trimmed_description = normalize_optional_text(
        input.description,
        1000,
        "The project description must stay within 1000 characters.",
    )?;
    let timestamp = current_timestamp()?;

    Ok(NewProject {
        id: generate_project_id(timestamp),
        name: trimmed_name.to_owned(),
        description: trimmed_description,
        created_at: timestamp,
        updated_at: timestamp,
        last_opened_at: timestamp,
    })
}

fn validate_project_editorial_defaults_changes(
    input: UpdateProjectEditorialDefaultsInput,
    connection: &mut rusqlite::Connection,
    updated_at: i64,
) -> Result<ProjectEditorialDefaultsChanges, DesktopCommandError> {
    let project_id = validate_project_id(&input.project_id)?;

    validate_project_exists(&project_id, connection)?;

    Ok(ProjectEditorialDefaultsChanges {
        project_id,
        default_glossary_id: normalize_default_glossary_id(input.default_glossary_id, connection)?,
        default_style_profile_id: normalize_default_style_profile_id(
            input.default_style_profile_id,
            connection,
        )?,
        default_rule_set_id: normalize_default_rule_set_id(input.default_rule_set_id, connection)?,
        updated_at,
    })
}

fn validate_project_id(project_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_project_id = project_id.trim();

    if trimmed_project_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The project selection is missing a valid project id.",
            None,
        ));
    }

    validate_safe_identifier(
        trimmed_project_id,
        "The project selection requires a safe persisted project id.",
    )?;

    Ok(trimmed_project_id.to_owned())
}

fn validate_project_exists(
    project_id: &str,
    connection: &mut rusqlite::Connection,
) -> Result<(), DesktopCommandError> {
    let mut repository = ProjectRepository::new(connection);
    let project_exists = repository.exists(project_id).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not verify the selected project.",
            Some(error.to_string()),
        )
    })?;

    if !project_exists {
        return Err(DesktopCommandError::validation(
            "The selected project does not exist.",
            None,
        ));
    }

    Ok(())
}

fn normalize_default_glossary_id(
    glossary_id: Option<String>,
    connection: &mut rusqlite::Connection,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_glossary_id = normalize_optional_identifier(glossary_id)?;

    if let Some(glossary_id) = &normalized_glossary_id {
        let mut repository = GlossaryRepository::new(connection);
        let glossary_exists = repository.exists(glossary_id).map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not verify the selected default glossary.",
                Some(error.to_string()),
            )
        })?;

        if !glossary_exists {
            return Err(DesktopCommandError::validation(
                "The selected default glossary does not exist.",
                None,
            ));
        }
    }

    Ok(normalized_glossary_id)
}

fn normalize_default_style_profile_id(
    style_profile_id: Option<String>,
    connection: &mut rusqlite::Connection,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_style_profile_id = normalize_optional_identifier(style_profile_id)?;

    if let Some(style_profile_id) = &normalized_style_profile_id {
        let mut repository = StyleProfileRepository::new(connection);
        let style_profile_exists = repository.exists(style_profile_id).map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not verify the selected default style profile.",
                Some(error.to_string()),
            )
        })?;

        if !style_profile_exists {
            return Err(DesktopCommandError::validation(
                "The selected default style profile does not exist.",
                None,
            ));
        }
    }

    Ok(normalized_style_profile_id)
}

fn normalize_default_rule_set_id(
    rule_set_id: Option<String>,
    connection: &mut rusqlite::Connection,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_rule_set_id = normalize_optional_identifier(rule_set_id)?;

    if let Some(rule_set_id) = &normalized_rule_set_id {
        let mut repository = RuleSetRepository::new(connection);
        let rule_set_exists = repository.exists(rule_set_id).map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not verify the selected default rule set.",
                Some(error.to_string()),
            )
        })?;

        if !rule_set_exists {
            return Err(DesktopCommandError::validation(
                "The selected default rule set does not exist.",
                None,
            ));
        }
    }

    Ok(normalized_rule_set_id)
}

fn normalize_optional_identifier(
    value: Option<String>,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_value = value
        .as_deref()
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(ToOwned::to_owned);

    if let Some(identifier) = &normalized_value {
        validate_safe_identifier(
            identifier,
            "Editorial default references require safe persisted ids.",
        )?;
    }

    Ok(normalized_value)
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

fn generate_project_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "prj_{}_{}",
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
                    "The desktop shell could not compute the current project timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid project timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::params;
    use tempfile::tempdir;

    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::projects::{CreateProjectInput, UpdateProjectEditorialDefaultsInput};

    use super::{
        create_project_with_runtime, list_projects_with_runtime, open_project_with_runtime,
        update_project_editorial_defaults_with_runtime, OpenProjectInput,
    };

    #[test]
    fn create_open_and_update_project_editorial_defaults_end_to_end() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let created_project = create_project_with_runtime(
            CreateProjectInput {
                name: "Clinical D5".to_owned(),
                description: Some("Project with editorial defaults.".to_owned()),
            },
            &runtime,
        )
        .expect("project should be created");

        let connection = runtime
            .open_connection()
            .expect("database connection should open");
        connection
            .execute(
                r#"
                INSERT INTO glossaries (
                  id,
                  name,
                  description,
                  project_id,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                )
                VALUES (?1, ?2, NULL, ?3, 'active', ?4, ?4, ?4)
                "#,
                params![
                    "gls_default_001",
                    "Clinical glossary",
                    created_project.id,
                    created_project.created_at
                ],
            )
            .expect("glossary should be inserted");
        connection
            .execute(
                r#"
                INSERT INTO style_profiles (
                  id,
                  name,
                  description,
                  tone,
                  formality,
                  treatment_preference,
                  consistency_instructions,
                  editorial_notes,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                )
                VALUES (?1, ?2, NULL, 'technical', 'formal', 'usted', NULL, NULL, 'active', ?3, ?3, ?3)
                "#,
                params!["stp_default_001", "Clinical style", created_project.created_at],
            )
            .expect("style profile should be inserted");
        connection
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
                VALUES (?1, ?2, NULL, 'active', ?3, ?3, ?3)
                "#,
                params!["rset_default_001", "Clinical rules", created_project.created_at],
            )
            .expect("rule set should be inserted");
        drop(connection);

        let opened_project = open_project_with_runtime(
            OpenProjectInput {
                project_id: created_project.id.clone(),
            },
            &runtime,
        )
        .expect("project should open");

        let updated_project = update_project_editorial_defaults_with_runtime(
            UpdateProjectEditorialDefaultsInput {
                project_id: created_project.id.clone(),
                default_glossary_id: Some("gls_default_001".to_owned()),
                default_style_profile_id: Some("stp_default_001".to_owned()),
                default_rule_set_id: Some("rset_default_001".to_owned()),
            },
            &runtime,
        )
        .expect("editorial defaults should update");

        let overview =
            list_projects_with_runtime(&runtime).expect("project overview should load");

        assert_eq!(opened_project.id, created_project.id);
        assert_eq!(
            updated_project.default_glossary_id.as_deref(),
            Some("gls_default_001")
        );
        assert_eq!(
            updated_project.default_style_profile_id.as_deref(),
            Some("stp_default_001")
        );
        assert_eq!(
            updated_project.default_rule_set_id.as_deref(),
            Some("rset_default_001")
        );
        assert_eq!(overview.projects.len(), 1);
        assert_eq!(
            overview.projects[0].default_rule_set_id.as_deref(),
            Some("rset_default_001")
        );
    }
}
