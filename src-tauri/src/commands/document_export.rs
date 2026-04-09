use std::path::Path;

use tauri::State;

use crate::commands::reconstructed_documents::{
    current_timestamp, load_reconstructed_document, validate_identifier,
};
use crate::document_export::{
    ExportReconstructedDocumentInput, ExportReconstructedDocumentResult,
};
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::documents::DocumentRepository;
use crate::reconstructed_documents::{ReconstructedDocument, ReconstructedDocumentBlock};

const DOCUMENT_EXPORT_FORMAT_MARKDOWN: &str = "md";
const DOCUMENT_EXPORT_MIME_MARKDOWN: &str = "text/markdown; charset=utf-8";

#[tauri::command]
pub fn export_reconstructed_document(
    input: ExportReconstructedDocumentInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<ExportReconstructedDocumentResult, DesktopCommandError> {
    export_reconstructed_document_with_runtime(input, database_runtime.inner())
}

pub(crate) fn export_reconstructed_document_with_runtime(
    input: ExportReconstructedDocumentInput,
    database_runtime: &DatabaseRuntime,
) -> Result<ExportReconstructedDocumentResult, DesktopCommandError> {
    let exported_at = current_timestamp()?;

    export_reconstructed_document_with_runtime_at(input, database_runtime, exported_at)
}

fn export_reconstructed_document_with_runtime_at(
    input: ExportReconstructedDocumentInput,
    database_runtime: &DatabaseRuntime,
    exported_at: i64,
) -> Result<ExportReconstructedDocumentResult, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for document export.",
            Some(error.to_string()),
        )
    })?;
    let document = DocumentRepository::new(&mut connection)
        .load_processing_record(&project_id, &document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not validate the selected document for export.",
                Some(error.to_string()),
            )
        })?
        .ok_or_else(|| {
            DesktopCommandError::validation(
                "The selected document does not exist in the active project.",
                None,
            )
        })?;
    let reconstructed_document = load_reconstructed_document(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        exported_at,
    )?;

    if !reconstructed_document
        .completeness
        .has_reconstructible_content
    {
        return Err(DesktopCommandError::validation(
            "The selected document does not contain reconstructible content to export yet.",
            None,
        ));
    }

    Ok(build_export_result(
        reconstructed_document,
        &document.name,
        exported_at,
    ))
}

fn build_export_result(
    reconstructed_document: ReconstructedDocument,
    document_name: &str,
    exported_at: i64,
) -> ExportReconstructedDocumentResult {
    let content = build_markdown_export(document_name, &reconstructed_document, exported_at);

    ExportReconstructedDocumentResult {
        project_id: reconstructed_document.project_id.clone(),
        document_id: reconstructed_document.document_id.clone(),
        document_name: document_name.to_owned(),
        format: DOCUMENT_EXPORT_FORMAT_MARKDOWN.to_owned(),
        mime_type: DOCUMENT_EXPORT_MIME_MARKDOWN.to_owned(),
        file_name: build_export_file_name(document_name),
        exported_at,
        status: reconstructed_document.status.clone(),
        content_source: reconstructed_document.content_source.clone(),
        is_complete: reconstructed_document.completeness.is_complete,
        total_segments: reconstructed_document.completeness.total_segments,
        translated_segments: reconstructed_document.completeness.translated_segments,
        fallback_segments: reconstructed_document.completeness.fallback_segments,
        content,
    }
}

fn build_markdown_export(
    document_name: &str,
    reconstructed_document: &ReconstructedDocument,
    exported_at: i64,
) -> String {
    let mut sections = vec![
        format!("# {document_name}"),
        String::new(),
        "Exported from the current reconstructed document snapshot in Translat.".to_owned(),
        String::new(),
        format!("- Project ID: {}", reconstructed_document.project_id),
        format!("- Document ID: {}", reconstructed_document.document_id),
        format!("- Export format: {DOCUMENT_EXPORT_FORMAT_MARKDOWN}"),
        format!("- Exported at (unix): {exported_at}"),
        format!("- Reconstruction status: {}", reconstructed_document.status),
        format!(
            "- Content source: {}",
            reconstructed_document.content_source
        ),
        format!(
            "- Completeness: {}/{} translated segments",
            reconstructed_document.completeness.translated_segments,
            reconstructed_document.completeness.total_segments
        ),
    ];

    if reconstructed_document.completeness.fallback_segments > 0 {
        sections.push(format!(
            "- Fallback segments: {}",
            reconstructed_document.completeness.fallback_segments
        ));
    }

    if reconstructed_document.completeness.fallback_segments > 0 {
        sections.push(String::new());
        sections.push(format!(
            "> Warning: This export still contains {} source-fallback segment(s).",
            reconstructed_document.completeness.fallback_segments
        ));
    }

    let rendered_blocks = reconstructed_document
        .blocks
        .iter()
        .map(render_markdown_block)
        .filter(|block| !block.is_empty())
        .collect::<Vec<_>>();

    if !rendered_blocks.is_empty() {
        sections.push(String::new());
        sections.push(rendered_blocks.join("\n\n"));
    } else if !reconstructed_document.resolved_text.trim().is_empty() {
        sections.push(String::new());
        sections.push(reconstructed_document.resolved_text.clone());
    }

    sections.join("\n")
}

fn render_markdown_block(block: &ReconstructedDocumentBlock) -> String {
    let (heading, body_segments) = if let Some(title) = block.title.as_ref() {
        if let Some(first_segment) = block.segments.first() {
            if title_matches_segment_title(title, &first_segment.source_text) {
                (Some(first_segment.resolved_text.clone()), &block.segments[1..])
            } else {
                (Some(title.clone()), block.segments.as_slice())
            }
        } else {
            (Some(title.clone()), &[][..])
        }
    } else {
        (None, block.segments.as_slice())
    };

    let mut parts = Vec::new();

    if let Some(heading) = heading {
        parts.push(format!("{} {}", markdown_heading_prefix(block.level), heading));
    }

    let body = body_segments
        .iter()
        .map(render_markdown_segment)
        .filter(|segment| !segment.trim().is_empty())
        .collect::<Vec<_>>();

    if !body.is_empty() {
        parts.push(body.join("\n\n"));
    } else if parts.is_empty() && !block.resolved_text.trim().is_empty() {
        parts.push(block.resolved_text.clone());
    }

    parts.join("\n\n")
}

fn render_markdown_segment(
    segment: &crate::reconstructed_documents::ReconstructedSegment,
) -> String {
    if segment.final_text.is_some() {
        segment.resolved_text.clone()
    } else {
        format!("[Source fallback] {}", segment.resolved_text)
    }
}

fn markdown_heading_prefix(level: Option<i64>) -> String {
    let heading_level = match level.unwrap_or(1) {
        level if level <= 1 => 2,
        level if level >= 5 => 6,
        level => usize::try_from(level + 1).unwrap_or(2),
    };

    "#".repeat(heading_level)
}

fn title_matches_segment_title(section_title: &str, source_text: &str) -> bool {
    normalize_for_heading_match(section_title) == normalize_for_heading_match(source_text)
}

fn normalize_for_heading_match(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn build_export_file_name(document_name: &str) -> String {
    let stem = Path::new(document_name)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(document_name);
    let sanitized: String = stem
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => character,
            ' ' | '.' => '_',
            _ => '_',
        })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    let base_name = if trimmed.is_empty() { "document" } else { trimmed };
    let shortened: String = base_name.chars().take(120).collect();

    format!("{shortened}.translated.{DOCUMENT_EXPORT_FORMAT_MARKDOWN}")
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, TempDir};

    use super::export_reconstructed_document_with_runtime_at;
    use crate::document_export::ExportReconstructedDocumentInput;
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_IMPORTED};
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::persistence::sections::DocumentSectionRepository;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::task_runs::TaskRunRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::NewProject;
    use crate::sections::{NewDocumentSection, DOCUMENT_SECTION_TYPE_CHAPTER};
    use crate::segments::{NewSegment, SegmentTranslationWrite, SEGMENT_STATUS_PENDING_TRANSLATION};
    use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_COMPLETED, TASK_RUN_STATUS_RUNNING};
    use crate::translate_chunk::TRANSLATE_CHUNK_ACTION_TYPE;
    use crate::translate_document::TRANSLATE_DOCUMENT_ACTION_TYPE;
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const PROJECT_ID: &str = "prj_export_001";
    const DOCUMENT_ID: &str = "doc_export_001";
    const NOW: i64 = 1_900_200_000;

    struct RuntimeFixture {
        _temporary_directory: TempDir,
        runtime: DatabaseRuntime,
    }

    fn create_runtime_fixture() -> RuntimeFixture {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key =
            load_or_create_encryption_key(&encryption_key_path).expect("key should persist");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        RuntimeFixture {
            _temporary_directory: temporary_directory,
            runtime,
        }
    }

    fn seed_export_graph(runtime: &DatabaseRuntime) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: PROJECT_ID.to_owned(),
                name: "Export project".to_owned(),
                description: None,
                created_at: NOW,
                updated_at: NOW,
                last_opened_at: NOW,
            })
            .expect("project should persist");
        ProjectRepository::new(&mut connection)
            .open_project(PROJECT_ID, NOW)
            .expect("project should become active");

        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: DOCUMENT_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
                name: "chaptered draft.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "ignored".to_owned(),
                file_size_bytes: 1_024,
                status: DOCUMENT_STATUS_IMPORTED.to_owned(),
                created_at: NOW,
                updated_at: NOW,
            })
            .expect("document should persist");

        SegmentRepository::new(&mut connection)
            .replace_for_document(
                PROJECT_ID,
                DOCUMENT_ID,
                &[
                    NewSegment {
                        id: "seg_export_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        source_text: "Chapter I".to_owned(),
                        source_word_count: 2,
                        source_character_count: 9,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_export_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        source_text: "The gate remained closed.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 25,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_export_0003".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 3,
                        source_text: "Chapter II".to_owned(),
                        source_word_count: 2,
                        source_character_count: 10,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_export_0004".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 4,
                        source_text: "The lantern burned all night.".to_owned(),
                        source_word_count: 5,
                        source_character_count: 29,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                ],
                NOW,
            )
            .expect("segments should persist");

        DocumentSectionRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[
                    NewDocumentSection {
                        id: "doc_export_001_sec_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        title: "Chapter I".to_owned(),
                        section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                        level: 1,
                        start_segment_sequence: 1,
                        end_segment_sequence: 2,
                        segment_count: 2,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewDocumentSection {
                        id: "doc_export_001_sec_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        title: "Chapter II".to_owned(),
                        section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                        level: 1,
                        start_segment_sequence: 3,
                        end_segment_sequence: 4,
                        segment_count: 2,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                ],
            )
            .expect("sections should persist");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[
                    NewTranslationChunk {
                        id: "doc_export_001_chunk_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "Chapter I\n\nThe gate remained closed.".to_owned(),
                        context_before_text: None,
                        context_after_text: Some("Chapter II".to_owned()),
                        start_segment_sequence: 1,
                        end_segment_sequence: 2,
                        segment_count: 2,
                        source_word_count: 6,
                        source_character_count: 34,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewTranslationChunk {
                        id: "doc_export_001_chunk_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "Chapter II\n\nThe lantern burned all night.".to_owned(),
                        context_before_text: None,
                        context_after_text: None,
                        start_segment_sequence: 3,
                        end_segment_sequence: 4,
                        segment_count: 2,
                        source_word_count: 7,
                        source_character_count: 39,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                ],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_export_001_chunk_0001".to_owned(),
                        segment_id: "seg_export_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_export_001_chunk_0001".to_owned(),
                        segment_id: "seg_export_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_export_001_chunk_0001".to_owned(),
                        segment_id: "seg_export_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_export_001_chunk_0002".to_owned(),
                        segment_id: "seg_export_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_export_001_chunk_0002".to_owned(),
                        segment_id: "seg_export_0004".to_owned(),
                        segment_sequence: 4,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                ],
            )
            .expect("chunks should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_export_doc_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                job_id: Some("job_export_doc_001".to_owned()),
                action_type: TRANSLATE_DOCUMENT_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: Some("{\"job\":\"translate\"}".to_owned()),
                output_payload: Some("{\"status\":\"completed\"}".to_owned()),
                error_message: None,
                started_at: NOW,
                completed_at: Some(NOW + 60),
                created_at: NOW,
                updated_at: NOW + 60,
            })
            .expect("document task run should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_export_chunk_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_export_001_chunk_0001".to_owned()),
                job_id: Some("job_export_doc_001".to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":1}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 1,
                completed_at: None,
                created_at: NOW + 1,
                updated_at: NOW + 1,
            })
            .expect("first chunk task run should persist");

        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_export_chunk_0001",
                "{\"translations\":[1,2]}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_export_0001".to_owned(),
                        target_text: "Capítulo I".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_export_0002".to_owned(),
                        target_text: "La puerta siguió cerrada.".to_owned(),
                    },
                ],
                NOW + 30,
            )
            .expect("first chunk translation projection should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_export_chunk_0002".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_export_001_chunk_0002".to_owned()),
                job_id: Some("job_export_doc_001".to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":2}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 31,
                completed_at: None,
                created_at: NOW + 31,
                updated_at: NOW + 31,
            })
            .expect("second chunk task run should persist");
    }

    #[test]
    fn export_reconstructed_document_rejects_invalid_identifiers() {
        let fixture = create_runtime_fixture();

        let error = export_reconstructed_document_with_runtime_at(
            ExportReconstructedDocumentInput {
                project_id: " ".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
            NOW + 90,
        )
        .expect_err("invalid ids should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("project id"));
    }

    #[test]
    fn export_reconstructed_document_rejects_unknown_documents() {
        let fixture = create_runtime_fixture();
        seed_export_graph(&fixture.runtime);

        let error = export_reconstructed_document_with_runtime_at(
            ExportReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: "doc_missing_001".to_owned(),
            },
            &fixture.runtime,
            NOW + 90,
        )
        .expect_err("unknown documents should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("does not exist"));
    }

    #[test]
    fn export_reconstructed_document_returns_markdown_for_partial_documents() {
        let fixture = create_runtime_fixture();
        seed_export_graph(&fixture.runtime);

        let result = export_reconstructed_document_with_runtime_at(
            ExportReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
            NOW + 90,
        )
        .expect("partial reconstructed documents should export");

        assert_eq!(result.document_name, "chaptered draft.txt");
        assert_eq!(result.file_name, "chaptered_draft.translated.md");
        assert_eq!(result.format, "md");
        assert_eq!(result.status, "partial");
        assert!(!result.is_complete);
        assert_eq!(result.fallback_segments, 2);
        assert!(result.content.contains("# chaptered draft.txt"));
        assert!(result.content.contains("## Capítulo I"));
        assert!(result.content.contains("La puerta siguió cerrada."));
        assert!(result.content.contains("## Chapter II"));
        assert!(result.content.contains("[Source fallback] The lantern burned all night."));
        assert!(result.content.contains("> Warning: This export still contains 2 source-fallback segment(s)."));
    }

    #[test]
    fn export_reconstructed_document_is_stable_for_a_fixed_timestamp() {
        let fixture = create_runtime_fixture();
        seed_export_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_export_chunk_0002",
                "{\"translations\":[3,4]}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_export_0003".to_owned(),
                        target_text: "Capítulo II".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_export_0004".to_owned(),
                        target_text: "La linterna ardió toda la noche.".to_owned(),
                    },
                ],
                NOW + 61,
            )
            .expect("second chunk translation projection should persist");
        drop(connection);

        let first = export_reconstructed_document_with_runtime_at(
            ExportReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
            NOW + 120,
        )
        .expect("complete reconstructed documents should export");
        let second = export_reconstructed_document_with_runtime_at(
            ExportReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
            NOW + 120,
        )
        .expect("exports should stay deterministic");

        assert_eq!(first, second);
        assert_eq!(first.status, "complete");
        assert!(first.is_complete);
        assert_eq!(first.fallback_segments, 0);
        assert!(first.content.contains("## Capítulo II"));
        assert!(first.content.contains("La linterna ardió toda la noche."));
        assert!(!first.content.contains("[Source fallback]"));
    }
}
