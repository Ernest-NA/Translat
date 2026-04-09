use std::collections::BTreeSet;
use std::path::Path;

use serde_json::json;
use tauri::State;

use crate::commands::reconstructed_documents::{
    current_timestamp, load_reconstructed_document, validate_identifier,
};
use crate::commands::translate_document_jobs::generate_task_run_id;
use crate::document_export::{
    ExportReconstructedDocumentInput, ExportReconstructedDocumentResult,
    EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE, EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_VERSION,
};
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::documents::DocumentRepository;
use crate::persistence::qa_findings::QaFindingRepository;
use crate::persistence::segments::SegmentRepository;
use crate::persistence::task_runs::TaskRunRepository;
use crate::qa_findings::QA_FINDING_STATUS_OPEN;
use crate::reconstructed_documents::{ReconstructedDocument, ReconstructedDocumentBlock};
use crate::segments::SegmentTranslationTraceSummary;
use crate::task_runs::{
    NewTaskRun, TaskRunSummary, TASK_RUN_STATUS_COMPLETED, TASK_RUN_STATUS_RUNNING,
};

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
    let open_finding_count = i64::try_from(
        QaFindingRepository::new(&mut connection)
            .list_by_document(&document_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not load QA findings while exporting the reconstructed document.",
                    Some(error.to_string()),
                )
            })?
            .into_iter()
            .filter(|finding| finding.status == QA_FINDING_STATUS_OPEN)
            .count(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid QA finding count while exporting the reconstructed document.",
            Some(error.to_string()),
        )
    })?;
    let segment_translation_traces = SegmentRepository::new(&mut connection)
        .list_translation_trace_by_document(&document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load segment translation provenance for document export.",
                Some(error.to_string()),
            )
        })?;
    let document_task_runs = TaskRunRepository::new(&mut connection)
        .list_by_document(&document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load task runs for export provenance.",
                Some(error.to_string()),
            )
        })?;
    let export_source_task_run =
        select_export_source_task_run(&segment_translation_traces, &document_task_runs);
    let task_run_id = generate_task_run_id(exported_at);
    let export_file_name = build_export_file_name(&document.name);
    let source_job_id = export_source_task_run
        .as_ref()
        .and_then(|task_run| task_run.job_id.clone());
    let source_task_run_id = export_source_task_run.map(|task_run| task_run.id);
    let input_payload = serde_json::to_string(&json!({
        "actionVersion": EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_VERSION,
        "documentName": document.name.as_str(),
        "format": DOCUMENT_EXPORT_FORMAT_MARKDOWN,
        "fileName": export_file_name,
        "exportedAt": exported_at,
        "reconstructedStatus": reconstructed_document.status.as_str(),
        "contentSource": reconstructed_document.content_source.as_str(),
        "isComplete": reconstructed_document.completeness.is_complete,
        "totalSegments": reconstructed_document.completeness.total_segments,
        "translatedSegments": reconstructed_document.completeness.translated_segments,
        "fallbackSegments": reconstructed_document.completeness.fallback_segments,
        "openFindingCount": open_finding_count,
        "sourceJobId": source_job_id,
        "sourceTaskRunId": source_task_run_id,
    }))
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not prepare the export trace payload.",
            Some(error.to_string()),
        )
    })?;

    TaskRunRepository::new(&mut connection)
        .create(&NewTaskRun {
            id: task_run_id.clone(),
            document_id: document_id.clone(),
            chunk_id: None,
            job_id: None,
            action_type: EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE.to_owned(),
            status: TASK_RUN_STATUS_RUNNING.to_owned(),
            input_payload: Some(input_payload),
            output_payload: None,
            error_message: None,
            started_at: exported_at,
            completed_at: None,
            created_at: exported_at,
            updated_at: exported_at,
        })
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not persist the export task run.",
                Some(error.to_string()),
            )
        })?;

    if !reconstructed_document
        .completeness
        .has_reconstructible_content
    {
        let validation_payload = serde_json::to_string(&json!({
            "actionVersion": EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_VERSION,
            "outcome": "validation_error",
        }))
        .unwrap_or_else(|_| "{\"outcome\":\"validation_error\"}".to_owned());

        let _ = TaskRunRepository::new(&mut connection).mark_failed(
            &task_run_id,
            "The selected document does not contain reconstructible content to export yet.",
            Some(&validation_payload),
            exported_at,
        );

        return Err(DesktopCommandError::validation(
            "The selected document does not contain reconstructible content to export yet.",
            None,
        ));
    }

    let result = build_export_result(
        reconstructed_document,
        &document.name,
        exported_at,
    );
    let output_payload = serde_json::to_string(&json!({
        "actionVersion": EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_VERSION,
        "fileName": result.file_name.as_str(),
        "format": result.format.as_str(),
        "mimeType": result.mime_type.as_str(),
        "contentBytes": result.content.len(),
        "status": result.status.as_str(),
        "contentSource": result.content_source.as_str(),
        "isComplete": result.is_complete,
    }))
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not prepare the export completion payload.",
            Some(error.to_string()),
        )
    })?;

    TaskRunRepository::new(&mut connection)
        .mark_completed(&task_run_id, &output_payload, exported_at)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not finalize the export task run.",
                Some(error.to_string()),
            )
        })?;

    Ok(result)
}

fn select_export_source_task_run(
    segment_translation_traces: &[SegmentTranslationTraceSummary],
    document_task_runs: &[TaskRunSummary],
) -> Option<TaskRunSummary> {
    let contributing_task_run_ids = segment_translation_traces
        .iter()
        .filter(|segment_trace| segment_trace.target_text.is_some())
        .filter_map(|segment_trace| segment_trace.last_task_run_id.as_deref())
        .collect::<BTreeSet<_>>();

    if contributing_task_run_ids.is_empty() {
        return None;
    }

    let mut selected_task_run: Option<&TaskRunSummary> = None;
    let mut selected_sort_key = i64::MIN;
    let mut selected_is_ambiguous = false;

    for task_run_id in contributing_task_run_ids {
        let Some(task_run) = document_task_runs.iter().find(|task_run| {
            task_run.id == task_run_id && task_run.status == TASK_RUN_STATUS_COMPLETED
        }) else {
            continue;
        };
        let sort_key = task_run.completed_at.unwrap_or(task_run.updated_at);

        if sort_key > selected_sort_key {
            selected_task_run = Some(task_run);
            selected_sort_key = sort_key;
            selected_is_ambiguous = false;
        } else if sort_key == selected_sort_key
            && selected_task_run
                .as_ref()
                .is_some_and(|selected| selected.id != task_run.id)
        {
            selected_is_ambiguous = true;
        }
    }

    if selected_is_ambiguous {
        None
    } else {
        selected_task_run.cloned()
    }
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
        format!("# {}", escape_markdown_heading_text(document_name)),
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
        sections.push(escape_markdown_paragraph_text(
            reconstructed_document.resolved_text.as_str(),
        ));
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
        parts.push(format!(
            "{} {}",
            markdown_heading_prefix(block.level),
            escape_markdown_heading_text(heading.as_str())
        ));
    }

    let body = body_segments
        .iter()
        .map(render_markdown_segment)
        .filter(|segment| !segment.trim().is_empty())
        .collect::<Vec<_>>();

    if !body.is_empty() {
        parts.push(body.join("\n\n"));
    } else if parts.is_empty() && !block.resolved_text.trim().is_empty() {
        parts.push(escape_markdown_paragraph_text(block.resolved_text.as_str()));
    }

    parts.join("\n\n")
}

fn render_markdown_segment(
    segment: &crate::reconstructed_documents::ReconstructedSegment,
) -> String {
    if segment.final_text.is_some() {
        escape_markdown_paragraph_text(segment.resolved_text.as_str())
    } else {
        format!(
            "[Source fallback] {}",
            escape_markdown_paragraph_text(segment.resolved_text.as_str())
        )
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
        .chars()
        .filter_map(|character| {
            if character.is_alphanumeric() {
                Some(character.to_ascii_lowercase())
            } else if character.is_whitespace() {
                Some(' ')
            } else {
                None
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_markdown_heading_text(value: &str) -> String {
    escape_markdown_text(value, true)
}

fn escape_markdown_paragraph_text(value: &str) -> String {
    escape_markdown_text(value, false)
}

fn escape_markdown_text(value: &str, collapse_newlines: bool) -> String {
    let normalized = if collapse_newlines {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        value.replace("\r\n", "\n")
    };

    normalized
        .lines()
        .map(escape_markdown_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_markdown_line(line: &str) -> String {
    let mut escaped = String::new();
    let trimmed = line.trim_start();
    let leading_whitespace_len = line.len().saturating_sub(trimmed.len());
    let leading_whitespace = &line[..leading_whitespace_len];

    escaped.push_str(leading_whitespace);

    if should_escape_markdown_line_prefix(trimmed) {
        escaped.push('\\');
    }

    for character in trimmed.chars() {
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '!' | '>' | '|'
        ) {
            escaped.push('\\');
        }

        escaped.push(character);
    }

    escaped
}

fn should_escape_markdown_line_prefix(line: &str) -> bool {
    let Some(first_character) = line.chars().next() else {
        return false;
    };

    if matches!(first_character, '#' | '>' | '-' | '+' | '*') {
        return true;
    }

    let mut digit_count = 0_usize;

    for character in line.chars() {
        if character.is_ascii_digit() {
            digit_count += 1;
            continue;
        }

        return digit_count > 0 && matches!(character, '.' | ')');
    }

    false
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
    let safe_base_name = if is_windows_reserved_file_name(&shortened) {
        format!("document_{shortened}")
    } else {
        shortened
    };

    format!("{safe_base_name}.translated.{DOCUMENT_EXPORT_FORMAT_MARKDOWN}")
}

fn is_windows_reserved_file_name(file_name: &str) -> bool {
    let stem = file_name
        .split('.')
        .next()
        .unwrap_or(file_name)
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();

    matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use tempfile::{tempdir, TempDir};

    use super::{
        build_export_file_name, escape_markdown_heading_text,
        escape_markdown_paragraph_text, export_reconstructed_document_with_runtime_at,
        select_export_source_task_run, title_matches_segment_title,
    };
    use crate::document_export::{
        ExportReconstructedDocumentInput, EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE,
    };
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
    use crate::segments::{
        NewSegment, SegmentTranslationTraceSummary, SegmentTranslationWrite,
        SEGMENT_STATUS_PENDING_TRANSLATION,
    };
    use crate::task_runs::{
        NewTaskRun, TaskRunSummary, TASK_RUN_STATUS_COMPLETED, TASK_RUN_STATUS_RUNNING,
    };
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

    #[test]
    fn export_reconstructed_document_attributes_trace_to_latest_contributing_chunk_run() {
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

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_export_chunk_0003".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_export_001_chunk_0002".to_owned()),
                job_id: Some("job_export_retranslate_001".to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":2,\"mode\":\"review_retranslation\"}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 91,
                completed_at: None,
                created_at: NOW + 91,
                updated_at: NOW + 91,
            })
            .expect("retranslation task run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_export_chunk_0003",
                "{\"translations\":[3,4],\"review\":\"applied\"}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_export_0003".to_owned(),
                        target_text: "Capítulo II".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_export_0004".to_owned(),
                        target_text: "La linterna siguió ardiendo toda la noche.".to_owned(),
                    },
                ],
                NOW + 101,
            )
            .expect("chunk-only retranslation should persist");
        drop(connection);

        let result = export_reconstructed_document_with_runtime_at(
            ExportReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
            NOW + 120,
        )
        .expect("export should succeed after chunk-only retranslation");

        assert!(result
            .content
            .contains("La linterna siguió ardiendo toda la noche."));

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let export_task_run = TaskRunRepository::new(&mut connection)
            .list_by_document(DOCUMENT_ID)
            .expect("task runs should load")
            .into_iter()
            .find(|task_run| task_run.action_type == EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE)
            .expect("export task run should persist");
        let export_input_payload: Value = serde_json::from_str(
            export_task_run
                .input_payload
                .as_deref()
                .expect("export task run should persist an input payload"),
        )
        .expect("export payload should decode");

        assert_eq!(
            export_input_payload["sourceJobId"].as_str(),
            Some("job_export_retranslate_001")
        );
        assert_eq!(
            export_input_payload["sourceTaskRunId"].as_str(),
            Some("task_export_chunk_0003")
        );
    }

    #[test]
    fn export_source_task_run_is_unset_when_latest_segment_contributors_tie() {
        let segment_translation_traces = vec![
            SegmentTranslationTraceSummary {
                id: "seg_001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                sequence: 1,
                target_text: Some("Uno".to_owned()),
                last_task_run_id: Some("task_chunk_a".to_owned()),
            },
            SegmentTranslationTraceSummary {
                id: "seg_002".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                sequence: 2,
                target_text: Some("Dos".to_owned()),
                last_task_run_id: Some("task_chunk_b".to_owned()),
            },
        ];
        let document_task_runs = vec![
            TaskRunSummary {
                id: "task_chunk_a".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("chunk_a".to_owned()),
                job_id: Some("job_a".to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: None,
                output_payload: None,
                error_message: None,
                started_at: NOW,
                completed_at: Some(NOW + 10),
                created_at: NOW,
                updated_at: NOW + 10,
            },
            TaskRunSummary {
                id: "task_chunk_b".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("chunk_b".to_owned()),
                job_id: Some("job_b".to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: None,
                output_payload: None,
                error_message: None,
                started_at: NOW + 1,
                completed_at: Some(NOW + 10),
                created_at: NOW + 1,
                updated_at: NOW + 10,
            },
        ];

        assert!(
            select_export_source_task_run(&segment_translation_traces, &document_task_runs)
                .is_none()
        );
    }

    #[test]
    fn build_export_file_name_avoids_windows_reserved_names() {
        assert_eq!(
            build_export_file_name("CON.txt"),
            "document_CON.translated.md"
        );
        assert_eq!(
            build_export_file_name("AUX"),
            "document_AUX.translated.md"
        );
        assert_eq!(
            build_export_file_name("nul"),
            "document_nul.translated.md"
        );
    }

    #[test]
    fn title_matching_ignores_common_punctuation_and_spacing_variations() {
        assert!(title_matches_segment_title("Chapter I:", "Chapter I"));
        assert!(title_matches_segment_title("Capítulo II", "Capítulo II."));
        assert!(title_matches_segment_title("Parte 3 - Final", "Parte 3 Final"));
        assert!(!title_matches_segment_title("Chapter I", "Chapter II"));
    }

    #[test]
    fn markdown_export_escapes_heading_and_paragraph_control_syntax() {
        assert_eq!(
            escape_markdown_heading_text("# Chapter [I]"),
            "\\\\# Chapter \\[I\\]"
        );
        assert_eq!(
            escape_markdown_paragraph_text("> quoted\n1. item\n_regular_ [link](url)"),
            "\\\\> quoted\n\\1. item\n\\_regular\\_ \\[link\\]\\(url\\)"
        );
    }
}
