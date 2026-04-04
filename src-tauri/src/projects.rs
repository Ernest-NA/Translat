use serde::{Deserialize, Serialize};

pub const ACTIVE_PROJECT_METADATA_KEY: &str = "workspace.active_project_id";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_glossary_id: Option<String>,
    pub default_style_profile_id: Option<String>,
    pub default_rule_set_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsOverview {
    pub active_project_id: Option<String>,
    pub projects: Vec<ProjectSummary>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectEditorialDefaultsInput {
    pub project_id: String,
    pub default_glossary_id: Option<String>,
    pub default_style_profile_id: Option<String>,
    pub default_rule_set_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProject {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectEditorialDefaultsChanges {
    pub project_id: String,
    pub default_glossary_id: Option<String>,
    pub default_style_profile_id: Option<String>,
    pub default_rule_set_id: Option<String>,
    pub updated_at: i64,
}
