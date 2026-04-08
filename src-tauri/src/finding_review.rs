use serde::{Deserialize, Serialize};

use crate::qa_findings::QaFindingSummary;
use crate::reconstructed_documents::{ReconstructedDocumentBlock, ReconstructedSegment};
use crate::task_runs::TaskRunSummary;
use crate::translate_chunk::TranslateChunkResult;
use crate::translation_chunks::{TranslationChunkSegmentSummary, TranslationChunkSummary};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InspectQaFindingInput {
    pub project_id: String,
    pub document_id: String,
    pub finding_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RetranslateChunkFromQaFindingInput {
    pub project_id: String,
    pub document_id: String,
    pub finding_id: String,
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QaFindingChunkAnchor {
    pub finding_id: String,
    pub chunk_id: Option<String>,
    pub chunk_sequence: Option<i64>,
    pub resolution_kind: String,
    pub resolution_message: String,
    pub can_retranslate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QaFindingReviewContext {
    pub project_id: String,
    pub document_id: String,
    pub finding: QaFindingSummary,
    pub anchor: QaFindingChunkAnchor,
    pub chunk: Option<TranslationChunkSummary>,
    pub chunk_segments: Vec<TranslationChunkSegmentSummary>,
    pub latest_chunk_task_run: Option<TaskRunSummary>,
    pub latest_document_task_run: Option<TaskRunSummary>,
    pub related_block: Option<ReconstructedDocumentBlock>,
    pub related_segments: Vec<ReconstructedSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QaFindingRetranslationResult {
    pub project_id: String,
    pub document_id: String,
    pub finding: QaFindingSummary,
    pub anchor: QaFindingChunkAnchor,
    pub correction_job_id: String,
    pub review_action_persisted: bool,
    pub review_action_warning: Option<String>,
    pub translate_result: TranslateChunkResult,
}
