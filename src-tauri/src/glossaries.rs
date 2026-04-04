use serde::{Deserialize, Serialize};

pub const ACTIVE_GLOSSARY_METADATA_KEY: &str = "workspace.active_glossary_id";
pub const GLOSSARY_STATUS_ACTIVE: &str = "active";
pub const GLOSSARY_STATUS_ARCHIVED: &str = "archived";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlossarySummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlossariesOverview {
    pub active_glossary_id: Option<String>,
    pub glossaries: Vec<GlossarySummary>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateGlossaryInput {
    pub name: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGlossaryInput {
    pub glossary_id: String,
    pub name: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGlossary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryChanges {
    pub glossary_id: String,
    pub name: String,
    pub description: Option<String>,
    pub project_id: Option<String>,
    pub status: String,
    pub updated_at: i64,
}
