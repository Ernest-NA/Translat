use serde::{Deserialize, Serialize};

use crate::sections::DocumentSectionSummary;
use crate::task_runs::TaskRunSummary;

pub const RECONSTRUCTED_DOCUMENT_STATUS_EMPTY: &str = "empty";
pub const RECONSTRUCTED_DOCUMENT_STATUS_UNTRANSLATED: &str = "untranslated";
pub const RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL: &str = "partial";
pub const RECONSTRUCTED_DOCUMENT_STATUS_COMPLETE: &str = "complete";

pub const RECONSTRUCTED_CONTENT_SOURCE_NONE: &str = "none";
pub const RECONSTRUCTED_CONTENT_SOURCE_TARGET: &str = "target";
pub const RECONSTRUCTED_CONTENT_SOURCE_SOURCE_FALLBACK: &str = "source_fallback";
pub const RECONSTRUCTED_CONTENT_SOURCE_MIXED: &str = "mixed";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GetReconstructedDocumentInput {
    pub project_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructedDocument {
    pub project_id: String,
    pub document_id: String,
    pub status: String,
    pub content_source: String,
    pub final_text: Option<String>,
    pub resolved_text: String,
    pub completeness: ReconstructedDocumentCompleteness,
    pub sections: Vec<ReconstructedDocumentSection>,
    pub blocks: Vec<ReconstructedDocumentBlock>,
    pub trace: ReconstructedDocumentTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructedDocumentCompleteness {
    pub total_segments: i64,
    pub translated_segments: i64,
    pub untranslated_segments: i64,
    pub fallback_segments: i64,
    pub total_sections: i64,
    pub total_blocks: i64,
    pub is_complete: bool,
    pub has_translated_content: bool,
    pub has_reconstructible_content: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructedDocumentSection {
    #[serde(flatten)]
    pub section: DocumentSectionSummary,
    pub status: String,
    pub content_source: String,
    pub translated_segment_count: i64,
    pub untranslated_segment_count: i64,
    pub fallback_segment_count: i64,
    pub block_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructedDocumentBlock {
    pub id: String,
    pub section_id: Option<String>,
    pub title: Option<String>,
    pub sequence: i64,
    pub kind: String,
    pub level: Option<i64>,
    pub start_segment_sequence: i64,
    pub end_segment_sequence: i64,
    pub segment_count: i64,
    pub translated_segment_count: i64,
    pub untranslated_segment_count: i64,
    pub fallback_segment_count: i64,
    pub status: String,
    pub content_source: String,
    pub final_text: Option<String>,
    pub resolved_text: String,
    pub segment_ids: Vec<String>,
    pub primary_chunk_ids: Vec<String>,
    pub segments: Vec<ReconstructedSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructedSegment {
    pub id: String,
    pub sequence: i64,
    pub source_text: String,
    pub final_text: Option<String>,
    pub resolved_text: String,
    pub resolved_from: String,
    pub status: String,
    pub primary_chunk_id: Option<String>,
    pub related_chunk_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructedDocumentTrace {
    pub chunk_count: i64,
    pub task_run_count: i64,
    pub document_task_run_ids: Vec<String>,
    pub latest_document_task_run: Option<TaskRunSummary>,
    pub chunks: Vec<ReconstructedDocumentChunkTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReconstructedDocumentChunkTrace {
    pub chunk_id: String,
    pub chunk_sequence: i64,
    pub start_segment_sequence: i64,
    pub end_segment_sequence: i64,
    pub core_segment_ids: Vec<String>,
    pub context_before_segment_ids: Vec<String>,
    pub context_after_segment_ids: Vec<String>,
    pub task_run_ids: Vec<String>,
    pub latest_task_run: Option<TaskRunSummary>,
}
