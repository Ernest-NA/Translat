use tauri::State;

use crate::commands::segments::load_segmented_document_overview;
use crate::context_builder::{
    build_translation_context as compose_translation_context, validate_action_scope,
    BuildTranslationContextInput, TranslationContextPreview,
};
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;

#[tauri::command]
pub fn build_translation_context(
    input: BuildTranslationContextInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<TranslationContextPreview, DesktopCommandError> {
    build_translation_context_with_runtime(input, database_runtime.inner())
}

pub(crate) fn build_translation_context_with_runtime(
    input: BuildTranslationContextInput,
    database_runtime: &DatabaseRuntime,
) -> Result<TranslationContextPreview, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let chunk_id = validate_identifier(&input.chunk_id, "chunk id")?;
    let action_scope = validate_action_scope(&input.action_scope)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for translation-context building.",
            Some(error.to_string()),
        )
    })?;
    let timestamp = current_timestamp()?;
    let segment_overview = load_segmented_document_overview(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        false,
        timestamp,
    )?;

    compose_translation_context(
        &mut connection,
        &segment_overview,
        &BuildTranslationContextInput {
            project_id,
            document_id,
            chunk_id,
            action_scope,
        },
    )
}

fn current_timestamp() -> Result<i64, DesktopCommandError> {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not read the system clock while building translation context.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid translation-context timestamp.",
            Some(error.to_string()),
        )
    })
}

fn validate_identifier(value: &str, label: &str) -> Result<String, DesktopCommandError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(DesktopCommandError::validation(
            format!("The translation context preview requires a valid {label}."),
            None,
        ));
    }

    if !trimmed
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(
            format!("The translation context preview requires a safe persisted {label}."),
            None,
        ));
    }

    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::build_translation_context_with_runtime;
    use crate::chapter_contexts::{NewChapterContext, CHAPTER_CONTEXT_SCOPE_DOCUMENT};
    use crate::context_builder::{
        BuildTranslationContextInput, TranslationContextPreview, CHAPTER_CONTEXT_MATCH_DOCUMENT,
        CHAPTER_CONTEXT_MATCH_SECTION, EDITORIAL_SOURCE_PROJECT_DEFAULT,
        EDITORIAL_SOURCE_WORKSPACE_ACTIVE,
    };
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::glossaries::GLOSSARY_STATUS_ARCHIVED;
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::sections::DocumentSectionRepository;
    use crate::persistence::chapter_contexts::ChapterContextRepository;
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::glossaries::GlossaryRepository;
    use crate::persistence::glossary_entries::GlossaryEntryRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::rule_sets::{RuleRepository, RuleSetRepository};
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::style_profiles::StyleProfileRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::{NewProject, ProjectEditorialDefaultsChanges};
    use crate::rule_sets::{
        NewRule, NewRuleSet, RULE_ACTION_SCOPE_EXPORT, RULE_ACTION_SCOPE_QA,
        RULE_ACTION_SCOPE_TRANSLATION, RULE_SET_STATUS_ACTIVE, RULE_SET_STATUS_ARCHIVED,
        RULE_SEVERITY_HIGH, RULE_SEVERITY_LOW, RULE_SEVERITY_MEDIUM,
        RULE_TYPE_CONSISTENCY, RULE_TYPE_PREFERENCE, RULE_TYPE_RESTRICTION,
    };
    use crate::sections::NewDocumentSection;
    use crate::segments::{NewSegment, SEGMENT_STATUS_PENDING_TRANSLATION};
    use crate::style_profiles::{
        NewStyleProfile, STYLE_PROFILE_FORMALITY_FORMAL, STYLE_PROFILE_STATUS_ACTIVE,
        STYLE_PROFILE_STATUS_ARCHIVED, STYLE_PROFILE_TONE_DIRECT, STYLE_PROFILE_TONE_TECHNICAL,
        STYLE_PROFILE_TREATMENT_TUTEO, STYLE_PROFILE_TREATMENT_USTED,
    };
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };
    use crate::{
        glossaries::{NewGlossary, GLOSSARY_STATUS_ACTIVE},
        glossary_entries::{NewGlossaryEntry, GLOSSARY_ENTRY_STATUS_ACTIVE},
    };

    fn create_runtime() -> (tempfile::TempDir, DatabaseRuntime) {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key =
            load_or_create_encryption_key(&encryption_key_path).expect("key should persist");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        (temporary_directory, runtime)
    }

    fn seed_context_builder_graph(runtime: &DatabaseRuntime) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_800_000_000_i64;

        let project = ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: "prj_active_001".to_owned(),
                name: "Context project".to_owned(),
                description: None,
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("project should persist");
        ProjectRepository::new(&mut connection)
            .open_project(&project.id, now)
            .expect("project should become active");

        let global_glossary = GlossaryRepository::new(&mut connection)
            .create(&NewGlossary {
                id: "gls_global_001".to_owned(),
                name: "Global glossary".to_owned(),
                description: None,
                project_id: None,
                status: GLOSSARY_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("global glossary should persist");
        let project_glossary = GlossaryRepository::new(&mut connection)
            .create(&NewGlossary {
                id: "gls_project_001".to_owned(),
                name: "Project glossary".to_owned(),
                description: None,
                project_id: Some(project.id.clone()),
                status: GLOSSARY_STATUS_ACTIVE.to_owned(),
                created_at: now + 1,
                updated_at: now + 1,
                last_opened_at: now + 1,
            })
            .expect("project glossary should persist");
        GlossaryRepository::new(&mut connection)
            .open_glossary(&global_glossary.id, now + 2)
            .expect("global glossary should become workspace active");

        GlossaryEntryRepository::new(&mut connection)
            .create(&NewGlossaryEntry {
                id: "gle_global_001".to_owned(),
                glossary_id: global_glossary.id.clone(),
                source_term: "Order".to_owned(),
                target_term: "Orden".to_owned(),
                context_note: Some("Global fallback.".to_owned()),
                status: GLOSSARY_ENTRY_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                source_variants: vec!["Orders".to_owned()],
                target_variants: vec!["Órdenes".to_owned()],
                forbidden_terms: vec!["Pedido".to_owned()],
            })
            .expect("global glossary entry should persist");
        GlossaryEntryRepository::new(&mut connection)
            .create(&NewGlossaryEntry {
                id: "gle_project_001".to_owned(),
                glossary_id: project_glossary.id.clone(),
                source_term: "Order".to_owned(),
                target_term: "Mandato".to_owned(),
                context_note: Some("Project-preferred term.".to_owned()),
                status: GLOSSARY_ENTRY_STATUS_ACTIVE.to_owned(),
                created_at: now + 1,
                updated_at: now + 1,
                source_variants: vec!["Orders".to_owned()],
                target_variants: vec!["Mandatos".to_owned()],
                forbidden_terms: vec!["Orden".to_owned()],
            })
            .expect("project glossary entry should persist");

        let style_profile = StyleProfileRepository::new(&mut connection)
            .create(&NewStyleProfile {
                id: "stp_project_001".to_owned(),
                name: "Technical style".to_owned(),
                description: None,
                tone: STYLE_PROFILE_TONE_TECHNICAL.to_owned(),
                formality: STYLE_PROFILE_FORMALITY_FORMAL.to_owned(),
                treatment_preference: STYLE_PROFILE_TREATMENT_USTED.to_owned(),
                consistency_instructions: Some("Keep command labels stable.".to_owned()),
                editorial_notes: Some("Avoid paraphrasing safety text.".to_owned()),
                status: STYLE_PROFILE_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("style profile should persist");

        let rule_set = RuleSetRepository::new(&mut connection)
            .create(&NewRuleSet {
                id: "rset_project_001".to_owned(),
                name: "Translation rules".to_owned(),
                description: None,
                status: RULE_SET_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("rule set should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_translation_001".to_owned(),
                rule_set_id: rule_set.id.clone(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
                rule_type: RULE_TYPE_RESTRICTION.to_owned(),
                severity: RULE_SEVERITY_HIGH.to_owned(),
                name: "Keep command names stable".to_owned(),
                description: Some("Use exact command labels.".to_owned()),
                guidance: "Never rename command labels inside the translation.".to_owned(),
                is_enabled: true,
                created_at: now,
                updated_at: now,
            })
            .expect("translation rule should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_qa_001".to_owned(),
                rule_set_id: rule_set.id.clone(),
                action_scope: RULE_ACTION_SCOPE_QA.to_owned(),
                rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                severity: RULE_SEVERITY_MEDIUM.to_owned(),
                name: "QA-only rule".to_owned(),
                description: None,
                guidance: "Inspect terminology drift during QA only.".to_owned(),
                is_enabled: true,
                created_at: now + 1,
                updated_at: now + 1,
            })
            .expect("qa rule should persist");

        ProjectRepository::new(&mut connection)
            .update_editorial_defaults(&ProjectEditorialDefaultsChanges {
                project_id: project.id.clone(),
                default_glossary_id: Some(project_glossary.id.clone()),
                default_style_profile_id: Some(style_profile.id.clone()),
                default_rule_set_id: Some(rule_set.id.clone()),
                updated_at: now + 2,
            })
            .expect("project defaults should persist");

        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: "doc_chunk_001".to_owned(),
                project_id: project.id.clone(),
                name: "chaptered.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "ignored".to_owned(),
                file_size_bytes: 256,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: now,
                updated_at: now,
            })
            .expect("document should persist");

        SegmentRepository::new(&mut connection)
            .replace_for_document(
                &project.id,
                "doc_chunk_001",
                &[
                    NewSegment {
                        id: "doc_chunk_001_seg_0001".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 1,
                        source_text: "Chapter 1".to_owned(),
                        source_word_count: 2,
                        source_character_count: 9,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "doc_chunk_001_seg_0002".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 2,
                        source_text: "The Order remains active.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 25,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "doc_chunk_001_seg_0003".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 3,
                        source_text: "Follow the command list.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 24,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "doc_chunk_001_seg_0004".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 4,
                        source_text: "Closing detail.".to_owned(),
                        source_word_count: 2,
                        source_character_count: 15,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                ],
                now,
            )
            .expect("segments should persist");

        crate::persistence::sections::DocumentSectionRepository::new(&mut connection)
            .replace_for_document(
                "doc_chunk_001",
                &[NewDocumentSection {
                    id: "doc_chunk_001_sec_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    title: "Chapter 1".to_owned(),
                    section_type: "chapter".to_owned(),
                    level: 1,
                    start_segment_sequence: 1,
                    end_segment_sequence: 4,
                    segment_count: 4,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect("sections should persist");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document(
                "doc_chunk_001",
                &[NewTranslationChunk {
                    id: "doc_chunk_001_chunk_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    builder_version: "tr12-basic-v1".to_owned(),
                    strategy: "section-aware-fixed-word-target-v1".to_owned(),
                    source_text: "The Order remains active.\n\nFollow the command list.".to_owned(),
                    context_before_text: Some("Chapter 1".to_owned()),
                    context_after_text: Some("Closing detail.".to_owned()),
                    start_segment_sequence: 2,
                    end_segment_sequence: 3,
                    segment_count: 2,
                    source_word_count: 8,
                    source_character_count: 49,
                    created_at: now,
                    updated_at: now,
                }],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0002".to_owned(),
                        segment_sequence: 2,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0003".to_owned(),
                        segment_sequence: 3,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0004".to_owned(),
                        segment_sequence: 4,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                ],
            )
            .expect("chunk should persist");

        ChapterContextRepository::new(&mut connection)
            .replace_for_document(
                "doc_chunk_001",
                &[
                    NewChapterContext {
                        id: "ctx_section_001".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        section_id: Some("doc_chunk_001_sec_0001".to_owned()),
                        task_run_id: None,
                        scope_type: "chapter".to_owned(),
                        start_segment_sequence: 1,
                        end_segment_sequence: 4,
                        context_text: "This chapter keeps military register.".to_owned(),
                        source_summary: Some("Opening context.".to_owned()),
                        context_word_count: 5,
                        context_character_count: 37,
                        created_at: now,
                        updated_at: now,
                    },
                    NewChapterContext {
                        id: "ctx_document_001".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        section_id: None,
                        task_run_id: None,
                        scope_type: CHAPTER_CONTEXT_SCOPE_DOCUMENT.to_owned(),
                        start_segment_sequence: 1,
                        end_segment_sequence: 4,
                        context_text: "Document-wide tone memory.".to_owned(),
                        source_summary: None,
                        context_word_count: 3,
                        context_character_count: 26,
                        created_at: now + 1,
                        updated_at: now + 1,
                    },
                ],
            )
            .expect("chapter contexts should persist");
    }

    fn build_preview(runtime: &DatabaseRuntime, action_scope: &str) -> TranslationContextPreview {
        build_translation_context_with_runtime(
            BuildTranslationContextInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                action_scope: action_scope.to_owned(),
            },
            runtime,
        )
        .expect("translation context should build")
    }

    #[test]
    fn build_translation_context_rejects_invalid_inputs() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);

        let invalid_chunk_error = build_translation_context_with_runtime(
            BuildTranslationContextInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: "missing_chunk".to_owned(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
            },
            &runtime,
        )
        .expect_err("missing chunk should fail");

        assert!(invalid_chunk_error
            .message
            .contains("does not exist in the active document"));

        let invalid_scope_error = build_translation_context_with_runtime(
            BuildTranslationContextInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                action_scope: "unsupported".to_owned(),
            },
            &runtime,
        )
        .expect_err("unsupported action scope should fail");

        assert!(invalid_scope_error
            .message
            .contains("requires a supported action scope"));
    }

    #[test]
    fn build_translation_context_resolves_editorial_layers_and_deduplicates_glossary_entries() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);

        let preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);

        assert_eq!(preview.glossary_layers.len(), 2);
        assert_eq!(
            preview.glossary_layers[0].source,
            EDITORIAL_SOURCE_PROJECT_DEFAULT
        );
        assert_eq!(preview.glossary_entries.len(), 1);
        assert_eq!(preview.glossary_entries[0].entry.target_term, "Mandato");
        assert_eq!(
            preview
                .style_profile
                .as_ref()
                .map(|style| style.style_profile.name.as_str()),
            Some("Technical style")
        );
    }

    #[test]
    fn build_translation_context_falls_back_from_archived_project_defaults() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_800_000_200_i64;

        let fallback_style = StyleProfileRepository::new(&mut connection)
            .create(&NewStyleProfile {
                id: "stp_workspace_001".to_owned(),
                name: "Fallback style".to_owned(),
                description: None,
                tone: STYLE_PROFILE_TONE_DIRECT.to_owned(),
                formality: STYLE_PROFILE_FORMALITY_FORMAL.to_owned(),
                treatment_preference: STYLE_PROFILE_TREATMENT_TUTEO.to_owned(),
                consistency_instructions: Some("Fallback style instructions.".to_owned()),
                editorial_notes: None,
                status: STYLE_PROFILE_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("fallback style should persist");
        StyleProfileRepository::new(&mut connection)
            .open_style_profile(&fallback_style.id, now + 1)
            .expect("fallback style should become active");

        let fallback_rule_set = RuleSetRepository::new(&mut connection)
            .create(&NewRuleSet {
                id: "rset_workspace_001".to_owned(),
                name: "Fallback rules".to_owned(),
                description: None,
                status: RULE_SET_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("fallback rule set should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_workspace_001".to_owned(),
                rule_set_id: fallback_rule_set.id.clone(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
                rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                severity: RULE_SEVERITY_MEDIUM.to_owned(),
                name: "Fallback rule".to_owned(),
                description: None,
                guidance: "Fallback rule guidance.".to_owned(),
                is_enabled: true,
                created_at: now,
                updated_at: now,
            })
            .expect("fallback rule should persist");
        RuleSetRepository::new(&mut connection)
            .open_rule_set(&fallback_rule_set.id, now + 1)
            .expect("fallback rule set should become active");

        connection
            .execute(
                "UPDATE glossaries SET status = ?2 WHERE id = ?1",
                rusqlite::params!["gls_project_001", GLOSSARY_STATUS_ARCHIVED],
            )
            .expect("project glossary should archive");
        connection
            .execute(
                "UPDATE style_profiles SET status = ?2 WHERE id = ?1",
                rusqlite::params!["stp_project_001", STYLE_PROFILE_STATUS_ARCHIVED],
            )
            .expect("project style should archive");
        connection
            .execute(
                "UPDATE rule_sets SET status = ?2 WHERE id = ?1",
                rusqlite::params!["rset_project_001", RULE_SET_STATUS_ARCHIVED],
            )
            .expect("project rule set should archive");
        drop(connection);

        let preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);

        assert_eq!(preview.glossary_layers.len(), 1);
        assert_eq!(
            preview.glossary_layers[0].source,
            EDITORIAL_SOURCE_WORKSPACE_ACTIVE
        );
        assert_eq!(preview.glossary_layers[0].glossary.id, "gls_global_001");
        assert_eq!(preview.glossary_entries[0].entry.target_term, "Orden");
        assert_eq!(
            preview
                .style_profile
                .as_ref()
                .map(|style| style.style_profile.id.as_str()),
            Some("stp_workspace_001")
        );
        assert_eq!(
            preview
                .rule_set
                .as_ref()
                .map(|rule_set| rule_set.rule_set.id.as_str()),
            Some("rset_workspace_001")
        );
        assert_eq!(
            preview.rules.iter().map(|rule| rule.rule.name.as_str()).collect::<Vec<_>>(),
            vec!["Fallback rule"]
        );
    }

    #[test]
    fn build_translation_context_falls_back_to_workspace_rule_set_for_missing_scope() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_800_000_250_i64;

        let fallback_rule_set = RuleSetRepository::new(&mut connection)
            .create(&NewRuleSet {
                id: "rset_workspace_scope_001".to_owned(),
                name: "Scoped fallback rules".to_owned(),
                description: None,
                status: RULE_SET_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("scoped fallback rule set should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_workspace_qa_001".to_owned(),
                rule_set_id: fallback_rule_set.id.clone(),
                action_scope: RULE_ACTION_SCOPE_EXPORT.to_owned(),
                rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                severity: RULE_SEVERITY_HIGH.to_owned(),
                name: "Workspace export fallback".to_owned(),
                description: None,
                guidance: "Export fallback guidance.".to_owned(),
                is_enabled: true,
                created_at: now,
                updated_at: now,
            })
            .expect("scoped fallback export rule should persist");
        RuleSetRepository::new(&mut connection)
            .open_rule_set(&fallback_rule_set.id, now + 1)
            .expect("scoped fallback rule set should become active");
        drop(connection);

        let preview = build_preview(&runtime, RULE_ACTION_SCOPE_EXPORT);

        assert_eq!(
            preview
                .rule_set
                .as_ref()
                .map(|rule_set| rule_set.rule_set.id.as_str()),
            Some("rset_workspace_scope_001")
        );
        assert_eq!(
            preview.rules.iter().map(|rule| rule.rule.name.as_str()).collect::<Vec<_>>(),
            vec!["Workspace export fallback"]
        );
    }

    #[test]
    fn build_translation_context_filters_rules_by_action_scope() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_800_000_100_i64;

        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_translation_002".to_owned(),
                rule_set_id: "rset_project_001".to_owned(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
                rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                severity: RULE_SEVERITY_MEDIUM.to_owned(),
                name: "Keep list formatting stable".to_owned(),
                description: None,
                guidance: "Preserve enumerated formatting in the translation.".to_owned(),
                is_enabled: true,
                created_at: now,
                updated_at: now,
            })
            .expect("medium translation rule should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_translation_003".to_owned(),
                rule_set_id: "rset_project_001".to_owned(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
                rule_type: RULE_TYPE_PREFERENCE.to_owned(),
                severity: RULE_SEVERITY_LOW.to_owned(),
                name: "Prefer recurring noun phrases".to_owned(),
                description: None,
                guidance: "Prefer repeated noun phrases when they remain natural.".to_owned(),
                is_enabled: true,
                created_at: now + 1,
                updated_at: now + 1,
            })
            .expect("low translation rule should persist");

        let translation_preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);
        let qa_preview = build_preview(&runtime, RULE_ACTION_SCOPE_QA);

        assert_eq!(translation_preview.rules.len(), 3);
        assert_eq!(
            translation_preview
                .rules
                .iter()
                .map(|rule| rule.rule.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Keep command names stable",
                "Keep list formatting stable",
                "Prefer recurring noun phrases",
            ]
        );
        assert_eq!(qa_preview.rules.len(), 1);
        assert_eq!(qa_preview.rules[0].rule.name, "QA-only rule");
    }

    #[test]
    fn build_translation_context_deduplicates_unicode_glossary_terms() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_800_000_150_i64;

        GlossaryEntryRepository::new(&mut connection)
            .create(&NewGlossaryEntry {
                id: "gle_global_unicode_001".to_owned(),
                glossary_id: "gls_global_001".to_owned(),
                source_term: "ÓRDEN".to_owned(),
                target_term: "Orden global".to_owned(),
                context_note: None,
                status: GLOSSARY_ENTRY_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                source_variants: vec![],
                target_variants: vec![],
                forbidden_terms: vec![],
            })
            .expect("global unicode glossary entry should persist");
        GlossaryEntryRepository::new(&mut connection)
            .create(&NewGlossaryEntry {
                id: "gle_project_unicode_001".to_owned(),
                glossary_id: "gls_project_001".to_owned(),
                source_term: "órden".to_owned(),
                target_term: "Mandato unicode".to_owned(),
                context_note: None,
                status: GLOSSARY_ENTRY_STATUS_ACTIVE.to_owned(),
                created_at: now + 1,
                updated_at: now + 1,
                source_variants: vec![],
                target_variants: vec![],
                forbidden_terms: vec![],
            })
            .expect("project unicode glossary entry should persist");
        drop(connection);

        let preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);

        assert_eq!(preview.glossary_entries.len(), 2);
        assert!(
            preview
                .glossary_entries
                .iter()
                .any(|entry| entry.entry.target_term == "Mandato unicode")
        );
        assert!(
            !preview
                .glossary_entries
                .iter()
                .any(|entry| entry.entry.target_term == "Orden global")
        );
    }

    #[test]
    fn build_translation_context_does_not_persist_rebuilt_sections() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");

        DocumentSectionRepository::new(&mut connection)
            .replace_for_document("doc_chunk_001", &[])
            .expect("sections should be cleared");
        assert!(
            DocumentSectionRepository::new(&mut connection)
                .list_by_document("doc_chunk_001")
                .expect("sections should list")
                .is_empty()
        );
        drop(connection);

        let preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);

        assert_eq!(
            preview
                .chunk_context
                .section
                .as_ref()
                .map(|section| section.title.as_str()),
            Some("Chapter 1")
        );

        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        assert!(
            DocumentSectionRepository::new(&mut connection)
                .list_by_document("doc_chunk_001")
                .expect("sections should remain unpersisted")
                .is_empty()
        );
    }

    #[test]
    fn build_translation_context_composes_chunk_and_accumulated_context() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);

        let preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);

        assert_eq!(preview.chunk_context.chunk.id, "doc_chunk_001_chunk_0001");
        assert_eq!(preview.chunk_context.core_segments.len(), 2);
        assert_eq!(preview.chunk_context.context_before_segments.len(), 1);
        assert_eq!(preview.chunk_context.context_after_segments.len(), 1);
        assert_eq!(
            preview
                .chunk_context
                .section
                .as_ref()
                .map(|section| section.id.as_str()),
            Some("doc_chunk_001_sec_0001")
        );
        assert_eq!(preview.accumulated_contexts.len(), 2);
        assert_eq!(
            preview.accumulated_contexts[0].match_reason,
            CHAPTER_CONTEXT_MATCH_SECTION
        );
        assert_eq!(
            preview.accumulated_contexts[1].match_reason,
            CHAPTER_CONTEXT_MATCH_DOCUMENT
        );
    }

    #[test]
    fn build_translation_context_ignores_section_contexts_without_range_overlap() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_800_000_300_i64;

        ChapterContextRepository::new(&mut connection)
            .replace_for_document(
                "doc_chunk_001",
                &[
                    NewChapterContext {
                        id: "ctx_section_overlap".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        section_id: Some("doc_chunk_001_sec_0001".to_owned()),
                        task_run_id: None,
                        scope_type: "chapter".to_owned(),
                        start_segment_sequence: 2,
                        end_segment_sequence: 3,
                        context_text: "Overlapping section context.".to_owned(),
                        source_summary: None,
                        context_word_count: 3,
                        context_character_count: 28,
                        created_at: now,
                        updated_at: now,
                    },
                    NewChapterContext {
                        id: "ctx_section_stale".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        section_id: Some("doc_chunk_001_sec_0001".to_owned()),
                        task_run_id: None,
                        scope_type: "chapter".to_owned(),
                        start_segment_sequence: 4,
                        end_segment_sequence: 4,
                        context_text: "Stale section context.".to_owned(),
                        source_summary: None,
                        context_word_count: 3,
                        context_character_count: 22,
                        created_at: now + 1,
                        updated_at: now + 1,
                    },
                ],
            )
            .expect("chapter contexts should persist");
        drop(connection);

        let preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);

        assert_eq!(
            preview
                .accumulated_contexts
                .iter()
                .map(|context| context.chapter_context.id.as_str())
                .collect::<Vec<_>>(),
            vec!["ctx_section_overlap"]
        );
        assert_eq!(
            preview.accumulated_contexts[0].match_reason,
            CHAPTER_CONTEXT_MATCH_SECTION
        );
    }

    #[test]
    fn build_translation_context_is_deterministic() {
        let (_temp_dir, runtime) = create_runtime();
        seed_context_builder_graph(&runtime);

        let first_preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);
        let second_preview = build_preview(&runtime, RULE_ACTION_SCOPE_TRANSLATION);

        assert_eq!(first_preview, second_preview);
        assert_eq!(
            first_preview.resolution.chapter_context_ids,
            vec!["ctx_section_001".to_owned(), "ctx_document_001".to_owned()]
        );
    }
}
