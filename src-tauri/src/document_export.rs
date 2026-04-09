use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportReconstructedDocumentInput {
    pub project_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportReconstructedDocumentResult {
    pub project_id: String,
    pub document_id: String,
    pub document_name: String,
    pub format: String,
    pub mime_type: String,
    pub file_name: String,
    pub exported_at: i64,
    pub status: String,
    pub content_source: String,
    pub is_complete: bool,
    pub total_segments: i64,
    pub translated_segments: i64,
    pub fallback_segments: i64,
    pub content: String,
}
