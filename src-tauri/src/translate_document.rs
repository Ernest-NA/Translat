use serde::{Deserialize, Serialize};

use crate::task_runs::TaskRunSummary;

pub const TRANSLATE_DOCUMENT_ACTION_TYPE: &str = "translate_document";
pub const TRANSLATE_DOCUMENT_ACTION_VERSION: &str = "tr17-translate-document-v1";
pub const TRANSLATE_DOCUMENT_STATUS_PENDING: &str = "pending";
pub const TRANSLATE_DOCUMENT_STATUS_RUNNING: &str = "running";
pub const TRANSLATE_DOCUMENT_STATUS_COMPLETED: &str = "completed";
pub const TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS: &str = "completed_with_errors";
pub const TRANSLATE_DOCUMENT_STATUS_FAILED: &str = "failed";
pub const TRANSLATE_DOCUMENT_STATUS_CANCELLED: &str = "cancelled";
pub const TRANSLATE_DOCUMENT_CHUNK_STATUS_PENDING: &str = "pending";
pub const TRANSLATE_DOCUMENT_CHUNK_STATUS_RUNNING: &str = "running";
pub const TRANSLATE_DOCUMENT_CHUNK_STATUS_COMPLETED: &str = "completed";
pub const TRANSLATE_DOCUMENT_CHUNK_STATUS_FAILED: &str = "failed";
pub const TRANSLATE_DOCUMENT_CHUNK_STATUS_CANCELLED: &str = "cancelled";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateDocumentInput {
    pub project_id: String,
    pub document_id: String,
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateDocumentJobInput {
    pub project_id: String,
    pub document_id: String,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateDocumentChunkResult {
    pub chunk_id: String,
    pub chunk_sequence: i64,
    pub status: String,
    pub task_run: Option<TaskRunSummary>,
    pub translated_segment_count: i64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateDocumentJobStatus {
    pub project_id: String,
    pub document_id: String,
    pub job_id: String,
    pub status: String,
    pub total_chunks: i64,
    pub pending_chunks: i64,
    pub running_chunks: i64,
    pub completed_chunks: i64,
    pub failed_chunks: i64,
    pub cancelled_chunks: i64,
    pub current_chunk_id: Option<String>,
    pub current_chunk_sequence: Option<i64>,
    pub last_completed_chunk_id: Option<String>,
    pub last_completed_chunk_sequence: Option<i64>,
    pub last_updated_at: Option<i64>,
    pub latest_document_task_run: Option<TaskRunSummary>,
    pub chunk_statuses: Vec<TranslateDocumentChunkResult>,
    pub task_runs: Vec<TaskRunSummary>,
    pub error_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateDocumentResult {
    pub project_id: String,
    pub document_id: String,
    pub job_id: String,
    pub status: String,
    pub action_version: String,
    pub task_run: TaskRunSummary,
    pub total_chunks: i64,
    pub completed_chunks: i64,
    pub failed_chunks: i64,
    pub chunk_results: Vec<TranslateDocumentChunkResult>,
    pub error_messages: Vec<String>,
}
