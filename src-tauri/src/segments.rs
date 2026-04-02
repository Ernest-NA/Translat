use serde::{Deserialize, Serialize};

pub const SEGMENT_STATUS_PENDING_TRANSLATION: &str = "pending_translation";

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentSummary {
    pub id: String,
    pub document_id: String,
    pub sequence: i64,
    pub source_text: String,
    pub source_word_count: i64,
    pub source_character_count: i64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDocumentInput {
    pub project_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSegment {
    pub id: String,
    pub document_id: String,
    pub sequence: i64,
    pub source_text: String,
    pub source_word_count: i64,
    pub source_character_count: i64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}
