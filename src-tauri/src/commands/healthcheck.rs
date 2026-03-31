use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::error::DesktopCommandError;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthcheckResponse {
    pub app_name: String,
    pub checked_at: u64,
    pub environment: String,
    pub message: String,
    pub status: String,
    pub version: String,
}

#[tauri::command]
pub fn healthcheck() -> Result<HealthcheckResponse, DesktopCommandError> {
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
        environment: app_environment().to_owned(),
        message: "Translat desktop shell is stable and ready for the next modules.".to_owned(),
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
