use serde::{Deserialize, Serialize};

pub const DOCUMENT_SECTION_TYPE_DOCUMENT: &str = "document";
pub const DOCUMENT_SECTION_TYPE_CHAPTER: &str = "chapter";
pub const DOCUMENT_SECTION_TYPE_SECTION: &str = "section";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSectionSummary {
    pub id: String,
    pub document_id: String,
    pub sequence: i64,
    pub title: String,
    pub section_type: String,
    pub level: i64,
    pub start_segment_sequence: i64,
    pub end_segment_sequence: i64,
    pub segment_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDocumentSection {
    pub id: String,
    pub document_id: String,
    pub sequence: i64,
    pub title: String,
    pub section_type: String,
    pub level: i64,
    pub start_segment_sequence: i64,
    pub end_segment_sequence: i64,
    pub segment_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
