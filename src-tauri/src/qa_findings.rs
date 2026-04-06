use serde::{Deserialize, Serialize};

#[allow(dead_code)]
pub const QA_FINDING_SEVERITY_LOW: &str = "low";
pub const QA_FINDING_SEVERITY_MEDIUM: &str = "medium";
pub const QA_FINDING_SEVERITY_HIGH: &str = "high";

pub const QA_FINDING_STATUS_OPEN: &str = "open";
pub const QA_FINDING_STATUS_RESOLVED: &str = "resolved";
#[allow(dead_code)]
pub const QA_FINDING_STATUS_DISMISSED: &str = "dismissed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QaFindingSummary {
    pub id: String,
    pub document_id: String,
    pub chunk_id: Option<String>,
    pub task_run_id: Option<String>,
    pub job_id: Option<String>,
    pub finding_type: String,
    pub severity: String,
    pub status: String,
    pub message: String,
    pub details: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewQaFinding {
    pub id: String,
    pub document_id: String,
    pub chunk_id: Option<String>,
    pub task_run_id: Option<String>,
    pub job_id: Option<String>,
    pub finding_type: String,
    pub severity: String,
    pub status: String,
    pub message: String,
    pub details: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}
