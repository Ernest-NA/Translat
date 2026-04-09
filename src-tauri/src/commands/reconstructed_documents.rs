use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::commands::segments::load_segmented_document_overview;
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::task_runs::TaskRunRepository;
use crate::persistence::translation_chunks::TranslationChunkRepository;
use crate::reconstructed_documents::{
    GetReconstructedDocumentInput, ReconstructedDocument, ReconstructedDocumentBlock,
    ReconstructedDocumentChunkTrace, ReconstructedDocumentCompleteness,
    ReconstructedDocumentSection, ReconstructedDocumentTrace, ReconstructedSegment,
    RECONSTRUCTED_CONTENT_SOURCE_MIXED, RECONSTRUCTED_CONTENT_SOURCE_NONE,
    RECONSTRUCTED_CONTENT_SOURCE_SOURCE_FALLBACK, RECONSTRUCTED_CONTENT_SOURCE_TARGET,
    RECONSTRUCTED_DOCUMENT_STATUS_COMPLETE, RECONSTRUCTED_DOCUMENT_STATUS_EMPTY,
    RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL, RECONSTRUCTED_DOCUMENT_STATUS_UNTRANSLATED,
};
use crate::sections::DocumentSectionSummary;
use crate::segments::SegmentSummary;
use crate::task_runs::TaskRunSummary;
use crate::translate_chunk::TRANSLATE_CHUNK_ACTION_TYPE;
use crate::translate_document::TRANSLATE_DOCUMENT_ACTION_TYPE;
use crate::translation_chunks::{
    TranslationChunkSegmentSummary, TranslationChunkSummary,
    TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER, TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE,
    TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
};

#[tauri::command]
pub fn get_reconstructed_document(
    input: GetReconstructedDocumentInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<ReconstructedDocument, DesktopCommandError> {
    get_reconstructed_document_with_runtime(input, database_runtime.inner())
}

pub(crate) fn get_reconstructed_document_with_runtime(
    input: GetReconstructedDocumentInput,
    database_runtime: &DatabaseRuntime,
) -> Result<ReconstructedDocument, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let reconstructed_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for document reconstruction.",
            Some(error.to_string()),
        )
    })?;
    load_reconstructed_document(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        reconstructed_at,
    )
}

pub(crate) fn load_reconstructed_document(
    connection: &mut rusqlite::Connection,
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    reconstructed_at: i64,
) -> Result<ReconstructedDocument, DesktopCommandError> {
    let segment_overview = load_segmented_document_overview(
        connection,
        database_runtime,
        project_id,
        document_id,
        false,
        reconstructed_at,
    )?;
    let mut chunk_repository = TranslationChunkRepository::new(connection);
    let chunks = chunk_repository
        .list_chunks_by_document(document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load translation chunks for reconstruction.",
                Some(error.to_string()),
            )
        })?;
    let chunk_segments = chunk_repository
        .list_chunk_segments_by_document(document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load translation chunk links for reconstruction.",
                Some(error.to_string()),
            )
        })?;
    let task_runs = TaskRunRepository::new(connection)
        .list_by_document(document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load task runs for reconstruction.",
                Some(error.to_string()),
            )
        })?;

    Ok(build_reconstructed_document(
        project_id,
        document_id,
        &segment_overview.sections,
        &segment_overview.segments,
        &chunks,
        &chunk_segments,
        &task_runs,
    ))
}

fn build_reconstructed_document(
    project_id: &str,
    document_id: &str,
    sections: &[DocumentSectionSummary],
    segments: &[SegmentSummary],
    chunks: &[TranslationChunkSummary],
    chunk_segments: &[TranslationChunkSegmentSummary],
    task_runs: &[TaskRunSummary],
) -> ReconstructedDocument {
    let current_chunk_ids = chunks
        .iter()
        .map(|chunk| chunk.id.as_str())
        .collect::<HashSet<_>>();
    let mut segment_related_chunk_ids: HashMap<String, Vec<String>> = HashMap::new();
    let mut segment_primary_chunk_id: HashMap<String, String> = HashMap::new();
    let mut chunk_core_segment_ids: HashMap<String, Vec<String>> = HashMap::new();
    let mut chunk_context_before_segment_ids: HashMap<String, Vec<String>> = HashMap::new();
    let mut chunk_context_after_segment_ids: HashMap<String, Vec<String>> = HashMap::new();

    for chunk_segment in chunk_segments {
        segment_related_chunk_ids
            .entry(chunk_segment.segment_id.clone())
            .or_default()
            .push(chunk_segment.chunk_id.clone());

        match chunk_segment.role.as_str() {
            TRANSLATION_CHUNK_SEGMENT_ROLE_CORE => {
                segment_primary_chunk_id
                    .entry(chunk_segment.segment_id.clone())
                    .or_insert_with(|| chunk_segment.chunk_id.clone());
                chunk_core_segment_ids
                    .entry(chunk_segment.chunk_id.clone())
                    .or_default()
                    .push(chunk_segment.segment_id.clone());
            }
            TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE => {
                chunk_context_before_segment_ids
                    .entry(chunk_segment.chunk_id.clone())
                    .or_default()
                    .push(chunk_segment.segment_id.clone());
            }
            TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER => {
                chunk_context_after_segment_ids
                    .entry(chunk_segment.chunk_id.clone())
                    .or_default()
                    .push(chunk_segment.segment_id.clone());
            }
            _ => {}
        }
    }

    let mut task_runs_by_chunk_id: HashMap<String, Vec<TaskRunSummary>> = HashMap::new();
    let mut document_task_runs = Vec::new();
    let mut orphaned_chunk_task_runs = Vec::new();

    for task_run in task_runs {
        if let Some(chunk_id) = task_run.chunk_id.as_ref() {
            if !current_chunk_ids.contains(chunk_id.as_str()) {
                if task_run.action_type == TRANSLATE_CHUNK_ACTION_TYPE {
                    orphaned_chunk_task_runs.push(task_run.clone());
                } else {
                    document_task_runs.push(task_run.clone());
                }

                continue;
            }

            task_runs_by_chunk_id
                .entry(chunk_id.clone())
                .or_default()
                .push(task_run.clone());
        } else if task_run.action_type == TRANSLATE_CHUNK_ACTION_TYPE {
            orphaned_chunk_task_runs.push(task_run.clone());
        } else if task_run.action_type == TRANSLATE_DOCUMENT_ACTION_TYPE {
            document_task_runs.push(task_run.clone());
        }
    }

    let reconstructed_segments = segments
        .iter()
        .map(|segment| {
            let final_text = segment.target_text.clone();
            let resolved_from = if final_text.is_some() {
                RECONSTRUCTED_CONTENT_SOURCE_TARGET.to_owned()
            } else {
                RECONSTRUCTED_CONTENT_SOURCE_SOURCE_FALLBACK.to_owned()
            };

            ReconstructedSegment {
                id: segment.id.clone(),
                sequence: segment.sequence,
                source_text: segment.source_text.clone(),
                final_text: final_text.clone(),
                resolved_text: final_text
                    .clone()
                    .unwrap_or_else(|| segment.source_text.clone()),
                resolved_from,
                status: segment.status.clone(),
                primary_chunk_id: segment_primary_chunk_id.get(&segment.id).cloned(),
                related_chunk_ids: segment_related_chunk_ids
                    .get(&segment.id)
                    .cloned()
                    .unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    let reconstructed_segments_by_sequence = reconstructed_segments
        .iter()
        .map(|segment| (segment.sequence, segment))
        .collect::<HashMap<_, _>>();

    let blocks = sections
        .iter()
        .map(|section| {
            let block_segments =
                collect_block_segments(section, &reconstructed_segments_by_sequence);
            let translated_segment_count = i64::try_from(
                block_segments
                    .iter()
                    .filter(|segment| segment.final_text.is_some())
                    .count(),
            )
            .expect("block translated segment count should fit in i64");
            let segment_count =
                i64::try_from(block_segments.len()).expect("block segment count should fit in i64");
            let fallback_segment_count = segment_count - translated_segment_count;
            let status = derive_reconstruction_status(segment_count, translated_segment_count);
            let content_source = derive_content_source(segment_count, fallback_segment_count);
            let primary_chunk_ids = ordered_primary_chunk_ids(&block_segments);

            ReconstructedDocumentBlock {
                id: section.id.clone(),
                section_id: Some(section.id.clone()),
                title: Some(section.title.clone()),
                sequence: section.sequence,
                kind: section.section_type.clone(),
                level: Some(section.level),
                start_segment_sequence: section.start_segment_sequence,
                end_segment_sequence: section.end_segment_sequence,
                segment_count,
                translated_segment_count,
                untranslated_segment_count: fallback_segment_count,
                fallback_segment_count,
                status: status.clone(),
                content_source: content_source.clone(),
                final_text: join_final_text(&block_segments),
                resolved_text: join_resolved_text(&block_segments),
                segment_ids: block_segments
                    .iter()
                    .map(|segment| segment.id.clone())
                    .collect(),
                primary_chunk_ids,
                segments: block_segments,
            }
        })
        .collect::<Vec<_>>();
    let block_ids_by_section_id = blocks
        .iter()
        .filter_map(|block| {
            block
                .section_id
                .as_ref()
                .map(|section_id| (section_id.clone(), block.id.clone()))
        })
        .collect::<HashMap<_, _>>();
    let reconstructed_sections = sections
        .iter()
        .map(|section| {
            let block = blocks
                .iter()
                .find(|block| block.section_id.as_deref() == Some(section.id.as_str()))
                .expect("section block should exist");

            ReconstructedDocumentSection {
                section: section.clone(),
                status: block.status.clone(),
                content_source: block.content_source.clone(),
                translated_segment_count: block.translated_segment_count,
                untranslated_segment_count: block.untranslated_segment_count,
                fallback_segment_count: block.fallback_segment_count,
                block_id: block_ids_by_section_id
                    .get(&section.id)
                    .cloned()
                    .expect("section block id should exist"),
            }
        })
        .collect::<Vec<_>>();
    let translated_segment_count = i64::try_from(
        reconstructed_segments
            .iter()
            .filter(|segment| segment.final_text.is_some())
            .count(),
    )
    .expect("document translated segment count should fit in i64");
    let total_segments = i64::try_from(reconstructed_segments.len())
        .expect("document segment count should fit in i64");
    let fallback_segment_count = total_segments - translated_segment_count;
    let status = derive_reconstruction_status(total_segments, translated_segment_count);
    let content_source = derive_content_source(total_segments, fallback_segment_count);
    let chunk_traces = chunks
        .iter()
        .map(|chunk| {
            let chunk_task_runs = task_runs_by_chunk_id.remove(&chunk.id).unwrap_or_default();

            ReconstructedDocumentChunkTrace {
                chunk_id: chunk.id.clone(),
                chunk_sequence: chunk.sequence,
                start_segment_sequence: chunk.start_segment_sequence,
                end_segment_sequence: chunk.end_segment_sequence,
                core_segment_ids: chunk_core_segment_ids.remove(&chunk.id).unwrap_or_default(),
                context_before_segment_ids: chunk_context_before_segment_ids
                    .remove(&chunk.id)
                    .unwrap_or_default(),
                context_after_segment_ids: chunk_context_after_segment_ids
                    .remove(&chunk.id)
                    .unwrap_or_default(),
                task_run_ids: chunk_task_runs
                    .iter()
                    .map(|task_run| task_run.id.clone())
                    .collect(),
                latest_task_run: chunk_task_runs.last().cloned(),
            }
        })
        .collect::<Vec<_>>();
    let latest_document_task_run = document_task_runs.last().cloned();

    ReconstructedDocument {
        project_id: project_id.to_owned(),
        document_id: document_id.to_owned(),
        status,
        content_source,
        final_text: join_final_text(&reconstructed_segments),
        resolved_text: join_resolved_text(&reconstructed_segments),
        completeness: ReconstructedDocumentCompleteness {
            total_segments,
            translated_segments: translated_segment_count,
            untranslated_segments: fallback_segment_count,
            fallback_segments: fallback_segment_count,
            total_sections: i64::try_from(reconstructed_sections.len())
                .expect("document section count should fit in i64"),
            total_blocks: i64::try_from(blocks.len())
                .expect("document block count should fit in i64"),
            is_complete: total_segments > 0 && translated_segment_count == total_segments,
            has_translated_content: translated_segment_count > 0,
            has_reconstructible_content: total_segments > 0,
        },
        sections: reconstructed_sections,
        blocks,
        trace: ReconstructedDocumentTrace {
            chunk_count: i64::try_from(chunk_traces.len())
                .expect("chunk trace count should fit in i64"),
            task_run_count: i64::try_from(task_runs.len())
                .expect("task run count should fit in i64"),
            document_task_run_ids: document_task_runs
                .iter()
                .map(|task_run| task_run.id.clone())
                .collect(),
            latest_document_task_run,
            orphaned_chunk_task_runs,
            chunks: chunk_traces,
        },
    }
}

fn collect_block_segments(
    section: &DocumentSectionSummary,
    reconstructed_segments_by_sequence: &HashMap<i64, &ReconstructedSegment>,
) -> Vec<ReconstructedSegment> {
    (section.start_segment_sequence..=section.end_segment_sequence)
        .filter_map(|sequence| {
            reconstructed_segments_by_sequence
                .get(&sequence)
                .copied()
                .cloned()
        })
        .collect()
}

fn ordered_primary_chunk_ids(segments: &[ReconstructedSegment]) -> Vec<String> {
    let mut chunk_ids = Vec::new();

    for segment in segments {
        if let Some(chunk_id) = segment.primary_chunk_id.as_ref() {
            if !chunk_ids.contains(chunk_id) {
                chunk_ids.push(chunk_id.clone());
            }
        }
    }

    chunk_ids
}

fn join_final_text(segments: &[ReconstructedSegment]) -> Option<String> {
    if segments.is_empty() || segments.iter().any(|segment| segment.final_text.is_none()) {
        return None;
    }

    Some(
        segments
            .iter()
            .filter_map(|segment| segment.final_text.clone())
            .collect::<Vec<_>>()
            .join("\n\n"),
    )
}

fn join_resolved_text(segments: &[ReconstructedSegment]) -> String {
    segments
        .iter()
        .map(|segment| segment.resolved_text.trim())
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn derive_reconstruction_status(total_segments: i64, translated_segments: i64) -> String {
    if total_segments == 0 {
        RECONSTRUCTED_DOCUMENT_STATUS_EMPTY.to_owned()
    } else if translated_segments == 0 {
        RECONSTRUCTED_DOCUMENT_STATUS_UNTRANSLATED.to_owned()
    } else if translated_segments == total_segments {
        RECONSTRUCTED_DOCUMENT_STATUS_COMPLETE.to_owned()
    } else {
        RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL.to_owned()
    }
}

fn derive_content_source(total_segments: i64, fallback_segments: i64) -> String {
    if total_segments == 0 {
        RECONSTRUCTED_CONTENT_SOURCE_NONE.to_owned()
    } else if fallback_segments == 0 {
        RECONSTRUCTED_CONTENT_SOURCE_TARGET.to_owned()
    } else if fallback_segments == total_segments {
        RECONSTRUCTED_CONTENT_SOURCE_SOURCE_FALLBACK.to_owned()
    } else {
        RECONSTRUCTED_CONTENT_SOURCE_MIXED.to_owned()
    }
}

pub(crate) fn current_timestamp() -> Result<i64, DesktopCommandError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not read the system clock while reconstructing a document.",
                Some(error.to_string()),
            )
        })?;

    i64::try_from(duration.as_secs()).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid reconstruction timestamp.",
            Some(error.to_string()),
        )
    })
}

pub(crate) fn validate_identifier(value: &str, label: &str) -> Result<String, DesktopCommandError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(DesktopCommandError::validation(
            format!("The reconstructed document flow requires a valid {label}."),
            None,
        ));
    }

    if !trimmed
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(
            format!("The reconstructed document flow requires a safe persisted {label}."),
            None,
        ));
    }

    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, TempDir};

    use super::{build_reconstructed_document, get_reconstructed_document_with_runtime};
    use crate::document_export::EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE;
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
    use crate::reconstructed_documents::{
        GetReconstructedDocumentInput, RECONSTRUCTED_CONTENT_SOURCE_MIXED,
        RECONSTRUCTED_CONTENT_SOURCE_SOURCE_FALLBACK, RECONSTRUCTED_CONTENT_SOURCE_TARGET,
        RECONSTRUCTED_DOCUMENT_STATUS_COMPLETE, RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL,
        RECONSTRUCTED_DOCUMENT_STATUS_UNTRANSLATED,
    };
    use crate::sections::DocumentSectionSummary;
    use crate::sections::{NewDocumentSection, DOCUMENT_SECTION_TYPE_CHAPTER};
    use crate::segments::SegmentSummary;
    use crate::segments::{
        NewSegment, SegmentTranslationWrite, SEGMENT_STATUS_PENDING_TRANSLATION,
    };
    use crate::task_runs::{
        NewTaskRun, TaskRunSummary, TASK_RUN_STATUS_COMPLETED, TASK_RUN_STATUS_RUNNING,
    };
    use crate::translate_chunk::TRANSLATE_CHUNK_ACTION_TYPE;
    use crate::translate_document::TRANSLATE_DOCUMENT_ACTION_TYPE;
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment, TranslationChunkSummary,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const PROJECT_ID: &str = "prj_active_001";
    const DOCUMENT_ID: &str = "doc_reconstruct_001";
    const NOW: i64 = 1_743_517_200;

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

    fn seed_reconstruction_graph(runtime: &DatabaseRuntime) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: PROJECT_ID.to_owned(),
                name: "Reconstruction project".to_owned(),
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
                name: "chaptered.txt".to_owned(),
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
                        id: "seg_0001".to_owned(),
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
                        id: "seg_0002".to_owned(),
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
                        id: "seg_0003".to_owned(),
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
                        id: "seg_0004".to_owned(),
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
                        id: "doc_reconstruct_001_sec_0001".to_owned(),
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
                        id: "doc_reconstruct_001_sec_0002".to_owned(),
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
                        id: "doc_reconstruct_001_chunk_0001".to_owned(),
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
                        id: "doc_reconstruct_001_chunk_0002".to_owned(),
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
                        chunk_id: "doc_reconstruct_001_chunk_0001".to_owned(),
                        segment_id: "seg_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_reconstruct_001_chunk_0001".to_owned(),
                        segment_id: "seg_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_reconstruct_001_chunk_0001".to_owned(),
                        segment_id: "seg_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_reconstruct_001_chunk_0002".to_owned(),
                        segment_id: "seg_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_reconstruct_001_chunk_0002".to_owned(),
                        segment_id: "seg_0004".to_owned(),
                        segment_sequence: 4,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                ],
            )
            .expect("chunks should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_doc_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                job_id: Some("job_translate_doc_001".to_owned()),
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
                id: "task_chunk_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_reconstruct_001_chunk_0001".to_owned()),
                job_id: Some("job_translate_doc_001".to_owned()),
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
                "task_chunk_0001",
                "{\"translations\":[1,2]}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_0001".to_owned(),
                        target_text: "Capítulo I".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_0002".to_owned(),
                        target_text: "La puerta siguió cerrada.".to_owned(),
                    },
                ],
                NOW + 30,
            )
            .expect("first chunk translation projection should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_chunk_0002".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_reconstruct_001_chunk_0002".to_owned()),
                job_id: Some("job_translate_doc_001".to_owned()),
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
    fn reconstructed_document_rejects_invalid_identifiers() {
        let fixture = create_runtime_fixture();

        let error = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: " ".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect_err("invalid ids should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("project id"));
    }

    #[test]
    fn reconstructed_document_rejects_unknown_documents() {
        let fixture = create_runtime_fixture();
        seed_reconstruction_graph(&fixture.runtime);

        let error = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: "doc_missing_001".to_owned(),
            },
            &fixture.runtime,
        )
        .expect_err("unknown documents should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("does not exist"));
    }

    #[test]
    fn reconstructed_document_preserves_segment_sequence_and_sections() {
        let fixture = create_runtime_fixture();
        seed_reconstruction_graph(&fixture.runtime);

        let document = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("reconstructed document should load");

        assert_eq!(document.blocks.len(), 2);
        assert_eq!(document.blocks[0].segment_ids, vec!["seg_0001", "seg_0002"]);
        assert_eq!(document.blocks[1].segment_ids, vec!["seg_0003", "seg_0004"]);
        assert_eq!(document.sections.len(), 2);
        assert_eq!(document.sections[0].section.title, "Chapter I");
        assert_eq!(document.sections[1].section.title, "Chapter II");
        assert_eq!(
            document.blocks[0].resolved_text,
            "Capítulo I\n\nLa puerta siguió cerrada."
        );
        assert_eq!(
            document.blocks[1].resolved_text,
            "Chapter II\n\nThe lantern burned all night."
        );
    }

    #[test]
    fn reconstructed_document_marks_partial_documents_and_source_fallbacks() {
        let fixture = create_runtime_fixture();
        seed_reconstruction_graph(&fixture.runtime);

        let document = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("reconstructed document should load");

        assert_eq!(document.status, RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL);
        assert_eq!(document.content_source, RECONSTRUCTED_CONTENT_SOURCE_MIXED);
        assert_eq!(document.final_text, None);
        assert_eq!(document.completeness.total_segments, 4);
        assert_eq!(document.completeness.translated_segments, 2);
        assert_eq!(document.completeness.untranslated_segments, 2);
        assert_eq!(document.completeness.fallback_segments, 2);
        assert_eq!(
            document.blocks[1].status,
            RECONSTRUCTED_DOCUMENT_STATUS_UNTRANSLATED
        );
        assert_eq!(
            document.blocks[1].content_source,
            RECONSTRUCTED_CONTENT_SOURCE_SOURCE_FALLBACK
        );
        assert_eq!(
            document.blocks[1].segments[0].resolved_from,
            "source_fallback"
        );
        assert_eq!(document.blocks[1].segments[0].final_text, None);
    }

    #[test]
    fn reconstructed_document_exposes_chunk_and_task_run_traceability() {
        let fixture = create_runtime_fixture();
        seed_reconstruction_graph(&fixture.runtime);

        let document = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("reconstructed document should load");

        assert_eq!(document.trace.chunk_count, 2);
        assert_eq!(document.trace.task_run_count, 3);
        assert_eq!(document.trace.document_task_run_ids, vec!["task_doc_0001"]);
        assert!(document.trace.orphaned_chunk_task_runs.is_empty());
        assert_eq!(
            document
                .trace
                .latest_document_task_run
                .as_ref()
                .map(|task_run| task_run.id.as_str()),
            Some("task_doc_0001")
        );
        assert_eq!(
            document.blocks[0].primary_chunk_ids,
            vec!["doc_reconstruct_001_chunk_0001"]
        );
        assert_eq!(
            document.blocks[1].segments[0].primary_chunk_id.as_deref(),
            Some("doc_reconstruct_001_chunk_0002")
        );
        assert_eq!(
            document.blocks[1].segments[0].related_chunk_ids,
            vec![
                "doc_reconstruct_001_chunk_0001".to_owned(),
                "doc_reconstruct_001_chunk_0002".to_owned()
            ]
        );
        assert_eq!(
            document.trace.chunks[0].core_segment_ids,
            vec!["seg_0001".to_owned(), "seg_0002".to_owned()]
        );
        assert_eq!(
            document.trace.chunks[0].context_after_segment_ids,
            vec!["seg_0003".to_owned()]
        );
        assert_eq!(
            document.trace.chunks[0].task_run_ids,
            vec!["task_chunk_0001"]
        );
        assert_eq!(
            document.trace.chunks[1]
                .latest_task_run
                .as_ref()
                .map(|task_run| task_run.id.as_str()),
            Some("task_chunk_0002")
        );
    }

    #[test]
    fn reconstructed_document_ignores_export_runs_in_document_level_trace() {
        let fixture = create_runtime_fixture();
        seed_reconstruction_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_export_snapshot_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                job_id: None,
                action_type: EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: Some("{\"fileName\":\"draft.translated.md\"}".to_owned()),
                output_payload: Some("{\"status\":\"complete\"}".to_owned()),
                error_message: None,
                started_at: NOW + 65,
                completed_at: Some(NOW + 65),
                created_at: NOW + 65,
                updated_at: NOW + 65,
            })
            .expect("export snapshot should persist");
        drop(connection);

        let document = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("reconstructed document should load after export snapshot");

        assert_eq!(document.trace.task_run_count, 4);
        assert_eq!(document.trace.document_task_run_ids, vec!["task_doc_0001"]);
        assert_eq!(
            document
                .trace
                .latest_document_task_run
                .as_ref()
                .map(|task_run| task_run.id.as_str()),
            Some("task_doc_0001")
        );
    }

    #[test]
    fn reconstructed_document_keeps_orphaned_chunk_runs_out_of_document_level_history() {
        let fixture = create_runtime_fixture();
        seed_reconstruction_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[NewTranslationChunk {
                    id: "doc_reconstruct_001_chunk_0101".to_owned(),
                    document_id: DOCUMENT_ID.to_owned(),
                    sequence: 101,
                    builder_version: "tr12-basic-v1".to_owned(),
                    strategy: "section-aware-fixed-word-target-v1".to_owned(),
                    source_text:
                        "Chapter I\n\nThe gate remained closed.\n\nChapter II\n\nThe lantern burned all night."
                            .to_owned(),
                    context_before_text: None,
                    context_after_text: None,
                    start_segment_sequence: 1,
                    end_segment_sequence: 4,
                    segment_count: 4,
                    source_word_count: 13,
                    source_character_count: 73,
                    created_at: NOW + 90,
                    updated_at: NOW + 90,
                }],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_reconstruct_001_chunk_0101".to_owned(),
                        segment_id: "seg_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_reconstruct_001_chunk_0101".to_owned(),
                        segment_id: "seg_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_reconstruct_001_chunk_0101".to_owned(),
                        segment_id: "seg_0003".to_owned(),
                        segment_sequence: 3,
                        position: 3,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_reconstruct_001_chunk_0101".to_owned(),
                        segment_id: "seg_0004".to_owned(),
                        segment_sequence: 4,
                        position: 4,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                ],
            )
            .expect("chunk replacement should preserve a rebuilt chunk set");
        drop(connection);

        let document = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("reconstructed document should load after chunk rebuild");

        assert_eq!(document.trace.task_run_count, 3);
        assert_eq!(document.trace.document_task_run_ids, vec!["task_doc_0001"]);
        assert_eq!(
            document
                .trace
                .latest_document_task_run
                .as_ref()
                .map(|task_run| task_run.id.as_str()),
            Some("task_doc_0001")
        );
        assert_eq!(
            document
                .trace
                .orphaned_chunk_task_runs
                .iter()
                .map(|task_run| task_run.id.as_str())
                .collect::<Vec<_>>(),
            vec!["task_chunk_0001", "task_chunk_0002"]
        );
        assert_eq!(document.trace.chunk_count, 1);
        assert_eq!(
            document.trace.chunks[0].chunk_id,
            "doc_reconstruct_001_chunk_0101"
        );
        assert!(document.trace.chunks[0].task_run_ids.is_empty());
    }

    #[test]
    fn reconstructed_document_marks_stale_chunk_ids_as_orphaned_history() {
        let sections = vec![DocumentSectionSummary {
            id: "doc_reconstruct_001_sec_0001".to_owned(),
            document_id: DOCUMENT_ID.to_owned(),
            sequence: 1,
            title: "Chapter I".to_owned(),
            section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
            level: 1,
            start_segment_sequence: 1,
            end_segment_sequence: 1,
            segment_count: 1,
            created_at: NOW,
            updated_at: NOW,
        }];
        let segments = vec![SegmentSummary {
            id: "seg_0001".to_owned(),
            document_id: DOCUMENT_ID.to_owned(),
            sequence: 1,
            source_text: "Chapter I".to_owned(),
            target_text: Some("Capítulo I".to_owned()),
            source_word_count: 2,
            source_character_count: 9,
            status: "translated".to_owned(),
            created_at: NOW,
            updated_at: NOW,
        }];
        let chunks = vec![TranslationChunkSummary {
            id: "doc_reconstruct_001_chunk_0101".to_owned(),
            document_id: DOCUMENT_ID.to_owned(),
            sequence: 101,
            builder_version: "tr12-basic-v1".to_owned(),
            strategy: "section-aware-fixed-word-target-v1".to_owned(),
            source_text: "Chapter I".to_owned(),
            context_before_text: None,
            context_after_text: None,
            start_segment_sequence: 1,
            end_segment_sequence: 1,
            segment_count: 1,
            source_word_count: 2,
            source_character_count: 9,
            created_at: NOW,
            updated_at: NOW,
        }];
        let task_runs = vec![
            TaskRunSummary {
                id: "task_doc_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                job_id: Some("job_translate_doc_001".to_owned()),
                action_type: TRANSLATE_DOCUMENT_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: None,
                output_payload: None,
                error_message: None,
                started_at: NOW,
                completed_at: Some(NOW + 60),
                created_at: NOW,
                updated_at: NOW + 60,
            },
            TaskRunSummary {
                id: "task_chunk_stale_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_reconstruct_001_chunk_0001".to_owned()),
                job_id: Some("job_translate_doc_001".to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: None,
                output_payload: None,
                error_message: None,
                started_at: NOW + 1,
                completed_at: Some(NOW + 30),
                created_at: NOW + 1,
                updated_at: NOW + 30,
            },
        ];

        let document = build_reconstructed_document(
            PROJECT_ID,
            DOCUMENT_ID,
            &sections,
            &segments,
            &chunks,
            &[],
            &task_runs,
        );

        assert_eq!(document.trace.document_task_run_ids, vec!["task_doc_0001"]);
        assert_eq!(document.trace.chunks[0].task_run_ids.len(), 0);
        assert_eq!(
            document
                .trace
                .orphaned_chunk_task_runs
                .iter()
                .map(|task_run| task_run.id.as_str())
                .collect::<Vec<_>>(),
            vec!["task_chunk_stale_0001"]
        );
    }

    #[test]
    fn reconstructed_document_becomes_complete_after_all_chunks_project_translations() {
        let fixture = create_runtime_fixture();
        seed_reconstruction_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");

        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_chunk_0002",
                "{\"translations\":[3,4]}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_0003".to_owned(),
                        target_text: "Capítulo II".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_0004".to_owned(),
                        target_text: "La linterna ardió toda la noche.".to_owned(),
                    },
                ],
                NOW + 45,
            )
            .expect("second chunk translation projection should persist");

        let document = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("reconstructed document should load");

        assert_eq!(document.status, RECONSTRUCTED_DOCUMENT_STATUS_COMPLETE);
        assert_eq!(document.content_source, RECONSTRUCTED_CONTENT_SOURCE_TARGET);
        assert_eq!(
            document.final_text.as_deref(),
            Some(
                "Capítulo I\n\nLa puerta siguió cerrada.\n\nCapítulo II\n\nLa linterna ardió toda la noche."
            )
        );
        assert_eq!(document.completeness.translated_segments, 4);
        assert_eq!(document.completeness.untranslated_segments, 0);
        assert!(document.completeness.is_complete);
    }

    #[test]
    fn reconstructed_document_is_stable_and_deterministic() {
        let fixture = create_runtime_fixture();
        seed_reconstruction_graph(&fixture.runtime);

        let first = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("first reconstruction should load");
        let second = get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("second reconstruction should load");

        assert_eq!(first, second);
    }
}
