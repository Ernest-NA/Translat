use serde::{Deserialize, Serialize};

use crate::task_runs::TaskRunSummary;

pub const TRANSLATE_DOCUMENT_ACTION_TYPE: &str = "translate_document";
pub const TRANSLATE_DOCUMENT_ACTION_VERSION: &str = "tr17-translate-document-v1";
pub const TRANSLATE_DOCUMENT_STATUS_COMPLETED: &str = "completed";
pub const TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS: &str = "completed_with_errors";
pub const TRANSLATE_DOCUMENT_STATUS_FAILED: &str = "failed";
pub const TRANSLATE_DOCUMENT_CHUNK_STATUS_COMPLETED: &str = "completed";
pub const TRANSLATE_DOCUMENT_CHUNK_STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateDocumentInput {
    pub project_id: String,
    pub document_id: String,
    pub job_id: Option<String>,
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
