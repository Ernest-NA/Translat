#![cfg_attr(not(test), allow(dead_code))]

use serde::{Deserialize, Serialize};

pub const CHAPTER_CONTEXT_SCOPE_DOCUMENT: &str = "document";
pub const CHAPTER_CONTEXT_SCOPE_CHAPTER: &str = "chapter";
#[allow(dead_code)]
pub const CHAPTER_CONTEXT_SCOPE_SECTION: &str = "section";
#[allow(dead_code)]
pub const CHAPTER_CONTEXT_SCOPE_RANGE: &str = "range";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChapterContextSummary {
    pub id: String,
    pub document_id: String,
    pub section_id: Option<String>,
    pub task_run_id: Option<String>,
    pub scope_type: String,
    pub start_segment_sequence: i64,
    pub end_segment_sequence: i64,
    pub context_text: String,
    pub source_summary: Option<String>,
    pub context_word_count: i64,
    pub context_character_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChapterContext {
    pub id: String,
    pub document_id: String,
    pub section_id: Option<String>,
    pub task_run_id: Option<String>,
    pub scope_type: String,
    pub start_segment_sequence: i64,
    pub end_segment_sequence: i64,
    pub context_text: String,
    pub source_summary: Option<String>,
    pub context_word_count: i64,
    pub context_character_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
