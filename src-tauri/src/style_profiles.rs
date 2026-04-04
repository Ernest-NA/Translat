use serde::{Deserialize, Serialize};

pub const ACTIVE_STYLE_PROFILE_METADATA_KEY: &str = "workspace.active_style_profile_id";
pub const STYLE_PROFILE_STATUS_ACTIVE: &str = "active";
pub const STYLE_PROFILE_STATUS_ARCHIVED: &str = "archived";
pub const STYLE_PROFILE_TONE_NEUTRAL: &str = "neutral";
pub const STYLE_PROFILE_TONE_DIRECT: &str = "direct";
pub const STYLE_PROFILE_TONE_WARM: &str = "warm";
pub const STYLE_PROFILE_TONE_TECHNICAL: &str = "technical";
pub const STYLE_PROFILE_FORMALITY_FORMAL: &str = "formal";
pub const STYLE_PROFILE_FORMALITY_NEUTRAL: &str = "neutral";
pub const STYLE_PROFILE_FORMALITY_SEMI_FORMAL: &str = "semi_formal";
pub const STYLE_PROFILE_FORMALITY_INFORMAL: &str = "informal";
pub const STYLE_PROFILE_TREATMENT_USTED: &str = "usted";
pub const STYLE_PROFILE_TREATMENT_TUTEO: &str = "tuteo";
pub const STYLE_PROFILE_TREATMENT_IMPERSONAL: &str = "impersonal";
pub const STYLE_PROFILE_TREATMENT_MIXED: &str = "mixed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StyleProfileSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tone: String,
    pub formality: String,
    pub treatment_preference: String,
    pub consistency_instructions: Option<String>,
    pub editorial_notes: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StyleProfilesOverview {
    pub active_style_profile_id: Option<String>,
    pub style_profiles: Vec<StyleProfileSummary>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateStyleProfileInput {
    pub name: String,
    pub description: Option<String>,
    pub tone: String,
    pub formality: String,
    pub treatment_preference: String,
    pub consistency_instructions: Option<String>,
    pub editorial_notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStyleProfileInput {
    pub style_profile_id: String,
    pub name: String,
    pub description: Option<String>,
    pub tone: String,
    pub formality: String,
    pub treatment_preference: String,
    pub consistency_instructions: Option<String>,
    pub editorial_notes: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStyleProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tone: String,
    pub formality: String,
    pub treatment_preference: String,
    pub consistency_instructions: Option<String>,
    pub editorial_notes: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleProfileChanges {
    pub style_profile_id: String,
    pub name: String,
    pub description: Option<String>,
    pub tone: String,
    pub formality: String,
    pub treatment_preference: String,
    pub consistency_instructions: Option<String>,
    pub editorial_notes: Option<String>,
    pub status: String,
    pub updated_at: i64,
}
