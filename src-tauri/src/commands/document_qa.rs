use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::json;
use tauri::State;

use crate::commands::reconstructed_documents::{
    current_timestamp, load_reconstructed_document, validate_identifier,
};
use crate::document_qa::{
    DocumentConsistencyQaResult, DocumentQaFindingsOverview, ListDocumentQaFindingsInput,
    RunDocumentConsistencyQaInput, DOCUMENT_QA_FINDING_TYPE_CHUNK_EXECUTION_ERROR,
    DOCUMENT_QA_FINDING_TYPE_NEIGHBOR_CHUNK_TRANSLATION_DRIFT,
    DOCUMENT_QA_FINDING_TYPE_ORPHANED_CHUNK_TASK_RUN,
    DOCUMENT_QA_FINDING_TYPE_PARTIAL_BLOCK_TRANSLATION,
    DOCUMENT_QA_FINDING_TYPE_SOURCE_FALLBACK_SEGMENT,
};
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::qa_findings::QaFindingRepository;
use crate::persistence::task_runs::TaskRunRepository;
use crate::qa_findings::{
    NewQaFinding, QA_FINDING_SEVERITY_HIGH, QA_FINDING_SEVERITY_LOW, QA_FINDING_SEVERITY_MEDIUM,
    QA_FINDING_STATUS_OPEN,
};
use crate::reconstructed_documents::{
    ReconstructedDocument, ReconstructedDocumentBlock, ReconstructedSegment,
    RECONSTRUCTED_CONTENT_SOURCE_MIXED, RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL,
};
use crate::task_runs::{TaskRunSummary, TASK_RUN_STATUS_CANCELLED, TASK_RUN_STATUS_FAILED};
use crate::translate_chunk::TRANSLATE_CHUNK_ACTION_TYPE;

const QA_FINDING_ID_PREFIX: &str = "qaf_tr21";

#[derive(Debug, Clone, PartialEq, Eq)]
struct QaFindingDraft {
    id: String,
    chunk_id: Option<String>,
    task_run_id: Option<String>,
    job_id: Option<String>,
    finding_type: String,
    severity: String,
    message: String,
    details: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct QaAnchor {
    chunk_id: Option<String>,
    task_run_id: Option<String>,
    job_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct QaTraceContext {
    chunk_latest_task_runs: HashMap<String, TaskRunSummary>,
    orphaned_chunk_task_runs: Vec<TaskRunSummary>,
    fallback_job_id: Option<String>,
}

#[tauri::command]
pub fn run_document_consistency_qa(
    input: RunDocumentConsistencyQaInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<DocumentConsistencyQaResult, DesktopCommandError> {
    run_document_consistency_qa_with_runtime(input, database_runtime.inner())
}

#[tauri::command]
pub fn list_document_qa_findings(
    input: ListDocumentQaFindingsInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<DocumentQaFindingsOverview, DesktopCommandError> {
    list_document_qa_findings_with_runtime(input, database_runtime.inner())
}

pub(crate) fn run_document_consistency_qa_with_runtime(
    input: RunDocumentConsistencyQaInput,
    database_runtime: &DatabaseRuntime,
) -> Result<DocumentConsistencyQaResult, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let job_id = input
        .job_id
        .map(|value| validate_identifier(&value, "job id"))
        .transpose()?;
    let executed_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for document QA.",
            Some(error.to_string()),
        )
    })?;
    let reconstructed_document = load_reconstructed_document(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        executed_at,
    )?;
    let requested_job_task_runs =
        load_requested_job_task_runs(&mut connection, &document_id, job_id.as_deref())?;
    let trace_context = build_trace_context(
        &reconstructed_document,
        job_id.as_deref(),
        &requested_job_task_runs,
    );
    let finding_drafts = build_finding_drafts(&reconstructed_document, &trace_context);
    let mut repository = QaFindingRepository::new(&mut connection);
    let mut generated_findings = Vec::new();

    for draft in finding_drafts {
        generated_findings.push(
            repository
                .upsert(&NewQaFinding {
                    id: draft.id,
                    document_id: document_id.clone(),
                    chunk_id: draft.chunk_id,
                    task_run_id: draft.task_run_id,
                    job_id: draft.job_id,
                    finding_type: draft.finding_type,
                    severity: draft.severity,
                    status: QA_FINDING_STATUS_OPEN.to_owned(),
                    message: draft.message,
                    details: draft.details,
                    created_at: executed_at,
                    updated_at: executed_at,
                })
                .map_err(|error| {
                    DesktopCommandError::internal(
                        "The desktop shell could not persist document QA findings.",
                        Some(error.to_string()),
                    )
                })?,
        );
    }

    Ok(DocumentConsistencyQaResult {
        project_id,
        document_id,
        job_id,
        reconstructed_status: reconstructed_document.status,
        reconstructed_content_source: reconstructed_document.content_source,
        generated_findings,
    })
}

pub(crate) fn list_document_qa_findings_with_runtime(
    input: ListDocumentQaFindingsInput,
    database_runtime: &DatabaseRuntime,
) -> Result<DocumentQaFindingsOverview, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let job_id = input
        .job_id
        .map(|value| validate_identifier(&value, "job id"))
        .transpose()?;
    let listed_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for listing QA findings.",
            Some(error.to_string()),
        )
    })?;
    let _ = load_reconstructed_document(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        listed_at,
    )?;
    let findings = QaFindingRepository::new(&mut connection)
        .list_by_document(&document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load QA findings for the selected document.",
                Some(error.to_string()),
            )
        })?
        .into_iter()
        .filter(|finding| match job_id.as_deref() {
            Some(job_id) => finding.job_id.as_deref() == Some(job_id),
            None => true,
        })
        .collect();

    Ok(DocumentQaFindingsOverview {
        project_id,
        document_id,
        job_id,
        findings,
    })
}

fn load_requested_job_task_runs(
    connection: &mut rusqlite::Connection,
    document_id: &str,
    job_id: Option<&str>,
) -> Result<Vec<TaskRunSummary>, DesktopCommandError> {
    let Some(job_id) = job_id else {
        return Ok(Vec::new());
    };

    let task_runs = TaskRunRepository::new(connection)
        .list_by_job_id(job_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not inspect task runs for the selected QA job.",
                Some(error.to_string()),
            )
        })?
        .into_iter()
        .filter(|task_run| task_run.document_id == document_id)
        .collect::<Vec<_>>();

    if task_runs.is_empty() {
        return Err(DesktopCommandError::validation(
            "The selected QA job does not exist for the active document.",
            None,
        ));
    }

    Ok(task_runs)
}

fn build_trace_context(
    reconstructed_document: &ReconstructedDocument,
    requested_job_id: Option<&str>,
    requested_job_task_runs: &[TaskRunSummary],
) -> QaTraceContext {
    let current_chunk_ids = reconstructed_document
        .trace
        .chunks
        .iter()
        .map(|chunk| chunk.chunk_id.as_str())
        .collect::<HashSet<_>>();

    if requested_job_id.is_none() {
        return QaTraceContext {
            chunk_latest_task_runs: reconstructed_document
                .trace
                .chunks
                .iter()
                .filter_map(|chunk| {
                    chunk
                        .latest_task_run
                        .as_ref()
                        .map(|task_run| (chunk.chunk_id.clone(), task_run.clone()))
                })
                .collect(),
            orphaned_chunk_task_runs: reconstructed_document
                .trace
                .orphaned_chunk_task_runs
                .clone(),
            fallback_job_id: reconstructed_document
                .trace
                .latest_document_task_run
                .as_ref()
                .and_then(|task_run| task_run.job_id.clone()),
        };
    }

    let mut chunk_latest_task_runs = HashMap::new();
    let mut orphaned_chunk_task_runs = Vec::new();

    for task_run in requested_job_task_runs {
        if let Some(chunk_id) = task_run.chunk_id.as_ref() {
            if current_chunk_ids.contains(chunk_id.as_str()) {
                chunk_latest_task_runs.insert(chunk_id.clone(), task_run.clone());
            } else if task_run.action_type == TRANSLATE_CHUNK_ACTION_TYPE {
                orphaned_chunk_task_runs.push(task_run.clone());
            }
        } else if task_run.action_type == TRANSLATE_CHUNK_ACTION_TYPE {
            orphaned_chunk_task_runs.push(task_run.clone());
        }
    }

    QaTraceContext {
        chunk_latest_task_runs,
        orphaned_chunk_task_runs,
        fallback_job_id: requested_job_id.map(str::to_owned),
    }
}

fn build_finding_drafts(
    reconstructed_document: &ReconstructedDocument,
    trace_context: &QaTraceContext,
) -> Vec<QaFindingDraft> {
    let mut findings = BTreeMap::new();
    let chunk_sequences = reconstructed_document
        .trace
        .chunks
        .iter()
        .map(|chunk| (chunk.chunk_id.as_str(), chunk.chunk_sequence))
        .collect::<HashMap<_, _>>();

    if reconstructed_document.content_source == RECONSTRUCTED_CONTENT_SOURCE_MIXED
        && reconstructed_document.completeness.has_translated_content
    {
        for block in &reconstructed_document.blocks {
            if block.status == RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL {
                let anchor = anchor_for_block(block, trace_context);
                let fallback_segment_ids = block
                    .segments
                    .iter()
                    .filter(|segment| segment.final_text.is_none())
                    .map(|segment| segment.id.clone())
                    .collect::<Vec<_>>();

                insert_finding(
                    &mut findings,
                    QaFindingDraft {
                        id: format!(
                            "{QA_FINDING_ID_PREFIX}_{document_id}_partial_block_translation_{}",
                            block.id,
                            document_id = reconstructed_document.document_id
                        ),
                        chunk_id: anchor.chunk_id,
                        task_run_id: anchor.task_run_id,
                        job_id: anchor.job_id,
                        finding_type: DOCUMENT_QA_FINDING_TYPE_PARTIAL_BLOCK_TRANSLATION.to_owned(),
                        severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                        message: format!(
                            "Block {} mixes translated and fallback segments.",
                            block.id
                        ),
                        details: Some(
                            json!({
                                "blockId": block.id,
                                "sectionId": block.section_id,
                                "translatedSegmentCount": block.translated_segment_count,
                                "fallbackSegmentCount": block.fallback_segment_count,
                                "segmentIds": block.segment_ids,
                                "fallbackSegmentIds": fallback_segment_ids,
                                "primaryChunkIds": block.primary_chunk_ids,
                            })
                            .to_string(),
                        ),
                    },
                );
            }

            for segment in &block.segments {
                if segment.final_text.is_none() {
                    let anchor = anchor_for_segment(segment, trace_context);

                    insert_finding(
                        &mut findings,
                        QaFindingDraft {
                            id: format!(
                                "{QA_FINDING_ID_PREFIX}_{document_id}_source_fallback_segment_{}",
                                segment.id,
                                document_id = reconstructed_document.document_id
                            ),
                            chunk_id: anchor.chunk_id,
                            task_run_id: anchor.task_run_id,
                            job_id: anchor.job_id,
                            finding_type: DOCUMENT_QA_FINDING_TYPE_SOURCE_FALLBACK_SEGMENT
                                .to_owned(),
                            severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                            message: format!(
                                "Segment {} fell back to source text while the document already contains translated content.",
                                segment.id
                            ),
                            details: Some(
                                json!({
                                    "segmentId": segment.id,
                                    "segmentSequence": segment.sequence,
                                    "sourceText": segment.source_text,
                                    "relatedChunkIds": segment.related_chunk_ids,
                                    "primaryChunkId": segment.primary_chunk_id,
                                })
                                .to_string(),
                            ),
                        },
                    );
                }
            }
        }
    }

    for chunk in &reconstructed_document.trace.chunks {
        let Some(task_run) = trace_context
            .chunk_latest_task_runs
            .get(chunk.chunk_id.as_str())
            .cloned()
        else {
            continue;
        };

        if matches!(
            task_run.status.as_str(),
            TASK_RUN_STATUS_FAILED | TASK_RUN_STATUS_CANCELLED
        ) {
            insert_finding(
                &mut findings,
                QaFindingDraft {
                    id: format!(
                        "{QA_FINDING_ID_PREFIX}_{document_id}_chunk_execution_error_{}",
                        task_run.id,
                        document_id = reconstructed_document.document_id
                    ),
                    chunk_id: Some(chunk.chunk_id.clone()),
                    task_run_id: Some(task_run.id.clone()),
                    job_id: task_run
                        .job_id
                        .clone()
                        .or_else(|| trace_context.fallback_job_id.clone()),
                    finding_type: DOCUMENT_QA_FINDING_TYPE_CHUNK_EXECUTION_ERROR.to_owned(),
                    severity: if task_run.status == TASK_RUN_STATUS_FAILED {
                        QA_FINDING_SEVERITY_HIGH.to_owned()
                    } else {
                        QA_FINDING_SEVERITY_MEDIUM.to_owned()
                    },
                    message: format!(
                        "Chunk {} has a latest task run in {} state.",
                        chunk.chunk_id, task_run.status
                    ),
                    details: Some(
                        json!({
                            "chunkId": chunk.chunk_id,
                            "chunkSequence": chunk.chunk_sequence,
                            "taskRunId": task_run.id,
                            "taskRunStatus": task_run.status,
                            "errorMessage": task_run.error_message,
                        })
                        .to_string(),
                    ),
                },
            );
        }
    }

    for task_run in &trace_context.orphaned_chunk_task_runs {
        insert_finding(
            &mut findings,
            QaFindingDraft {
                id: format!(
                    "{QA_FINDING_ID_PREFIX}_{document_id}_orphaned_chunk_task_run_{}",
                    task_run.id,
                    document_id = reconstructed_document.document_id
                ),
                chunk_id: None,
                task_run_id: Some(task_run.id.clone()),
                job_id: task_run
                    .job_id
                    .clone()
                    .or_else(|| trace_context.fallback_job_id.clone()),
                finding_type: DOCUMENT_QA_FINDING_TYPE_ORPHANED_CHUNK_TASK_RUN.to_owned(),
                severity: QA_FINDING_SEVERITY_LOW.to_owned(),
                message: format!(
                    "Chunk task run {} no longer maps to a current reconstructed chunk.",
                    task_run.id
                ),
                details: Some(
                    json!({
                        "taskRunId": task_run.id,
                        "originalChunkId": task_run.chunk_id,
                        "taskRunStatus": task_run.status,
                        "actionType": task_run.action_type,
                    })
                    .to_string(),
                ),
            },
        );
    }

    let mut repeated_sources: HashMap<String, Vec<&ReconstructedSegment>> = HashMap::new();

    for block in &reconstructed_document.blocks {
        for segment in &block.segments {
            if segment.final_text.is_some() && segment.primary_chunk_id.is_some() {
                repeated_sources
                    .entry(normalize_for_comparison(segment.source_text.as_str()))
                    .or_default()
                    .push(segment);
            }
        }
    }

    for group in repeated_sources.values_mut() {
        group.sort_by_key(|segment| {
            let chunk_sequence = segment
                .primary_chunk_id
                .as_deref()
                .and_then(|chunk_id| chunk_sequences.get(chunk_id))
                .copied()
                .unwrap_or(i64::MAX);

            (chunk_sequence, segment.sequence)
        });

        for window in group.windows(2) {
            let [previous_segment, current_segment] = window else {
                continue;
            };
            let Some(previous_chunk_id) = previous_segment.primary_chunk_id.as_deref() else {
                continue;
            };
            let Some(current_chunk_id) = current_segment.primary_chunk_id.as_deref() else {
                continue;
            };
            let Some(previous_chunk_sequence) = chunk_sequences.get(previous_chunk_id).copied()
            else {
                continue;
            };
            let Some(current_chunk_sequence) = chunk_sequences.get(current_chunk_id).copied()
            else {
                continue;
            };

            if current_chunk_sequence != previous_chunk_sequence + 1 {
                continue;
            }

            if normalize_for_comparison(previous_segment.final_text.as_deref().unwrap_or_default())
                == normalize_for_comparison(
                    current_segment.final_text.as_deref().unwrap_or_default(),
                )
            {
                continue;
            }

            let anchor = anchor_for_chunk_id(Some(current_chunk_id), trace_context);

            insert_finding(
                &mut findings,
                QaFindingDraft {
                    id: format!(
                        "{QA_FINDING_ID_PREFIX}_{document_id}_neighbor_chunk_translation_drift_{}_{}",
                        previous_segment.id,
                        current_segment.id,
                        document_id = reconstructed_document.document_id
                    ),
                    chunk_id: anchor.chunk_id,
                    task_run_id: anchor.task_run_id,
                    job_id: anchor.job_id,
                    finding_type: DOCUMENT_QA_FINDING_TYPE_NEIGHBOR_CHUNK_TRANSLATION_DRIFT
                        .to_owned(),
                    severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                    message: format!(
                        "Repeated source text is translated differently across neighboring chunks {} and {}.",
                        previous_chunk_id, current_chunk_id
                    ),
                    details: Some(
                        json!({
                            "sourceText": current_segment.source_text,
                            "previousSegmentId": previous_segment.id,
                            "previousChunkId": previous_chunk_id,
                            "previousTargetText": previous_segment.final_text,
                            "currentSegmentId": current_segment.id,
                            "currentChunkId": current_chunk_id,
                            "currentTargetText": current_segment.final_text,
                        })
                        .to_string(),
                    ),
                },
            );
        }
    }

    findings.into_values().collect()
}

fn insert_finding(findings: &mut BTreeMap<String, QaFindingDraft>, draft: QaFindingDraft) {
    findings.insert(draft.id.clone(), draft);
}

fn anchor_for_block(
    block: &ReconstructedDocumentBlock,
    trace_context: &QaTraceContext,
) -> QaAnchor {
    let anchor_chunk_id = block
        .segments
        .iter()
        .find(|segment| segment.final_text.is_none())
        .and_then(|segment| {
            segment
                .primary_chunk_id
                .clone()
                .or_else(|| segment.related_chunk_ids.last().cloned())
        })
        .or_else(|| block.primary_chunk_ids.last().cloned());

    anchor_for_chunk_id(anchor_chunk_id.as_deref(), trace_context)
}

fn anchor_for_segment(segment: &ReconstructedSegment, trace_context: &QaTraceContext) -> QaAnchor {
    let anchor_chunk_id = segment
        .primary_chunk_id
        .as_deref()
        .or_else(|| segment.related_chunk_ids.last().map(String::as_str));

    anchor_for_chunk_id(anchor_chunk_id, trace_context)
}

fn anchor_for_chunk_id(chunk_id: Option<&str>, trace_context: &QaTraceContext) -> QaAnchor {
    let chunk_id = chunk_id.map(str::to_owned);
    let task_run = chunk_id
        .as_deref()
        .and_then(|chunk_id| trace_context.chunk_latest_task_runs.get(chunk_id))
        .cloned();

    QaAnchor {
        chunk_id,
        task_run_id: task_run.as_ref().map(|task_run| task_run.id.clone()),
        job_id: task_run
            .and_then(|task_run| task_run.job_id)
            .or_else(|| trace_context.fallback_job_id.clone()),
    }
}

fn normalize_for_comparison(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, TempDir};

    use super::{list_document_qa_findings_with_runtime, run_document_consistency_qa_with_runtime};
    use crate::document_qa::{
        ListDocumentQaFindingsInput, RunDocumentConsistencyQaInput,
        DOCUMENT_QA_FINDING_TYPE_CHUNK_EXECUTION_ERROR,
        DOCUMENT_QA_FINDING_TYPE_NEIGHBOR_CHUNK_TRANSLATION_DRIFT,
        DOCUMENT_QA_FINDING_TYPE_PARTIAL_BLOCK_TRANSLATION,
        DOCUMENT_QA_FINDING_TYPE_SOURCE_FALLBACK_SEGMENT,
    };
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_IMPORTED};
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::qa_findings::QaFindingRepository;
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::persistence::sections::DocumentSectionRepository;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::task_runs::TaskRunRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::NewProject;
    use crate::qa_findings::QA_FINDING_STATUS_OPEN;
    use crate::reconstructed_documents::{
        RECONSTRUCTED_CONTENT_SOURCE_MIXED, RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL,
    };
    use crate::sections::{NewDocumentSection, DOCUMENT_SECTION_TYPE_CHAPTER};
    use crate::segments::{
        NewSegment, SegmentTranslationWrite, SEGMENT_STATUS_PENDING_TRANSLATION,
    };
    use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_RUNNING};
    use crate::translate_chunk::TRANSLATE_CHUNK_ACTION_TYPE;
    use crate::translate_document::TRANSLATE_DOCUMENT_ACTION_TYPE;
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const PROJECT_ID: &str = "prj_qa_001";
    const DOCUMENT_ID: &str = "doc_qa_001";
    const JOB_ID: &str = "job_translate_doc_qa_001";
    const NOW: i64 = 1_775_612_800;

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

    fn seed_document_qa_graph(runtime: &DatabaseRuntime) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: PROJECT_ID.to_owned(),
                name: "Document QA project".to_owned(),
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
                name: "document-qa.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "ignored".to_owned(),
                file_size_bytes: 2_048,
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
                        source_text: "Guard the gate.".to_owned(),
                        source_word_count: 3,
                        source_character_count: 15,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        source_text: "Hold the line.".to_owned(),
                        source_word_count: 3,
                        source_character_count: 14,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_0003".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 3,
                        source_text: "Hold the line.".to_owned(),
                        source_word_count: 3,
                        source_character_count: 14,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_0004".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 4,
                        source_text: "The flame must endure.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 23,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_0005".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 5,
                        source_text: "Keep the rune covered.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 22,
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
                        id: "sec_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        title: "Part I".to_owned(),
                        section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                        level: 1,
                        start_segment_sequence: 1,
                        end_segment_sequence: 2,
                        segment_count: 2,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewDocumentSection {
                        id: "sec_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        title: "Part II".to_owned(),
                        section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                        level: 1,
                        start_segment_sequence: 3,
                        end_segment_sequence: 5,
                        segment_count: 3,
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
                        id: "doc_qa_001_chunk_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "Guard the gate.\n\nHold the line.".to_owned(),
                        context_before_text: None,
                        context_after_text: None,
                        start_segment_sequence: 1,
                        end_segment_sequence: 2,
                        segment_count: 2,
                        source_word_count: 6,
                        source_character_count: 29,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewTranslationChunk {
                        id: "doc_qa_001_chunk_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "Hold the line.\n\nThe flame must endure.".to_owned(),
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
                    NewTranslationChunk {
                        id: "doc_qa_001_chunk_0003".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 3,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "Keep the rune covered.".to_owned(),
                        context_before_text: None,
                        context_after_text: None,
                        start_segment_sequence: 5,
                        end_segment_sequence: 5,
                        segment_count: 1,
                        source_word_count: 4,
                        source_character_count: 22,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                ],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0001".to_owned(),
                        segment_id: "seg_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0001".to_owned(),
                        segment_id: "seg_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0002".to_owned(),
                        segment_id: "seg_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0002".to_owned(),
                        segment_id: "seg_0004".to_owned(),
                        segment_sequence: 4,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0003".to_owned(),
                        segment_id: "seg_0005".to_owned(),
                        segment_sequence: 5,
                        position: 1,
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
                job_id: Some(JOB_ID.to_owned()),
                action_type: TRANSLATE_DOCUMENT_ACTION_TYPE.to_owned(),
                status: "completed".to_owned(),
                input_payload: Some("{\"job\":\"translate_document\"}".to_owned()),
                output_payload: Some("{\"status\":\"completed_with_errors\"}".to_owned()),
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
                chunk_id: Some("doc_qa_001_chunk_0001".to_owned()),
                job_id: Some(JOB_ID.to_owned()),
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
                        target_text: "Vigilad la puerta.".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_0002".to_owned(),
                        target_text: "Mantened la línea.".to_owned(),
                    },
                ],
                NOW + 20,
            )
            .expect("first chunk translation should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_chunk_0002".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_qa_001_chunk_0002".to_owned()),
                job_id: Some(JOB_ID.to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":2}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 21,
                completed_at: None,
                created_at: NOW + 21,
                updated_at: NOW + 21,
            })
            .expect("second chunk task run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_chunk_0002",
                "{\"translations\":[3,4]}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_0003".to_owned(),
                        target_text: "Sosteneos.".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_0004".to_owned(),
                        target_text: "La llama debe perdurar.".to_owned(),
                    },
                ],
                NOW + 40,
            )
            .expect("second chunk translation should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_chunk_0003".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_qa_001_chunk_0003".to_owned()),
                job_id: Some(JOB_ID.to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":3}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 41,
                completed_at: None,
                created_at: NOW + 41,
                updated_at: NOW + 41,
            })
            .expect("third chunk task run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_failed(
                "task_chunk_0003",
                "The model request failed.",
                None,
                NOW + 50,
            )
            .expect("third chunk failure should persist");
    }

    #[test]
    fn run_document_consistency_qa_rejects_invalid_identifiers() {
        let fixture = create_runtime_fixture();

        let error = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: " ".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
        )
        .expect_err("invalid ids should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("project id"));
    }

    #[test]
    fn run_document_consistency_qa_rejects_unknown_documents() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);

        let error = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: "doc_missing_001".to_owned(),
                job_id: None,
            },
            &fixture.runtime,
        )
        .expect_err("unknown documents should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("does not exist"));
    }

    #[test]
    fn run_document_consistency_qa_generates_findings_for_partial_documents() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);

        let result = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document QA should succeed");

        assert_eq!(
            result.reconstructed_status,
            RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL
        );
        assert_eq!(
            result.reconstructed_content_source,
            RECONSTRUCTED_CONTENT_SOURCE_MIXED
        );
        assert_eq!(result.generated_findings.len(), 4);
        assert!(result.generated_findings.iter().any(
            |finding| finding.finding_type == DOCUMENT_QA_FINDING_TYPE_SOURCE_FALLBACK_SEGMENT
        ));
        assert!(result
            .generated_findings
            .iter()
            .any(|finding| finding.finding_type
                == DOCUMENT_QA_FINDING_TYPE_PARTIAL_BLOCK_TRANSLATION));
        assert!(result
            .generated_findings
            .iter()
            .any(|finding| finding.finding_type == DOCUMENT_QA_FINDING_TYPE_CHUNK_EXECUTION_ERROR));
    }

    #[test]
    fn run_document_consistency_qa_generates_neighbor_chunk_drift_findings() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);

        let result = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document QA should succeed");

        let drift_finding = result
            .generated_findings
            .iter()
            .find(|finding| {
                finding.finding_type == DOCUMENT_QA_FINDING_TYPE_NEIGHBOR_CHUNK_TRANSLATION_DRIFT
            })
            .expect("neighbor chunk drift finding should exist");

        assert_eq!(
            drift_finding.chunk_id.as_deref(),
            Some("doc_qa_001_chunk_0002")
        );
        assert_eq!(
            drift_finding.task_run_id.as_deref(),
            Some("task_chunk_0002")
        );
        assert_eq!(drift_finding.job_id.as_deref(), Some(JOB_ID));
    }

    #[test]
    fn run_document_consistency_qa_persists_findings_with_traceability() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);

        let _ = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document QA should succeed");

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let findings = QaFindingRepository::new(&mut connection)
            .list_by_document(DOCUMENT_ID)
            .expect("persisted findings should load");

        assert_eq!(findings.len(), 4);

        let fallback_finding = findings
            .iter()
            .find(|finding| {
                finding.finding_type == DOCUMENT_QA_FINDING_TYPE_SOURCE_FALLBACK_SEGMENT
            })
            .expect("fallback finding should persist");

        assert_eq!(
            fallback_finding.chunk_id.as_deref(),
            Some("doc_qa_001_chunk_0003")
        );
        assert_eq!(
            fallback_finding.task_run_id.as_deref(),
            Some("task_chunk_0003")
        );
        assert_eq!(fallback_finding.job_id.as_deref(), Some(JOB_ID));
        assert_eq!(fallback_finding.status, QA_FINDING_STATUS_OPEN);
    }

    #[test]
    fn run_document_consistency_qa_avoids_duplicate_findings_on_rerun() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);

        let first = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("first QA run should succeed");
        let second = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("second QA run should succeed");

        let first_ids = first
            .generated_findings
            .iter()
            .map(|finding| finding.id.clone())
            .collect::<Vec<_>>();
        let second_ids = second
            .generated_findings
            .iter()
            .map(|finding| finding.id.clone())
            .collect::<Vec<_>>();

        assert_eq!(first_ids, second_ids);

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let findings = QaFindingRepository::new(&mut connection)
            .list_by_document(DOCUMENT_ID)
            .expect("persisted findings should load");

        assert_eq!(findings.len(), 4);
    }

    #[test]
    fn list_document_qa_findings_filters_by_job_id() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);

        let _ = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document QA should succeed");

        let listed = list_document_qa_findings_with_runtime(
            ListDocumentQaFindingsInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("QA findings listing should succeed");

        assert_eq!(listed.findings.len(), 4);
        assert!(listed
            .findings
            .iter()
            .all(|finding| finding.job_id.as_deref() == Some(JOB_ID)));
    }

    #[test]
    fn run_document_consistency_qa_tracks_stale_chunk_runs_as_orphaned_findings() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[NewTranslationChunk {
                    id: "doc_qa_001_chunk_0101".to_owned(),
                    document_id: DOCUMENT_ID.to_owned(),
                    sequence: 101,
                    builder_version: "tr12-basic-v1".to_owned(),
                    strategy: "section-aware-fixed-word-target-v1".to_owned(),
                    source_text: "Merged document.".to_owned(),
                    context_before_text: None,
                    context_after_text: None,
                    start_segment_sequence: 1,
                    end_segment_sequence: 5,
                    segment_count: 5,
                    source_word_count: 17,
                    source_character_count: 92,
                    created_at: NOW + 100,
                    updated_at: NOW + 100,
                }],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0101".to_owned(),
                        segment_id: "seg_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0101".to_owned(),
                        segment_id: "seg_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0101".to_owned(),
                        segment_id: "seg_0003".to_owned(),
                        segment_sequence: 3,
                        position: 3,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0101".to_owned(),
                        segment_id: "seg_0004".to_owned(),
                        segment_sequence: 4,
                        position: 4,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_qa_001_chunk_0101".to_owned(),
                        segment_id: "seg_0005".to_owned(),
                        segment_sequence: 5,
                        position: 5,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                ],
            )
            .expect("chunk replacement should persist");
        drop(connection);

        let result = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document QA should succeed after chunk rebuild");

        assert!(result
            .generated_findings
            .iter()
            .any(|finding| finding.finding_type == "orphaned_chunk_task_run"));
    }
}
