#![cfg_attr(not(test), allow(dead_code))]

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
pub const TASK_RUN_STATUS_PENDING: &str = "pending";
pub const TASK_RUN_STATUS_RUNNING: &str = "running";
pub const TASK_RUN_STATUS_COMPLETED: &str = "completed";
#[allow(dead_code)]
pub const TASK_RUN_STATUS_FAILED: &str = "failed";
#[allow(dead_code)]
pub const TASK_RUN_STATUS_CANCELLED: &str = "cancelled";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskRunSummary {
    pub id: String,
    pub document_id: String,
    pub chunk_id: Option<String>,
    pub job_id: Option<String>,
    pub action_type: String,
    pub status: String,
    pub input_payload: Option<String>,
    pub output_payload: Option<String>,
    pub error_message: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTaskRun {
    pub id: String,
    pub document_id: String,
    pub chunk_id: Option<String>,
    pub job_id: Option<String>,
    pub action_type: String,
    pub status: String,
    pub input_payload: Option<String>,
    pub output_payload: Option<String>,
    pub error_message: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
