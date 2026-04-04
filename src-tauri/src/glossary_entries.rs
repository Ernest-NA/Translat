use serde::{Deserialize, Serialize};

pub const GLOSSARY_ENTRY_STATUS_ACTIVE: &str = "active";
pub const GLOSSARY_ENTRY_STATUS_ARCHIVED: &str = "archived";
pub const GLOSSARY_ENTRY_VARIANT_TYPE_SOURCE: &str = "source";
pub const GLOSSARY_ENTRY_VARIANT_TYPE_TARGET: &str = "target";
pub const GLOSSARY_ENTRY_VARIANT_TYPE_FORBIDDEN: &str = "forbidden";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntrySummary {
    pub id: String,
    pub glossary_id: String,
    pub source_term: String,
    pub target_term: String,
    pub context_note: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub source_variants: Vec<String>,
    pub target_variants: Vec<String>,
    pub forbidden_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntriesOverview {
    pub glossary_id: String,
    pub entries: Vec<GlossaryEntrySummary>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListGlossaryEntriesInput {
    pub glossary_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateGlossaryEntryInput {
    pub glossary_id: String,
    pub source_term: String,
    pub target_term: String,
    pub context_note: Option<String>,
    #[serde(default)]
    pub source_variants: Vec<String>,
    #[serde(default)]
    pub target_variants: Vec<String>,
    #[serde(default)]
    pub forbidden_terms: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateGlossaryEntryInput {
    pub glossary_entry_id: String,
    pub glossary_id: String,
    pub source_term: String,
    pub target_term: String,
    pub context_note: Option<String>,
    #[serde(default)]
    pub source_variants: Vec<String>,
    #[serde(default)]
    pub target_variants: Vec<String>,
    #[serde(default)]
    pub forbidden_terms: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGlossaryEntry {
    pub id: String,
    pub glossary_id: String,
    pub source_term: String,
    pub target_term: String,
    pub context_note: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub source_variants: Vec<String>,
    pub target_variants: Vec<String>,
    pub forbidden_terms: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlossaryEntryChanges {
    pub glossary_entry_id: String,
    pub glossary_id: String,
    pub source_term: String,
    pub target_term: String,
    pub context_note: Option<String>,
    pub status: String,
    pub updated_at: i64,
    pub source_variants: Vec<String>,
    pub target_variants: Vec<String>,
    pub forbidden_terms: Vec<String>,
}
