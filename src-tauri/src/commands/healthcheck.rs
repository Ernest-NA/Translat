use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::State;

use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::{DatabaseRuntime, DatabaseStatus};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckResponse {
    pub app_name: String,
    pub checked_at: u64,
    pub database: DatabaseStatus,
    pub environment: String,
    pub message: String,
    pub status: String,
    pub version: String,
}

#[tauri::command]
pub fn healthcheck(
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<HealthcheckResponse, DesktopCommandError> {
    let database_status = database_runtime.inspect().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not inspect the encrypted SQLite bootstrap state.",
            Some(error.to_string()),
        )
    })?;

    build_healthcheck_response(database_status)
}

pub fn build_healthcheck_response(
    database: DatabaseStatus,
) -> Result<HealthcheckResponse, DesktopCommandError> {
    let checked_at = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not compute the current timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_millis(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid timestamp size.",
            Some(error.to_string()),
        )
    })?;

    Ok(HealthcheckResponse {
        app_name: "Translat".to_owned(),
        checked_at,
        database,
        environment: app_environment().to_owned(),
        message: "Translat desktop shell is stable, and the encrypted SQLite bootstrap is ready for repositories and migrations.".to_owned(),
        status: "ok".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
}

fn app_environment() -> &'static str {
    if cfg!(debug_assertions) {
        "development"
    } else {
        "production"
    }
}
