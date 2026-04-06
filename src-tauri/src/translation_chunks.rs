use serde::{Deserialize, Serialize};

pub const TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE: &str = "context_before";
pub const TRANSLATION_CHUNK_SEGMENT_ROLE_CORE: &str = "core";
pub const TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER: &str = "context_after";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslationChunkSummary {
    pub id: String,
    pub document_id: String,
    pub sequence: i64,
    pub builder_version: String,
    pub strategy: String,
    pub source_text: String,
    pub context_before_text: Option<String>,
    pub context_after_text: Option<String>,
    pub start_segment_sequence: i64,
    pub end_segment_sequence: i64,
    pub segment_count: i64,
    pub source_word_count: i64,
    pub source_character_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslationChunkSegmentSummary {
    pub chunk_id: String,
    pub segment_id: String,
    pub segment_sequence: i64,
    pub position: i64,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTranslationChunksOverview {
    pub project_id: String,
    pub document_id: String,
    pub chunks: Vec<TranslationChunkSummary>,
    pub chunk_segments: Vec<TranslationChunkSegmentSummary>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildDocumentTranslationChunksInput {
    pub project_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListDocumentTranslationChunksInput {
    pub project_id: String,
    pub document_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTranslationChunk {
    pub id: String,
    pub document_id: String,
    pub sequence: i64,
    pub builder_version: String,
    pub strategy: String,
    pub source_text: String,
    pub context_before_text: Option<String>,
    pub context_after_text: Option<String>,
    pub start_segment_sequence: i64,
    pub end_segment_sequence: i64,
    pub segment_count: i64,
    pub source_word_count: i64,
    pub source_character_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTranslationChunkSegment {
    pub chunk_id: String,
    pub segment_id: String,
    pub segment_sequence: i64,
    pub position: i64,
    pub role: String,
}
