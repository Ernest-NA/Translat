use serde::{Deserialize, Serialize};

use crate::qa_findings::QaFindingSummary;

pub const DOCUMENT_QA_FINDING_TYPE_SOURCE_FALLBACK_SEGMENT: &str = "source_fallback_segment";
pub const DOCUMENT_QA_FINDING_TYPE_PARTIAL_BLOCK_TRANSLATION: &str = "partial_block_translation";
pub const DOCUMENT_QA_FINDING_TYPE_NEIGHBOR_CHUNK_TRANSLATION_DRIFT: &str =
    "neighbor_chunk_translation_drift";
pub const DOCUMENT_QA_FINDING_TYPE_CHUNK_EXECUTION_ERROR: &str = "chunk_execution_error";
pub const DOCUMENT_QA_FINDING_TYPE_ORPHANED_CHUNK_TASK_RUN: &str = "orphaned_chunk_task_run";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunDocumentConsistencyQaInput {
    pub project_id: String,
    pub document_id: String,
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDocumentQaFindingsInput {
    pub project_id: String,
    pub document_id: String,
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentConsistencyQaResult {
    pub project_id: String,
    pub document_id: String,
    pub job_id: Option<String>,
    pub reconstructed_status: String,
    pub reconstructed_content_source: String,
    pub generated_findings: Vec<QaFindingSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentQaFindingsOverview {
    pub project_id: String,
    pub document_id: String,
    pub job_id: Option<String>,
    pub findings: Vec<QaFindingSummary>,
}
