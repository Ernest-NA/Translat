use std::collections::{HashMap, HashSet};
use std::env;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::context_builder::TranslationContextPreview;
use crate::error::DesktopCommandError;
use crate::segments::SegmentTranslationWrite;
use crate::task_runs::TaskRunSummary;

pub const TRANSLATE_CHUNK_ACTION_TYPE: &str = "translate_chunk";
pub const TRANSLATE_CHUNK_ACTION_VERSION: &str = "tr15-translate-chunk-v1";
pub const TRANSLATE_CHUNK_PROMPT_VERSION: &str = "tr15-translate-chunk-prompt-v1";
pub const TRANSLATE_CHUNK_PROVIDER_OPENAI: &str = "openai";
pub const TRANSLATE_CHUNK_DEFAULT_MODEL: &str = "gpt-5-mini";
const OPENAI_RESPONSES_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";
const OPENAI_MODEL_ENV: &str = "TRANSLAT_OPENAI_MODEL";
const OPENAI_BASE_URL_ENV: &str = "OPENAI_BASE_URL";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkInput {
    pub project_id: String,
    pub document_id: String,
    pub chunk_id: String,
    pub job_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslatedChunkSegmentSummary {
    pub segment_id: String,
    pub sequence: i64,
    pub target_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkResult {
    pub project_id: String,
    pub document_id: String,
    pub chunk_id: String,
    pub task_run: TaskRunSummary,
    pub provider: String,
    pub model: String,
    pub action_version: String,
    pub prompt_version: String,
    pub translated_segments: Vec<TranslatedChunkSegmentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkActionRequest {
    pub action_type: String,
    pub action_version: String,
    pub prompt_version: String,
    pub target_locale: String,
    pub project_id: String,
    pub document_id: String,
    pub chunk_id: String,
    pub chunk_sequence: i64,
    pub source_text: String,
    pub context_before_text: Option<String>,
    pub context_after_text: Option<String>,
    pub glossary_entries: Vec<TranslateChunkGlossaryEntry>,
    pub style_profile: Option<TranslateChunkStyleProfile>,
    pub rules: Vec<TranslateChunkRule>,
    pub accumulated_contexts: Vec<TranslateChunkAccumulatedContext>,
    pub segments: Vec<TranslateChunkSegmentInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkGlossaryEntry {
    pub source_term: String,
    pub target_term: String,
    pub context_note: Option<String>,
    pub layer: String,
    pub source: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkStyleProfile {
    pub name: String,
    pub tone: String,
    pub formality: String,
    pub treatment_preference: String,
    pub consistency_instructions: Option<String>,
    pub editorial_notes: Option<String>,
    pub source: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkRule {
    pub name: String,
    pub rule_type: String,
    pub severity: String,
    pub guidance: String,
    pub source: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkAccumulatedContext {
    pub context_text: String,
    pub source_summary: Option<String>,
    pub match_reason: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkSegmentInput {
    pub segment_id: String,
    pub sequence: i64,
    pub source_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkActionResponse {
    pub translations: Vec<TranslateChunkTranslation>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkTranslation {
    pub segment_id: String,
    pub target_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateChunkTaskRunOutput {
    pub provider: String,
    pub model: String,
    pub action_version: String,
    pub prompt_version: String,
    pub translations: Vec<TranslateChunkTranslation>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslateChunkModelOutput {
    pub provider: String,
    pub model: String,
    pub raw_output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslateChunkExecutionFailure {
    pub message: String,
    pub details: Option<String>,
    pub raw_output: Option<String>,
}

pub trait TranslateChunkExecutor {
    fn execute(
        &self,
        request: &TranslateChunkActionRequest,
    ) -> Result<TranslateChunkModelOutput, TranslateChunkExecutionFailure>;
}

pub struct OpenAiTranslateChunkExecutor {
    client: Client,
    api_key: String,
    endpoint: String,
    model: String,
}

impl OpenAiTranslateChunkExecutor {
    pub fn from_environment() -> Result<Self, DesktopCommandError> {
        let api_key = env::var(OPENAI_API_KEY_ENV).map_err(|_| {
            DesktopCommandError::validation(
                "The translate_chunk action requires OPENAI_API_KEY in the desktop environment.",
                None,
            )
        })?;
        let model = env::var(OPENAI_MODEL_ENV).unwrap_or_else(|_| TRANSLATE_CHUNK_DEFAULT_MODEL.to_owned());
        let endpoint = env::var(OPENAI_BASE_URL_ENV)
            .map(|base_url| format!("{}/v1/responses", base_url.trim_end_matches('/')))
            .unwrap_or_else(|_| OPENAI_RESPONSES_ENDPOINT.to_owned());
        let client = Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not initialize the HTTP client for translate_chunk.",
                    Some(error.to_string()),
                )
            })?;

        Ok(Self {
            client,
            api_key,
            endpoint,
            model,
        })
    }
}

impl TranslateChunkExecutor for OpenAiTranslateChunkExecutor {
    fn execute(
        &self,
        request: &TranslateChunkActionRequest,
    ) -> Result<TranslateChunkModelOutput, TranslateChunkExecutionFailure> {
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": self.model,
                "instructions": build_system_prompt(),
                "input": build_user_prompt(request)
            }))
            .send()
            .map_err(|error| TranslateChunkExecutionFailure {
                message: "The OpenAI translation request could not be completed.".to_owned(),
                details: Some(error.to_string()),
                raw_output: None,
            })?;
        let status = response.status();
        let response_body = response.text().map_err(|error| TranslateChunkExecutionFailure {
            message: "The OpenAI translation response could not be read.".to_owned(),
            details: Some(error.to_string()),
            raw_output: None,
        })?;

        if !status.is_success() {
            return Err(TranslateChunkExecutionFailure {
                message: "The OpenAI translation request returned an error response.".to_owned(),
                details: Some(format!("status {} body {}", status.as_u16(), response_body)),
                raw_output: Some(response_body),
            });
        }

        let parsed_response: OpenAiResponsesEnvelope =
            serde_json::from_str(&response_body).map_err(|error| TranslateChunkExecutionFailure {
                message:
                    "The OpenAI translation response could not be decoded as a Responses API payload."
                        .to_owned(),
                details: Some(error.to_string()),
                raw_output: Some(response_body.clone()),
            })?;
        let raw_output = parsed_response.extract_text().ok_or_else(|| TranslateChunkExecutionFailure {
            message: "The OpenAI translation response did not contain text output.".to_owned(),
            details: Some(response_body.clone()),
            raw_output: Some(response_body.clone()),
        })?;

        Ok(TranslateChunkModelOutput {
            provider: TRANSLATE_CHUNK_PROVIDER_OPENAI.to_owned(),
            model: self.model.clone(),
            raw_output,
        })
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesEnvelope {
    output: Vec<OpenAiResponsesOutputItem>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesOutputItem {
    #[serde(rename = "type")]
    item_type: String,
    content: Option<Vec<OpenAiResponsesContentItem>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponsesContentItem {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

impl OpenAiResponsesEnvelope {
    fn extract_text(&self) -> Option<String> {
        for output_item in &self.output {
            if output_item.item_type != "message" {
                continue;
            }

            for content_item in output_item.content.as_deref().unwrap_or(&[]) {
                if content_item.content_type == "output_text" {
                    if let Some(text) = content_item.text.as_deref() {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            return Some(trimmed.to_owned());
                        }
                    }
                }
            }
        }

        None
    }
}

pub fn build_action_request(context: &TranslationContextPreview) -> TranslateChunkActionRequest {
    TranslateChunkActionRequest {
        action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
        action_version: TRANSLATE_CHUNK_ACTION_VERSION.to_owned(),
        prompt_version: TRANSLATE_CHUNK_PROMPT_VERSION.to_owned(),
        target_locale: "es-ES".to_owned(),
        project_id: context.project_id.clone(),
        document_id: context.document_id.clone(),
        chunk_id: context.chunk_id.clone(),
        chunk_sequence: context.chunk_context.chunk.sequence,
        source_text: context.chunk_context.chunk.source_text.clone(),
        context_before_text: context.chunk_context.chunk.context_before_text.clone(),
        context_after_text: context.chunk_context.chunk.context_after_text.clone(),
        glossary_entries: context
            .glossary_entries
            .iter()
            .map(|entry| TranslateChunkGlossaryEntry {
                source_term: entry.entry.source_term.clone(),
                target_term: entry.entry.target_term.clone(),
                context_note: entry.entry.context_note.clone(),
                layer: entry.layer.clone(),
                source: entry.source.clone(),
                priority: entry.priority,
            })
            .collect(),
        style_profile: context.style_profile.as_ref().map(|style_profile| {
            TranslateChunkStyleProfile {
                name: style_profile.style_profile.name.clone(),
                tone: style_profile.style_profile.tone.clone(),
                formality: style_profile.style_profile.formality.clone(),
                treatment_preference: style_profile.style_profile.treatment_preference.clone(),
                consistency_instructions: style_profile
                    .style_profile
                    .consistency_instructions
                    .clone(),
                editorial_notes: style_profile.style_profile.editorial_notes.clone(),
                source: style_profile.source.clone(),
                priority: style_profile.priority,
            }
        }),
        rules: context
            .rules
            .iter()
            .map(|rule| TranslateChunkRule {
                name: rule.rule.name.clone(),
                rule_type: rule.rule.rule_type.clone(),
                severity: rule.rule.severity.clone(),
                guidance: rule.rule.guidance.clone(),
                source: rule.source.clone(),
                priority: rule.priority,
            })
            .collect(),
        accumulated_contexts: context
            .accumulated_contexts
            .iter()
            .map(|resolved_context| TranslateChunkAccumulatedContext {
                context_text: resolved_context.chapter_context.context_text.clone(),
                source_summary: resolved_context.chapter_context.source_summary.clone(),
                match_reason: resolved_context.match_reason.clone(),
                priority: resolved_context.priority,
            })
            .collect(),
        segments: context
            .chunk_context
            .core_segments
            .iter()
            .map(|segment| TranslateChunkSegmentInput {
                segment_id: segment.id.clone(),
                sequence: segment.sequence,
                source_text: segment.source_text.clone(),
            })
            .collect(),
    }
}

pub fn parse_action_response(
    raw_output: &str,
) -> Result<TranslateChunkActionResponse, DesktopCommandError> {
    let normalized_output = strip_json_fence(raw_output).trim().to_owned();

    if normalized_output.is_empty() {
        return Err(DesktopCommandError::internal(
            "The translate_chunk action returned an empty model response.",
            None,
        ));
    }

    serde_json::from_str(&normalized_output).map_err(|error| {
        DesktopCommandError::validation(
            "The translate_chunk action did not return valid JSON.",
            Some(error.to_string()),
        )
    })
}

pub fn validate_action_response(
    request: &TranslateChunkActionRequest,
    response: &TranslateChunkActionResponse,
) -> Result<Vec<ValidatedTranslatedSegment>, DesktopCommandError> {
    if response.translations.is_empty() {
        return Err(DesktopCommandError::validation(
            "The translate_chunk action returned no translated segments.",
            None,
        ));
    }

    let expected_segments = request
        .segments
        .iter()
        .map(|segment| (segment.segment_id.clone(), segment))
        .collect::<HashMap<_, _>>();
    let expected_segment_ids = expected_segments.keys().cloned().collect::<HashSet<_>>();
    let mut seen_segment_ids = HashSet::new();

    if response.translations.len() != request.segments.len() {
        return Err(DesktopCommandError::validation(
            "The translate_chunk action returned an unexpected number of translated segments.",
            Some(format!(
                "expected {} translated segments but received {}",
                request.segments.len(),
                response.translations.len()
            )),
        ));
    }

    for translation in &response.translations {
        if !expected_segment_ids.contains(&translation.segment_id) {
            return Err(DesktopCommandError::validation(
                "The translate_chunk action returned a segment id outside the active chunk core.",
                Some(translation.segment_id.clone()),
            ));
        }

        if !seen_segment_ids.insert(translation.segment_id.clone()) {
            return Err(DesktopCommandError::validation(
                "The translate_chunk action returned duplicate segment ids.",
                Some(translation.segment_id.clone()),
            ));
        }

        if translation.target_text.trim().is_empty() {
            return Err(DesktopCommandError::validation(
                "The translate_chunk action returned an empty translated segment.",
                Some(translation.segment_id.clone()),
            ));
        }
    }

    let ordered_translations = request
        .segments
        .iter()
        .map(|segment| {
            let translated_segment = response
                .translations
                .iter()
                .find(|translation| translation.segment_id == segment.segment_id)
                .ok_or_else(|| {
                    DesktopCommandError::validation(
                        "The translate_chunk action omitted a required segment translation.",
                        Some(segment.segment_id.clone()),
                    )
                })?;

            Ok(ValidatedTranslatedSegment {
                segment_id: segment.segment_id.clone(),
                sequence: segment.sequence,
                target_text: translated_segment.target_text.trim().to_owned(),
            })
        })
        .collect::<Result<Vec<_>, DesktopCommandError>>()?;

    Ok(ordered_translations)
}

pub fn serialize_task_run_output(
    provider: &str,
    model: &str,
    response: &TranslateChunkActionResponse,
) -> Result<String, DesktopCommandError> {
    serde_json::to_string(&TranslateChunkTaskRunOutput {
        provider: provider.to_owned(),
        model: model.to_owned(),
        action_version: TRANSLATE_CHUNK_ACTION_VERSION.to_owned(),
        prompt_version: TRANSLATE_CHUNK_PROMPT_VERSION.to_owned(),
        translations: response.translations.clone(),
        notes: response.notes.clone(),
    })
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not serialize the translate_chunk output payload.",
            Some(error.to_string()),
        )
    })
}

fn build_system_prompt() -> &'static str {
    "You are Translat, an editorial translation engine specialized in translating English source segments into Castilian Spanish. Translate only the provided core segments. Respect glossary targets, style instructions, action-scoped rules, and accumulated context. Return only JSON with this exact shape: {\"translations\":[{\"segmentId\":\"...\",\"targetText\":\"...\"}],\"notes\":string|null}. Do not add Markdown fences or explanations."
}

fn build_user_prompt(request: &TranslateChunkActionRequest) -> String {
    format!(
        "Translate the following chunk request into Castilian Spanish and return only JSON.\n\n{}",
        serde_json::to_string_pretty(request)
            .unwrap_or_else(|_| "{\"serialization\":\"failed\"}".to_owned())
    )
}

fn strip_json_fence(raw_output: &str) -> &str {
    let trimmed = raw_output.trim();

    if let Some(stripped) = trimmed.strip_prefix("```json") {
        return stripped.trim().trim_end_matches("```").trim();
    }

    if let Some(stripped) = trimmed.strip_prefix("```") {
        return stripped.trim().trim_end_matches("```").trim();
    }

    trimmed
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTranslatedSegment {
    pub segment_id: String,
    pub sequence: i64,
    pub target_text: String,
}

impl ValidatedTranslatedSegment {
    pub fn to_segment_write(&self) -> SegmentTranslationWrite {
        SegmentTranslationWrite {
            segment_id: self.segment_id.clone(),
            target_text: self.target_text.clone(),
        }
    }

    pub fn to_summary(&self) -> TranslatedChunkSegmentSummary {
        TranslatedChunkSegmentSummary {
            segment_id: self.segment_id.clone(),
            sequence: self.sequence,
            target_text: self.target_text.clone(),
        }
    }
}
