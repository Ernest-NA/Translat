use serde::{Deserialize, Serialize};

use crate::qa_findings::QaFindingSummary;
use crate::task_runs::TaskRunSummary;

pub const OPERATIONAL_WARNING_SEVERITY_INFO: &str = "info";
pub const OPERATIONAL_WARNING_SEVERITY_WARNING: &str = "warning";
#[allow(dead_code)]
pub const OPERATIONAL_WARNING_SEVERITY_ERROR: &str = "error";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InspectDocumentOperationalStateInput {
    pub project_id: String,
    pub document_id: String,
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InspectJobTraceInput {
    pub project_id: String,
    pub document_id: String,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationalInspectionWarning {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructedOperationalSnapshot {
    pub status: String,
    pub content_source: String,
    pub total_segments: i64,
    pub translated_segments: i64,
    pub fallback_segments: i64,
    pub is_complete: bool,
    pub latest_document_task_run_id: Option<String>,
    pub orphaned_chunk_task_run_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationalExportTrace {
    pub task_run: TaskRunSummary,
    pub file_name: String,
    pub format: String,
    pub exported_at: i64,
    pub reconstructed_status: String,
    pub content_source: String,
    pub is_complete: bool,
    pub total_segments: i64,
    pub translated_segments: i64,
    pub fallback_segments: i64,
    pub source_job_id: Option<String>,
    pub source_task_run_id: Option<String>,
    pub open_finding_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentJobOverview {
    pub job_id: String,
    pub status: String,
    pub last_updated_at: Option<i64>,
    pub total_chunks: i64,
    pub completed_chunks: i64,
    pub failed_chunks: i64,
    pub cancelled_chunks: i64,
    pub finding_count: i64,
    pub open_finding_count: i64,
    pub latest_error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JobTraceInspection {
    pub overview: DocumentJobOverview,
    pub task_runs: Vec<TaskRunSummary>,
    pub findings: Vec<QaFindingSummary>,
    pub related_exports: Vec<OperationalExportTrace>,
    pub warnings: Vec<OperationalInspectionWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentOperationalState {
    pub project_id: String,
    pub document_id: String,
    pub document_name: String,
    pub document_status: String,
    pub observed_at: i64,
    pub selected_job_id: Option<String>,
    pub recent_runs: Vec<TaskRunSummary>,
    pub jobs: Vec<DocumentJobOverview>,
    pub selected_job: Option<JobTraceInspection>,
    pub findings: Vec<QaFindingSummary>,
    pub open_finding_count: i64,
    pub reconstruction: ReconstructedOperationalSnapshot,
    pub exports: Vec<OperationalExportTrace>,
    pub warnings: Vec<OperationalInspectionWarning>,
}
