use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::projects::ProjectRepository;
use crate::projects::{CreateProjectInput, NewProject, ProjectSummary, ProjectsOverview};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectInput {
    pub project_id: String,
}

#[tauri::command]
pub fn list_projects(
    database_runtime: State<'_, DatabaseRuntime>,
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
    let project_id = input.project_id.trim();

    if project_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The project selection is missing a valid project id.",
            None,
        ));
    }

    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for project selection.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = ProjectRepository::new(&mut connection);

    repository
        .open_project(project_id, current_timestamp()?)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not open the requested project.",
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

    let trimmed_description = input
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(description) = &trimmed_description {
        if description.chars().count() > 1000 {
            return Err(DesktopCommandError::validation(
                "The project description must stay within 1000 characters.",
                None,
            ));
        }
    }

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
