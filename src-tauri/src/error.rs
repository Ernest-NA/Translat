use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCommandError {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    pub message: String,
}

impl DesktopCommandError {
    pub fn internal(message: impl Into<String>, details: Option<String>) -> Self {
        Self {
            code: "DESKTOP_COMMAND_FAILED".to_owned(),
            details,
            message: message.into(),
        }
    }
}
