use serde::{Deserialize, Serialize};

pub const ACTIVE_RULE_SET_METADATA_KEY: &str = "workspace.active_rule_set_id";
pub const RULE_SET_STATUS_ACTIVE: &str = "active";
pub const RULE_SET_STATUS_ARCHIVED: &str = "archived";
pub const RULE_TYPE_CONSISTENCY: &str = "consistency";
pub const RULE_TYPE_PREFERENCE: &str = "preference";
pub const RULE_TYPE_RESTRICTION: &str = "restriction";
pub const RULE_ACTION_SCOPE_TRANSLATION: &str = "translation";
pub const RULE_ACTION_SCOPE_RETRANSLATION: &str = "retranslation";
pub const RULE_ACTION_SCOPE_QA: &str = "qa";
pub const RULE_ACTION_SCOPE_EXPORT: &str = "export";
pub const RULE_ACTION_SCOPE_CONSISTENCY_REVIEW: &str = "consistency_review";
pub const RULE_SEVERITY_LOW: &str = "low";
pub const RULE_SEVERITY_MEDIUM: &str = "medium";
pub const RULE_SEVERITY_HIGH: &str = "high";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuleSetSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuleSetsOverview {
    pub active_rule_set_id: Option<String>,
    pub rule_sets: Vec<RuleSetSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuleSummary {
    pub id: String,
    pub rule_set_id: String,
    pub action_scope: String,
    pub rule_type: String,
    pub severity: String,
    pub name: String,
    pub description: Option<String>,
    pub guidance: String,
    pub is_enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuleSetRulesOverview {
    pub rule_set_id: String,
    pub rules: Vec<RuleSummary>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuleSetInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRuleSetInput {
    pub rule_set_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListRuleSetRulesInput {
    pub rule_set_id: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuleInput {
    pub rule_set_id: String,
    pub action_scope: String,
    pub rule_type: String,
    pub severity: String,
    pub name: String,
    pub description: Option<String>,
    pub guidance: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRuleInput {
    pub rule_id: String,
    pub rule_set_id: String,
    pub action_scope: String,
    pub rule_type: String,
    pub severity: String,
    pub name: String,
    pub description: Option<String>,
    pub guidance: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRuleSet {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_opened_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSetChanges {
    pub rule_set_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRule {
    pub id: String,
    pub rule_set_id: String,
    pub action_scope: String,
    pub rule_type: String,
    pub severity: String,
    pub name: String,
    pub description: Option<String>,
    pub guidance: String,
    pub is_enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleChanges {
    pub rule_id: String,
    pub rule_set_id: String,
    pub action_scope: String,
    pub rule_type: String,
    pub severity: String,
    pub name: String,
    pub description: Option<String>,
    pub guidance: String,
    pub is_enabled: bool,
    pub updated_at: i64,
}
