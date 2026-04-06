use std::cmp::Reverse;
use std::collections::HashMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::chapter_contexts::{
    ChapterContextSummary, CHAPTER_CONTEXT_SCOPE_CHAPTER, CHAPTER_CONTEXT_SCOPE_DOCUMENT,
    CHAPTER_CONTEXT_SCOPE_RANGE, CHAPTER_CONTEXT_SCOPE_SECTION,
};
use crate::error::DesktopCommandError;
use crate::glossaries::{GlossarySummary, GLOSSARY_STATUS_ACTIVE};
use crate::glossary_entries::{GlossaryEntrySummary, GLOSSARY_ENTRY_STATUS_ACTIVE};
use crate::persistence::chapter_contexts::ChapterContextRepository;
use crate::persistence::glossaries::GlossaryRepository;
use crate::persistence::glossary_entries::GlossaryEntryRepository;
use crate::persistence::projects::ProjectRepository;
use crate::persistence::rule_sets::{RuleRepository, RuleSetRepository};
use crate::persistence::style_profiles::StyleProfileRepository;
use crate::persistence::translation_chunks::TranslationChunkRepository;
use crate::projects::ProjectSummary;
use crate::rule_sets::{
    RuleSetSummary, RuleSummary, RULE_ACTION_SCOPE_CONSISTENCY_REVIEW, RULE_ACTION_SCOPE_EXPORT,
    RULE_ACTION_SCOPE_QA, RULE_ACTION_SCOPE_RETRANSLATION, RULE_ACTION_SCOPE_TRANSLATION,
    RULE_SET_STATUS_ACTIVE, RULE_SEVERITY_HIGH, RULE_SEVERITY_LOW, RULE_SEVERITY_MEDIUM,
};
use crate::sections::DocumentSectionSummary;
use crate::segments::{DocumentSegmentsOverview, SegmentSummary};
use crate::style_profiles::{StyleProfileSummary, STYLE_PROFILE_STATUS_ACTIVE};
use crate::translation_chunks::{
    TranslationChunkSummary, TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER,
    TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
};

pub const EDITORIAL_LAYER_PROJECT: &str = "project";
pub const EDITORIAL_LAYER_SHARED: &str = "shared";
pub const EDITORIAL_SOURCE_PROJECT_DEFAULT: &str = "project_default";
pub const EDITORIAL_SOURCE_WORKSPACE_ACTIVE: &str = "workspace_active";
pub const CHAPTER_CONTEXT_MATCH_DOCUMENT: &str = "document";
pub const CHAPTER_CONTEXT_MATCH_RANGE: &str = "range";
pub const CHAPTER_CONTEXT_MATCH_SECTION: &str = "section";

const PROJECT_DEFAULT_PRIORITY: i64 = 300;
const WORKSPACE_ACTIVE_PRIORITY: i64 = 200;
const CHAPTER_CONTEXT_SECTION_PRIORITY: i64 = 300;
const CHAPTER_CONTEXT_RANGE_PRIORITY: i64 = 200;
const CHAPTER_CONTEXT_DOCUMENT_PRIORITY: i64 = 100;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildTranslationContextInput {
    pub project_id: String,
    pub document_id: String,
    pub chunk_id: String,
    pub action_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedGlossaryLayer {
    pub glossary: GlossarySummary,
    pub layer: String,
    pub source: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedGlossaryEntry {
    pub entry: GlossaryEntrySummary,
    pub glossary_name: String,
    pub layer: String,
    pub source: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedStyleProfile {
    pub style_profile: StyleProfileSummary,
    pub source: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRuleSet {
    pub rule_set: RuleSetSummary,
    pub source: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRule {
    pub rule: RuleSummary,
    pub rule_set_name: String,
    pub source: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslationChunkContext {
    pub chunk: TranslationChunkSummary,
    pub section: Option<DocumentSectionSummary>,
    pub core_segments: Vec<SegmentSummary>,
    pub context_before_segments: Vec<SegmentSummary>,
    pub context_after_segments: Vec<SegmentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedChapterContext {
    pub chapter_context: ChapterContextSummary,
    pub match_reason: String,
    pub priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslationContextResolution {
    pub glossary_ids: Vec<String>,
    pub style_profile_id: Option<String>,
    pub rule_set_id: Option<String>,
    pub section_id: Option<String>,
    pub chapter_context_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslationContextPreview {
    pub project_id: String,
    pub document_id: String,
    pub chunk_id: String,
    pub action_scope: String,
    pub glossary_layers: Vec<ResolvedGlossaryLayer>,
    pub glossary_entries: Vec<ResolvedGlossaryEntry>,
    pub style_profile: Option<ResolvedStyleProfile>,
    pub rule_set: Option<ResolvedRuleSet>,
    pub rules: Vec<ResolvedRule>,
    pub chunk_context: TranslationChunkContext,
    pub accumulated_contexts: Vec<ResolvedChapterContext>,
    pub resolution: TranslationContextResolution,
}

pub(crate) fn build_translation_context(
    connection: &mut Connection,
    segment_overview: &DocumentSegmentsOverview,
    input: &BuildTranslationContextInput,
) -> Result<TranslationContextPreview, DesktopCommandError> {
    let action_scope = validate_action_scope(&input.action_scope)?;
    let project = load_project_summary(connection, &input.project_id)?;
    let chunk_context = load_chunk_context(
        connection,
        &input.document_id,
        &input.chunk_id,
        segment_overview,
    )?;
    let glossary_layers = resolve_glossary_layers(connection, &project)?;
    let glossary_entries = resolve_glossary_entries(connection, &glossary_layers)?;
    let style_profile = resolve_style_profile(connection, &project)?;
    let (rule_set, rules) = resolve_rules(connection, &project, &action_scope)?;
    let accumulated_contexts = resolve_chapter_contexts(
        connection,
        &input.document_id,
        &segment_overview.sections,
        &chunk_context,
    )?;
    let resolution = TranslationContextResolution {
        glossary_ids: glossary_layers
            .iter()
            .map(|layer| layer.glossary.id.clone())
            .collect(),
        style_profile_id: style_profile
            .as_ref()
            .map(|resolved_style| resolved_style.style_profile.id.clone()),
        rule_set_id: rule_set
            .as_ref()
            .map(|resolved_rule_set| resolved_rule_set.rule_set.id.clone()),
        section_id: chunk_context
            .section
            .as_ref()
            .map(|section| section.id.clone()),
        chapter_context_ids: accumulated_contexts
            .iter()
            .map(|context| context.chapter_context.id.clone())
            .collect(),
    };

    Ok(TranslationContextPreview {
        project_id: input.project_id.clone(),
        document_id: input.document_id.clone(),
        chunk_id: input.chunk_id.clone(),
        action_scope,
        glossary_layers,
        glossary_entries,
        style_profile,
        rule_set,
        rules,
        chunk_context,
        accumulated_contexts,
        resolution,
    })
}

fn load_project_summary(
    connection: &mut Connection,
    project_id: &str,
) -> Result<ProjectSummary, DesktopCommandError> {
    ProjectRepository::new(connection)
        .load_overview()
        .map_err(|error| {
            DesktopCommandError::internal(
                "The context builder could not load the persisted projects.",
                Some(error.to_string()),
            )
        })?
        .projects
        .into_iter()
        .find(|project| project.id == project_id)
        .ok_or_else(|| {
            DesktopCommandError::validation("The selected project does not exist anymore.", None)
        })
}

fn load_chunk_context(
    connection: &mut Connection,
    document_id: &str,
    chunk_id: &str,
    segment_overview: &DocumentSegmentsOverview,
) -> Result<TranslationChunkContext, DesktopCommandError> {
    let (chunk, chunk_links) = {
        let mut repository = TranslationChunkRepository::new(connection);
        let chunks = repository
            .list_chunks_by_document(document_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The context builder could not load translation chunks for the selected document.",
                    Some(error.to_string()),
                )
            })?;
        let chunk = chunks
            .into_iter()
            .find(|chunk| chunk.id == chunk_id)
            .ok_or_else(|| {
                DesktopCommandError::validation(
                    "The selected translation chunk does not exist in the active document.",
                    None,
                )
            })?;
        let chunk_links = repository
            .list_chunk_segments_by_document(document_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The context builder could not load translation chunk links for the selected document.",
                    Some(error.to_string()),
                )
            })?
            .into_iter()
            .filter(|link| link.chunk_id == chunk_id)
            .collect::<Vec<_>>();

        (chunk, chunk_links)
    };
    let section = select_chunk_section(&segment_overview.sections, &chunk);
    let segment_map = segment_overview
        .segments
        .iter()
        .cloned()
        .map(|segment| (segment.id.clone(), segment))
        .collect::<HashMap<_, _>>();
    let mut core_segments = Vec::new();
    let mut context_before_segments = Vec::new();
    let mut context_after_segments = Vec::new();

    for link in chunk_links {
        let segment = segment_map.get(&link.segment_id).cloned().ok_or_else(|| {
            DesktopCommandError::internal(
                "The context builder could not map a persisted chunk link back to a document segment.",
                Some(format!(
                    "Missing segment {} while composing context for chunk {}.",
                    link.segment_id, chunk.id
                )),
            )
        })?;

        match link.role.as_str() {
            TRANSLATION_CHUNK_SEGMENT_ROLE_CORE => core_segments.push(segment),
            TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE => context_before_segments.push(segment),
            TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER => context_after_segments.push(segment),
            _ => {}
        }
    }

    if core_segments.is_empty() {
        return Err(DesktopCommandError::internal(
            "The selected translation chunk does not have a persisted core segment mapping.",
            Some(chunk.id),
        ));
    }

    Ok(TranslationChunkContext {
        chunk,
        section,
        core_segments,
        context_before_segments,
        context_after_segments,
    })
}

fn select_chunk_section(
    sections: &[DocumentSectionSummary],
    chunk: &TranslationChunkSummary,
) -> Option<DocumentSectionSummary> {
    select_chunk_sections(sections, chunk).into_iter().next()
}

fn select_chunk_sections(
    sections: &[DocumentSectionSummary],
    chunk: &TranslationChunkSummary,
) -> Vec<DocumentSectionSummary> {
    let mut containing_sections = sections
        .iter()
        .filter(|section| {
            section.start_segment_sequence <= chunk.start_segment_sequence
                && section.end_segment_sequence >= chunk.end_segment_sequence
        })
        .cloned()
        .collect::<Vec<_>>();

    containing_sections.sort_by_key(|section| {
        (
            section.segment_count,
            Reverse(section.level),
            section.sequence,
        )
    });

    containing_sections
}

fn resolve_glossary_layers(
    connection: &mut Connection,
    project: &ProjectSummary,
) -> Result<Vec<ResolvedGlossaryLayer>, DesktopCommandError> {
    let overview = GlossaryRepository::new(connection)
        .load_overview()
        .map_err(|error| {
            DesktopCommandError::internal(
                "The context builder could not load the persisted glossaries.",
                Some(error.to_string()),
            )
        })?;
    let mut glossary_layers = Vec::new();
    let mut seen_glossary_ids = Vec::new();

    if let Some(glossary_id) = project.default_glossary_id.as_deref() {
        if let Some(resolved_glossary) = resolve_optional_glossary(
            &overview.glossaries,
            glossary_id,
            &project.id,
            EDITORIAL_SOURCE_PROJECT_DEFAULT,
            PROJECT_DEFAULT_PRIORITY,
        ) {
            seen_glossary_ids.push(resolved_glossary.glossary.id.clone());
            glossary_layers.push(resolved_glossary);
        }
    }

    if let Some(glossary_id) = overview.active_glossary_id.as_deref() {
        if !seen_glossary_ids
            .iter()
            .any(|seen_id| seen_id == glossary_id)
        {
            if let Some(resolved_glossary) = resolve_optional_glossary(
                &overview.glossaries,
                glossary_id,
                &project.id,
                EDITORIAL_SOURCE_WORKSPACE_ACTIVE,
                WORKSPACE_ACTIVE_PRIORITY,
            ) {
                glossary_layers.push(resolved_glossary);
            }
        }
    }

    glossary_layers.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.glossary.name.cmp(&right.glossary.name))
            .then_with(|| left.glossary.id.cmp(&right.glossary.id))
    });

    Ok(glossary_layers)
}
fn resolve_optional_glossary(
    glossaries: &[GlossarySummary],
    glossary_id: &str,
    project_id: &str,
    source: &str,
    priority: i64,
) -> Option<ResolvedGlossaryLayer> {
    let glossary = glossaries
        .iter()
        .find(|glossary| glossary.id == glossary_id)?;

    if glossary.status != GLOSSARY_STATUS_ACTIVE {
        return None;
    }

    if glossary
        .project_id
        .as_deref()
        .is_some_and(|glossary_project_id| glossary_project_id != project_id)
    {
        return None;
    }

    Some(ResolvedGlossaryLayer {
        glossary: glossary.clone(),
        layer: if glossary.project_id.as_deref() == Some(project_id) {
            EDITORIAL_LAYER_PROJECT.to_owned()
        } else {
            EDITORIAL_LAYER_SHARED.to_owned()
        },
        source: source.to_owned(),
        priority,
    })
}

fn resolve_glossary_entries(
    connection: &mut Connection,
    glossary_layers: &[ResolvedGlossaryLayer],
) -> Result<Vec<ResolvedGlossaryEntry>, DesktopCommandError> {
    let mut selected_entries = HashMap::<String, ResolvedGlossaryEntry>::new();

    for glossary_layer in glossary_layers {
        let entries = GlossaryEntryRepository::new(connection)
            .list_by_glossary(&glossary_layer.glossary.id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The context builder could not load glossary entries for the selected editorial layer.",
                    Some(error.to_string()),
                )
            })?;

        for entry in entries {
            if entry.status != GLOSSARY_ENTRY_STATUS_ACTIVE {
                continue;
            }

            let resolved_entry = ResolvedGlossaryEntry {
                entry: entry.clone(),
                glossary_name: glossary_layer.glossary.name.clone(),
                layer: glossary_layer.layer.clone(),
                source: glossary_layer.source.clone(),
                priority: glossary_layer.priority,
            };
            let normalized_source_term = normalize_term_key(&entry.source_term);

            match selected_entries.get(&normalized_source_term) {
                Some(existing_entry)
                    if !should_replace_glossary_entry(existing_entry, &resolved_entry) => {}
                _ => {
                    selected_entries.insert(normalized_source_term, resolved_entry);
                }
            }
        }
    }

    let mut glossary_entries = selected_entries.into_values().collect::<Vec<_>>();
    glossary_entries.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| {
                normalize_term_key(&left.entry.source_term)
                    .cmp(&normalize_term_key(&right.entry.source_term))
            })
            .then_with(|| {
                normalize_term_key(&left.entry.target_term)
                    .cmp(&normalize_term_key(&right.entry.target_term))
            })
            .then_with(|| left.entry.id.cmp(&right.entry.id))
    });

    Ok(glossary_entries)
}

fn should_replace_glossary_entry(
    existing_entry: &ResolvedGlossaryEntry,
    candidate_entry: &ResolvedGlossaryEntry,
) -> bool {
    candidate_entry.priority > existing_entry.priority
        || (candidate_entry.priority == existing_entry.priority
            && candidate_entry.entry.updated_at > existing_entry.entry.updated_at)
        || (candidate_entry.priority == existing_entry.priority
            && candidate_entry.entry.updated_at == existing_entry.entry.updated_at
            && (candidate_entry.glossary_name < existing_entry.glossary_name
                || (candidate_entry.glossary_name == existing_entry.glossary_name
                    && candidate_entry.entry.id < existing_entry.entry.id)))
}

fn resolve_style_profile(
    connection: &mut Connection,
    project: &ProjectSummary,
) -> Result<Option<ResolvedStyleProfile>, DesktopCommandError> {
    let overview = StyleProfileRepository::new(connection)
        .load_overview()
        .map_err(|error| {
            DesktopCommandError::internal(
                "The context builder could not load the persisted style profiles.",
                Some(error.to_string()),
            )
        })?;

    if let Some(style_profile_id) = project.default_style_profile_id.as_deref() {
        if let Some(style_profile) = resolve_optional_style_profile(
            &overview.style_profiles,
            style_profile_id,
            EDITORIAL_SOURCE_PROJECT_DEFAULT,
            PROJECT_DEFAULT_PRIORITY,
        )
        {
            return Ok(Some(style_profile));
        }
    }

    Ok(overview
        .active_style_profile_id
        .as_deref()
        .and_then(|style_profile_id| {
            resolve_optional_style_profile(
                &overview.style_profiles,
                style_profile_id,
                EDITORIAL_SOURCE_WORKSPACE_ACTIVE,
                WORKSPACE_ACTIVE_PRIORITY,
            )
        }))
}

fn resolve_optional_style_profile(
    style_profiles: &[StyleProfileSummary],
    style_profile_id: &str,
    source: &str,
    priority: i64,
) -> Option<ResolvedStyleProfile> {
    let style_profile = style_profiles
        .iter()
        .find(|style_profile| style_profile.id == style_profile_id)?;

    if style_profile.status != STYLE_PROFILE_STATUS_ACTIVE {
        return None;
    }

    Some(ResolvedStyleProfile {
        style_profile: style_profile.clone(),
        source: source.to_owned(),
        priority,
    })
}

fn resolve_rules(
    connection: &mut Connection,
    project: &ProjectSummary,
    action_scope: &str,
) -> Result<(Option<ResolvedRuleSet>, Vec<ResolvedRule>), DesktopCommandError> {
    let overview = RuleSetRepository::new(connection)
        .load_overview()
        .map_err(|error| {
            DesktopCommandError::internal(
                "The context builder could not load the persisted rule sets.",
                Some(error.to_string()),
            )
        })?;
    let mut candidates = Vec::new();

    if let Some(rule_set_id) = project.default_rule_set_id.as_deref() {
        if let Some(rule_set) = resolve_optional_rule_set(
            &overview.rule_sets,
            rule_set_id,
            EDITORIAL_SOURCE_PROJECT_DEFAULT,
            PROJECT_DEFAULT_PRIORITY,
        ) {
            candidates.push(rule_set);
        }
    }

    if let Some(rule_set_id) = overview.active_rule_set_id.as_deref() {
        if let Some(rule_set) = resolve_optional_rule_set(
            &overview.rule_sets,
            rule_set_id,
            EDITORIAL_SOURCE_WORKSPACE_ACTIVE,
            WORKSPACE_ACTIVE_PRIORITY,
        ) {
            if !candidates
                .iter()
                .any(|candidate| candidate.rule_set.id == rule_set.rule_set.id)
            {
                candidates.push(rule_set);
            }
        }
    }

    let mut fallback_rule_set = None;
    let mut selected_rule_set = None;
    let mut combined_rules = Vec::new();

    for candidate in candidates {
        let rules = load_resolved_rules_for_rule_set(connection, &candidate, action_scope)?;

        if fallback_rule_set.is_none() {
            fallback_rule_set = Some(candidate.clone());
        }

        if !rules.is_empty() && selected_rule_set.is_none() {
            selected_rule_set = Some(candidate.clone());
        }

        if !rules.is_empty() {
            combined_rules.extend(rules);
        }
    }

    combined_rules.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| {
                rule_severity_rank(&left.rule.severity)
                    .cmp(&rule_severity_rank(&right.rule.severity))
            })
            .then_with(|| left.rule.name.cmp(&right.rule.name))
            .then_with(|| left.rule.id.cmp(&right.rule.id))
    });

    Ok((
        selected_rule_set.or(fallback_rule_set),
        combined_rules,
    ))
}

fn load_resolved_rules_for_rule_set(
    connection: &mut Connection,
    resolved_rule_set: &ResolvedRuleSet,
    action_scope: &str,
) -> Result<Vec<ResolvedRule>, DesktopCommandError> {
    let rules_overview = RuleRepository::new(connection)
        .load_overview(&resolved_rule_set.rule_set.id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The context builder could not load rules for the selected rule set.",
                Some(error.to_string()),
            )
        })?;
    let mut resolved_rules = rules_overview
        .rules
        .into_iter()
        .filter(|rule| rule.is_enabled && rule.action_scope == action_scope)
        .map(|rule| ResolvedRule {
            rule,
            rule_set_name: resolved_rule_set.rule_set.name.clone(),
            source: resolved_rule_set.source.clone(),
            priority: resolved_rule_set.priority,
        })
        .collect::<Vec<_>>();

    resolved_rules.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| {
                rule_severity_rank(&left.rule.severity)
                    .cmp(&rule_severity_rank(&right.rule.severity))
            })
            .then_with(|| left.rule.name.cmp(&right.rule.name))
            .then_with(|| left.rule.id.cmp(&right.rule.id))
    });

    Ok(resolved_rules)
}
fn resolve_optional_rule_set(
    rule_sets: &[RuleSetSummary],
    rule_set_id: &str,
    source: &str,
    priority: i64,
) -> Option<ResolvedRuleSet> {
    let rule_set = rule_sets
        .iter()
        .find(|rule_set| rule_set.id == rule_set_id)?;

    if rule_set.status != RULE_SET_STATUS_ACTIVE {
        return None;
    }

    Some(ResolvedRuleSet {
        rule_set: rule_set.clone(),
        source: source.to_owned(),
        priority,
    })
}

fn resolve_chapter_contexts(
    connection: &mut Connection,
    document_id: &str,
    sections: &[DocumentSectionSummary],
    chunk_context: &TranslationChunkContext,
) -> Result<Vec<ResolvedChapterContext>, DesktopCommandError> {
    let containing_sections = select_chunk_sections(sections, &chunk_context.chunk);
    let contexts = ChapterContextRepository::new(connection)
        .list_by_document(document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The context builder could not load the persisted accumulated chapter contexts.",
                Some(error.to_string()),
            )
        })?;
    let mut resolved_contexts = contexts
        .into_iter()
        .filter_map(|context| {
            resolve_chapter_context_match(
                &context,
                &containing_sections,
                chunk_context.chunk.start_segment_sequence,
                chunk_context.chunk.end_segment_sequence,
            )
            .map(|(match_reason, priority)| ResolvedChapterContext {
                chapter_context: context,
                match_reason: match_reason.to_owned(),
                priority,
            })
        })
        .collect::<Vec<_>>();

    resolved_contexts.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| {
                left.chapter_context
                    .start_segment_sequence
                    .cmp(&right.chapter_context.start_segment_sequence)
            })
            .then_with(|| {
                left.chapter_context
                    .end_segment_sequence
                    .cmp(&right.chapter_context.end_segment_sequence)
            })
            .then_with(|| left.chapter_context.id.cmp(&right.chapter_context.id))
    });

    Ok(resolved_contexts)
}

fn rule_severity_rank(severity: &str) -> i64 {
    match severity {
        RULE_SEVERITY_HIGH => 0,
        RULE_SEVERITY_MEDIUM => 1,
        RULE_SEVERITY_LOW => 2,
        _ => 3,
    }
}

fn resolve_chapter_context_match(
    context: &ChapterContextSummary,
    containing_sections: &[DocumentSectionSummary],
    chunk_start: i64,
    chunk_end: i64,
) -> Option<(&'static str, i64)> {
    if !ranges_overlap(
        context.start_segment_sequence,
        context.end_segment_sequence,
        chunk_start,
        chunk_end,
    ) {
        return None;
    }

    if let Some(context_section_id) = context.section_id.as_deref() {
        if let Some(section_index) = containing_sections
            .iter()
            .position(|section| section.id == context_section_id)
        {
            return Some((
                CHAPTER_CONTEXT_MATCH_SECTION,
                CHAPTER_CONTEXT_SECTION_PRIORITY
                    - i64::try_from(section_index).unwrap_or(CHAPTER_CONTEXT_SECTION_PRIORITY),
            ));
        }

        return None;
    }

    match context.scope_type.as_str() {
        CHAPTER_CONTEXT_SCOPE_DOCUMENT => Some((
            CHAPTER_CONTEXT_MATCH_DOCUMENT,
            CHAPTER_CONTEXT_DOCUMENT_PRIORITY,
        )),
        CHAPTER_CONTEXT_SCOPE_CHAPTER
        | CHAPTER_CONTEXT_SCOPE_SECTION
        | CHAPTER_CONTEXT_SCOPE_RANGE => {
            Some((CHAPTER_CONTEXT_MATCH_RANGE, CHAPTER_CONTEXT_RANGE_PRIORITY))
        }
        _ => Some((CHAPTER_CONTEXT_MATCH_RANGE, CHAPTER_CONTEXT_RANGE_PRIORITY)),
    }
}

fn ranges_overlap(left_start: i64, left_end: i64, right_start: i64, right_end: i64) -> bool {
    left_start <= right_end && right_start <= left_end
}

fn normalize_term_key(value: &str) -> String {
    value.trim().to_lowercase()
}

pub(crate) fn validate_action_scope(action_scope: &str) -> Result<String, DesktopCommandError> {
    let normalized_action_scope = action_scope.trim().to_ascii_lowercase();

    match normalized_action_scope.as_str() {
        RULE_ACTION_SCOPE_TRANSLATION
        | RULE_ACTION_SCOPE_RETRANSLATION
        | RULE_ACTION_SCOPE_QA
        | RULE_ACTION_SCOPE_EXPORT
        | RULE_ACTION_SCOPE_CONSISTENCY_REVIEW => Ok(normalized_action_scope),
        _ => Err(DesktopCommandError::validation(
            "The translation context preview requires a supported action scope.",
            None,
        )),
    }
}
