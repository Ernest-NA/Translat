use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::error::DesktopCommandError;
use crate::glossaries::{
    CreateGlossaryInput, GlossariesOverview, GlossaryChanges, GlossarySummary, NewGlossary,
    UpdateGlossaryInput, GLOSSARY_STATUS_ACTIVE, GLOSSARY_STATUS_ARCHIVED,
};
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::glossaries::GlossaryRepository;
use crate::persistence::projects::ProjectRepository;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenGlossaryInput {
    pub glossary_id: String,
}

#[tauri::command]
pub fn list_glossaries(
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<GlossariesOverview, DesktopCommandError> {
    list_glossaries_with_runtime(database_runtime.inner())
}

fn list_glossaries_with_runtime(
    database_runtime: &DatabaseRuntime,
) -> Result<GlossariesOverview, DesktopCommandError> {
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for glossary listing.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = GlossaryRepository::new(&mut connection);

    repository.load_overview().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not load the persisted glossaries.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn create_glossary(
    input: CreateGlossaryInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<GlossarySummary, DesktopCommandError> {
    create_glossary_with_runtime(input, database_runtime.inner())
}

fn create_glossary_with_runtime(
    input: CreateGlossaryInput,
    database_runtime: &DatabaseRuntime,
) -> Result<GlossarySummary, DesktopCommandError> {
    let timestamp = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for glossary creation.",
            Some(error.to_string()),
        )
    })?;
    let new_glossary = validate_new_glossary(input, &mut connection, timestamp)?;
    let mut repository = GlossaryRepository::new(&mut connection);

    repository.create(&new_glossary).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not create the requested glossary.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn open_glossary(
    input: OpenGlossaryInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<GlossarySummary, DesktopCommandError> {
    open_glossary_with_runtime(input, database_runtime.inner())
}

fn open_glossary_with_runtime(
    input: OpenGlossaryInput,
    database_runtime: &DatabaseRuntime,
) -> Result<GlossarySummary, DesktopCommandError> {
    let glossary_id = validate_glossary_id(&input.glossary_id)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for glossary selection.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = GlossaryRepository::new(&mut connection);

    repository
        .open_glossary(&glossary_id, current_timestamp()?)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not open the requested glossary.",
                Some(error.to_string()),
            )
        })
}

#[tauri::command]
pub fn update_glossary(
    input: UpdateGlossaryInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<GlossarySummary, DesktopCommandError> {
    update_glossary_with_runtime(input, database_runtime.inner())
}

fn update_glossary_with_runtime(
    input: UpdateGlossaryInput,
    database_runtime: &DatabaseRuntime,
) -> Result<GlossarySummary, DesktopCommandError> {
    let updated_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for glossary updates.",
            Some(error.to_string()),
        )
    })?;
    let changes = validate_glossary_changes(input, &mut connection, updated_at)?;
    let mut repository = GlossaryRepository::new(&mut connection);

    repository.update(&changes).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not update the requested glossary.",
            Some(error.to_string()),
        )
    })
}

fn validate_new_glossary(
    input: CreateGlossaryInput,
    connection: &mut rusqlite::Connection,
    timestamp: i64,
) -> Result<NewGlossary, DesktopCommandError> {
    Ok(NewGlossary {
        id: generate_glossary_id(timestamp),
        name: validate_glossary_name(&input.name)?,
        description: normalize_description(input.description)?,
        project_id: normalize_project_id(input.project_id, connection)?,
        status: GLOSSARY_STATUS_ACTIVE.to_owned(),
        created_at: timestamp,
        updated_at: timestamp,
        last_opened_at: timestamp,
    })
}

fn validate_glossary_changes(
    input: UpdateGlossaryInput,
    connection: &mut rusqlite::Connection,
    updated_at: i64,
) -> Result<GlossaryChanges, DesktopCommandError> {
    Ok(GlossaryChanges {
        glossary_id: validate_glossary_id(&input.glossary_id)?,
        name: validate_glossary_name(&input.name)?,
        description: normalize_description(input.description)?,
        project_id: normalize_project_id(input.project_id, connection)?,
        status: validate_status(&input.status)?,
        updated_at,
    })
}

fn validate_glossary_name(name: &str) -> Result<String, DesktopCommandError> {
    let trimmed_name = name.trim();

    if trimmed_name.is_empty() {
        return Err(DesktopCommandError::validation(
            "The glossary name is required.",
            None,
        ));
    }

    if trimmed_name.chars().count() > 120 {
        return Err(DesktopCommandError::validation(
            "The glossary name must stay within 120 characters.",
            None,
        ));
    }

    Ok(trimmed_name.to_owned())
}

fn normalize_description(
    description: Option<String>,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_description = description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(description) = &normalized_description {
        if description.chars().count() > 1000 {
            return Err(DesktopCommandError::validation(
                "The glossary description must stay within 1000 characters.",
                None,
            ));
        }
    }

    Ok(normalized_description)
}

fn normalize_project_id(
    project_id: Option<String>,
    connection: &mut rusqlite::Connection,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_project_id = project_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(project_id) = &normalized_project_id {
        validate_safe_identifier(project_id, "The glossary project reference is invalid.")?;

        let project_exists = ProjectRepository::new(connection)
            .exists(project_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not validate the glossary project reference.",
                    Some(error.to_string()),
                )
            })?;

        if !project_exists {
            return Err(DesktopCommandError::validation(
                "The selected glossary project does not exist.",
                None,
            ));
        }
    }

    Ok(normalized_project_id)
}

fn validate_status(status: &str) -> Result<String, DesktopCommandError> {
    let normalized_status = status.trim().to_ascii_lowercase();

    match normalized_status.as_str() {
        GLOSSARY_STATUS_ACTIVE | GLOSSARY_STATUS_ARCHIVED => Ok(normalized_status),
        _ => Err(DesktopCommandError::validation(
            "The glossary status must be active or archived.",
            None,
        )),
    }
}

fn validate_glossary_id(glossary_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_glossary_id = glossary_id.trim();

    if trimmed_glossary_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The glossary selection is missing a valid glossary id.",
            None,
        ));
    }

    validate_safe_identifier(
        trimmed_glossary_id,
        "The glossary selection requires a safe persisted glossary id.",
    )?;

    Ok(trimmed_glossary_id.to_owned())
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

fn generate_glossary_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "gls_{}_{}",
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
                    "The desktop shell could not compute the current glossary timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid glossary timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::glossaries::{CreateGlossaryInput, UpdateGlossaryInput, GLOSSARY_STATUS_ARCHIVED};
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::projects::NewProject;

    use super::{
        create_glossary_with_runtime, list_glossaries_with_runtime, open_glossary_with_runtime,
        update_glossary_with_runtime, OpenGlossaryInput,
    };

    #[test]
    fn create_open_and_update_glossary_end_to_end() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");
        let now = 1_775_315_200_i64;

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: "prj_active_001".to_owned(),
                name: "Active project".to_owned(),
                description: None,
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("project should be created");
        drop(connection);

        let created_glossary = create_glossary_with_runtime(
            CreateGlossaryInput {
                name: "Clinical core".to_owned(),
                description: Some("Base glossary for the project.".to_owned()),
                project_id: Some("prj_active_001".to_owned()),
            },
            &runtime,
        )
        .expect("glossary should be created");

        let opened_glossary = open_glossary_with_runtime(
            OpenGlossaryInput {
                glossary_id: created_glossary.id.clone(),
            },
            &runtime,
        )
        .expect("glossary should open");

        let updated_glossary = update_glossary_with_runtime(
            UpdateGlossaryInput {
                glossary_id: created_glossary.id.clone(),
                name: "Clinical archive".to_owned(),
                description: Some("Archived while D2 lands entries.".to_owned()),
                project_id: None,
                status: GLOSSARY_STATUS_ARCHIVED.to_owned(),
            },
            &runtime,
        )
        .expect("glossary should update");

        let overview =
            list_glossaries_with_runtime(&runtime).expect("glossary overview should load");

        assert_eq!(
            created_glossary.project_id,
            Some("prj_active_001".to_owned())
        );
        assert_eq!(opened_glossary.id, created_glossary.id);
        assert_eq!(updated_glossary.id, created_glossary.id);
        assert_eq!(updated_glossary.project_id, None);
        assert_eq!(updated_glossary.status, GLOSSARY_STATUS_ARCHIVED);
        assert_eq!(
            overview.active_glossary_id,
            Some(created_glossary.id.clone())
        );
        assert_eq!(overview.glossaries.len(), 1);
        assert_eq!(overview.glossaries[0].name, "Clinical archive");
    }
}
