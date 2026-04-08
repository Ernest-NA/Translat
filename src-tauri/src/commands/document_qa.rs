use std::collections::{BTreeMap, HashMap, HashSet};

use rusqlite::params;
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
use crate::persistence::documents::DocumentRepository;
use crate::persistence::qa_findings::QaFindingRepository;
use crate::persistence::task_runs::TaskRunRepository;
use crate::qa_findings::{
    NewQaFinding, QaFindingSummary, QA_FINDING_SEVERITY_HIGH, QA_FINDING_SEVERITY_LOW,
    QA_FINDING_SEVERITY_MEDIUM, QA_FINDING_STATUS_OPEN,
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

impl QaFindingDraft {
    fn to_new_qa_finding(&self, document_id: &str, executed_at: i64) -> NewQaFinding {
        NewQaFinding {
            id: self.id.clone(),
            document_id: document_id.to_owned(),
            chunk_id: self.chunk_id.clone(),
            task_run_id: self.task_run_id.clone(),
            job_id: self.job_id.clone(),
            finding_type: self.finding_type.clone(),
            severity: self.severity.clone(),
            status: QA_FINDING_STATUS_OPEN.to_owned(),
            message: self.message.clone(),
            details: self.details.clone(),
            created_at: executed_at,
            updated_at: executed_at,
        }
    }
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
    finding_scope_token: String,
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
    let generated_findings = persist_document_consistency_findings(
        &mut connection,
        &document_id,
        &trace_context.finding_scope_token,
        &finding_drafts,
        executed_at,
    )?;

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
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for listing QA findings.",
            Some(error.to_string()),
        )
    })?;
    validate_document_scope(&mut connection, &project_id, &document_id)?;
    validate_listing_job_scope(&mut connection, &document_id, job_id.as_deref())?;
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

fn persist_document_consistency_findings(
    connection: &mut rusqlite::Connection,
    document_id: &str,
    finding_scope_token: &str,
    finding_drafts: &[QaFindingDraft],
    executed_at: i64,
) -> Result<Vec<QaFindingSummary>, DesktopCommandError> {
    let transaction = connection.transaction().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not start the document QA persistence transaction.",
            Some(error.to_string()),
        )
    })?;

    for draft in finding_drafts {
        let finding = draft.to_new_qa_finding(document_id, executed_at);

        validate_document_links_in_transaction(&transaction, &finding).map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not validate document QA finding links.",
                Some(error.to_string()),
            )
        })?;

        transaction
            .execute(
                r#"
                INSERT INTO qa_findings (
                  id,
                  document_id,
                  chunk_id,
                  task_run_id,
                  job_id,
                  finding_type,
                  severity,
                  status,
                  message,
                  details,
                  created_at,
                  updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(id) DO UPDATE SET
                  document_id = excluded.document_id,
                  chunk_id = excluded.chunk_id,
                  task_run_id = excluded.task_run_id,
                  job_id = excluded.job_id,
                  finding_type = excluded.finding_type,
                  severity = excluded.severity,
                  status = excluded.status,
                  message = excluded.message,
                  details = excluded.details,
                  updated_at = excluded.updated_at
                "#,
                params![
                    finding.id,
                    finding.document_id,
                    finding.chunk_id,
                    finding.task_run_id,
                    finding.job_id,
                    finding.finding_type,
                    finding.severity,
                    finding.status,
                    finding.message,
                    finding.details,
                    finding.created_at,
                    finding.updated_at
                ],
            )
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not persist document QA findings.",
                    Some(error.to_string()),
                )
            })?;
    }

    let retained_ids = finding_drafts
        .iter()
        .map(|draft| draft.id.as_str())
        .collect::<HashSet<_>>();
    let stale_ids = load_generated_document_qa_ids(&transaction, document_id, finding_scope_token)?
        .into_iter()
        .filter(|finding_id| !retained_ids.contains(finding_id.as_str()))
        .collect::<Vec<_>>();

    for stale_id in stale_ids {
        transaction
            .execute("DELETE FROM qa_findings WHERE id = ?1", [stale_id.as_str()])
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not retire stale document QA findings.",
                    Some(error.to_string()),
                )
            })?;
    }

    transaction.commit().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not commit document QA findings.",
            Some(error.to_string()),
        )
    })?;

    let mut repository = QaFindingRepository::new(connection);
    let mut generated_findings = Vec::new();

    for draft in finding_drafts {
        generated_findings.push(
            repository
                .load_by_id(&draft.id)
                .map_err(|error| {
                    DesktopCommandError::internal(
                        "The desktop shell could not reload persisted document QA findings.",
                        Some(error.to_string()),
                    )
                })?
                .ok_or_else(|| {
                    DesktopCommandError::internal(
                        "The desktop shell could not reload a persisted document QA finding.",
                        Some(draft.id.clone()),
                    )
                })?,
        );
    }

    Ok(generated_findings)
}

fn load_generated_document_qa_ids(
    transaction: &rusqlite::Transaction<'_>,
    document_id: &str,
    finding_scope_token: &str,
) -> Result<Vec<String>, DesktopCommandError> {
    let mut statement = transaction
        .prepare(
            r#"
            SELECT id
            FROM qa_findings
            WHERE document_id = ?1 AND id LIKE ?2
            ORDER BY id ASC
            "#,
        )
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not inspect existing document QA findings.",
                Some(error.to_string()),
            )
        })?;
    let rows = statement
        .query_map(
            params![
                document_id,
                format!("{QA_FINDING_ID_PREFIX}_{document_id}_{finding_scope_token}_%")
            ],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not read existing document QA findings.",
                Some(error.to_string()),
            )
        })?;
    let mut ids = Vec::new();

    for row in rows {
        ids.push(row.map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not decode an existing document QA finding id.",
                Some(error.to_string()),
            )
        })?);
    }

    Ok(ids)
}

fn validate_document_links_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    qa_finding: &NewQaFinding,
) -> Result<(), String> {
    validate_linked_document_in_transaction(
        transaction,
        "chunk",
        qa_finding.id.as_str(),
        qa_finding.document_id.as_str(),
        qa_finding.chunk_id.as_deref(),
        "SELECT document_id FROM translation_chunks WHERE id = ?1",
    )?;
    validate_linked_document_in_transaction(
        transaction,
        "task run",
        qa_finding.id.as_str(),
        qa_finding.document_id.as_str(),
        qa_finding.task_run_id.as_deref(),
        "SELECT document_id FROM task_runs WHERE id = ?1",
    )?;

    Ok(())
}

fn validate_linked_document_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    linked_entity: &str,
    finding_id: &str,
    document_id: &str,
    linked_id: Option<&str>,
    query: &str,
) -> Result<(), String> {
    let Some(linked_id) = linked_id else {
        return Ok(());
    };

    let linked_document_id = transaction
        .query_row(query, [linked_id], |row| row.get::<_, String>(0))
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => format!(
                "The document QA flow could not find {linked_entity} {linked_id} for finding {finding_id}."
            ),
            other => format!(
                "The document QA flow could not validate {linked_entity} {linked_id} for finding {finding_id}: {other}"
            ),
        })?;

    if linked_document_id != document_id {
        return Err(format!(
            "The document QA flow received {linked_entity} {linked_id} for document {linked_document_id}, but finding {finding_id} targets document {document_id}."
        ));
    }

    Ok(())
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

fn validate_document_scope(
    connection: &mut rusqlite::Connection,
    project_id: &str,
    document_id: &str,
) -> Result<(), DesktopCommandError> {
    let document = DocumentRepository::new(connection)
        .load_processing_record(project_id, document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not validate the selected document for document QA.",
                Some(error.to_string()),
            )
        })?;

    if document.is_none() {
        return Err(DesktopCommandError::validation(
            "The selected document does not exist in the active project.",
            None,
        ));
    }

    Ok(())
}

fn validate_listing_job_scope(
    connection: &mut rusqlite::Connection,
    document_id: &str,
    job_id: Option<&str>,
) -> Result<(), DesktopCommandError> {
    let Some(job_id) = job_id else {
        return Ok(());
    };

    let has_persisted_findings = QaFindingRepository::new(connection)
        .list_by_job_id(job_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not inspect persisted QA findings for the selected job.",
                Some(error.to_string()),
            )
        })?
        .into_iter()
        .any(|finding| finding.document_id == document_id);

    if has_persisted_findings {
        return Ok(());
    }

    let has_task_runs = TaskRunRepository::new(connection)
        .list_by_job_id(job_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not inspect task runs for the selected QA job.",
                Some(error.to_string()),
            )
        })?
        .into_iter()
        .any(|task_run| task_run.document_id == document_id);

    if has_task_runs {
        return Ok(());
    }

    Err(DesktopCommandError::validation(
        "The selected QA job does not exist for the active document.",
        None,
    ))
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
            finding_scope_token: build_finding_scope_token(
                reconstructed_document
                    .trace
                    .latest_document_task_run
                    .as_ref()
                    .and_then(|task_run| task_run.job_id.as_deref()),
            ),
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
        finding_scope_token: build_finding_scope_token(requested_job_id),
    }
}

fn build_finding_scope_token(job_id: Option<&str>) -> String {
    match job_id {
        Some(job_id) => job_id.to_owned(),
        None => "document_current".to_owned(),
    }
}

fn build_finding_drafts(
    reconstructed_document: &ReconstructedDocument,
    trace_context: &QaTraceContext,
) -> Vec<QaFindingDraft> {
    let mut findings = BTreeMap::new();
    let finding_id_prefix = format!(
        "{QA_FINDING_ID_PREFIX}_{document_id}_{scope}",
        document_id = reconstructed_document.document_id,
        scope = trace_context.finding_scope_token.as_str()
    );
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
                        id: format!("{finding_id_prefix}_partial_block_translation_{}", block.id,),
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
                                "{finding_id_prefix}_source_fallback_segment_{}",
                                segment.id,
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
                    id: format!("{finding_id_prefix}_chunk_execution_error_{}", task_run.id,),
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
                    "{finding_id_prefix}_orphaned_chunk_task_run_{}",
                    task_run.id,
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
                        "{finding_id_prefix}_neighbor_chunk_translation_drift_{}_{}",
                        previous_segment.id,
                        current_segment.id,
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

    use super::{
        list_document_qa_findings_with_runtime, persist_document_consistency_findings,
        run_document_consistency_qa_with_runtime, QaFindingDraft,
    };
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
    use crate::qa_findings::{QA_FINDING_SEVERITY_MEDIUM, QA_FINDING_STATUS_OPEN};
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
    fn run_document_consistency_qa_preserves_findings_across_distinct_jobs() {
        let second_job_id = "job_translate_doc_qa_002";
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
        .expect("first job QA should succeed");

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_doc_0002".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                job_id: Some(second_job_id.to_owned()),
                action_type: TRANSLATE_DOCUMENT_ACTION_TYPE.to_owned(),
                status: "completed".to_owned(),
                input_payload: Some("{\"job\":\"translate_document_retry\"}".to_owned()),
                output_payload: Some("{\"status\":\"completed_with_errors\"}".to_owned()),
                error_message: None,
                started_at: NOW + 90,
                completed_at: Some(NOW + 120),
                created_at: NOW + 90,
                updated_at: NOW + 120,
            })
            .expect("second document task run should persist");
        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_chunk_0011".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_qa_001_chunk_0001".to_owned()),
                job_id: Some(second_job_id.to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":1,\"job\":2}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 91,
                completed_at: None,
                created_at: NOW + 91,
                updated_at: NOW + 91,
            })
            .expect("second job chunk 1 run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_chunk_0011",
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
                NOW + 100,
            )
            .expect("second job chunk 1 projection should persist");
        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_chunk_0012".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_qa_001_chunk_0002".to_owned()),
                job_id: Some(second_job_id.to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":2,\"job\":2}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 101,
                completed_at: None,
                created_at: NOW + 101,
                updated_at: NOW + 101,
            })
            .expect("second job chunk 2 run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_chunk_0012",
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
                NOW + 110,
            )
            .expect("second job chunk 2 projection should persist");
        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_chunk_0013".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_qa_001_chunk_0003".to_owned()),
                job_id: Some(second_job_id.to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":3,\"job\":2}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 111,
                completed_at: None,
                created_at: NOW + 111,
                updated_at: NOW + 111,
            })
            .expect("second job chunk 3 run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_failed(
                "task_chunk_0013",
                "The model request failed again.",
                None,
                NOW + 115,
            )
            .expect("second job chunk 3 failure should persist");
        drop(connection);

        let second = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(second_job_id.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("second job QA should succeed");

        assert_eq!(first.generated_findings.len(), 4);
        assert_eq!(second.generated_findings.len(), 4);
        assert!(
            first.generated_findings[0].id != second.generated_findings[0].id,
            "distinct jobs must keep distinct finding ids"
        );

        let listed_first = list_document_qa_findings_with_runtime(
            ListDocumentQaFindingsInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("first job findings should remain listable");
        let listed_second = list_document_qa_findings_with_runtime(
            ListDocumentQaFindingsInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(second_job_id.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("second job findings should be listable");

        assert_eq!(listed_first.findings.len(), 4);
        assert_eq!(listed_second.findings.len(), 4);
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
    fn list_document_qa_findings_rejects_unknown_job_ids() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);

        let error = list_document_qa_findings_with_runtime(
            ListDocumentQaFindingsInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_missing_001".to_owned()),
            },
            &fixture.runtime,
        )
        .expect_err("unknown QA job ids should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("does not exist"));
    }

    #[test]
    fn list_document_qa_findings_uses_persisted_history_when_trace_rows_are_gone() {
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

        let connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        connection
            .execute(
                "DELETE FROM task_runs WHERE document_id = ?1",
                [DOCUMENT_ID],
            )
            .expect("task runs should be compacted for listing regression");
        connection
            .execute(
                "DELETE FROM translation_chunks WHERE document_id = ?1",
                [DOCUMENT_ID],
            )
            .expect("chunks should be compacted for listing regression");
        drop(connection);

        let listed = list_document_qa_findings_with_runtime(
            ListDocumentQaFindingsInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("persisted QA findings should still be listable");

        assert_eq!(listed.findings.len(), 4);
        assert!(listed
            .findings
            .iter()
            .all(|finding| finding.job_id.as_deref() == Some(JOB_ID)));
    }

    #[test]
    fn run_document_consistency_qa_retires_stale_findings_after_document_changes() {
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
        .expect("first QA run should succeed");

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_chunk_0004".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_qa_001_chunk_0003".to_owned()),
                job_id: Some(JOB_ID.to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":3,\"retry\":true}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 61,
                completed_at: None,
                created_at: NOW + 61,
                updated_at: NOW + 61,
            })
            .expect("retry chunk task run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_chunk_0004",
                "{\"translations\":[5]}",
                &[SegmentTranslationWrite {
                    segment_id: "seg_0005".to_owned(),
                    target_text: "Mantened la runa cubierta.".to_owned(),
                }],
                NOW + 80,
            )
            .expect("retry chunk translation should persist");
        drop(connection);

        let rerun = run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("second QA run should succeed");

        assert_eq!(rerun.generated_findings.len(), 1);
        assert_eq!(
            rerun.generated_findings[0].finding_type,
            DOCUMENT_QA_FINDING_TYPE_NEIGHBOR_CHUNK_TRANSLATION_DRIFT
        );

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let findings = QaFindingRepository::new(&mut connection)
            .list_by_document(DOCUMENT_ID)
            .expect("persisted findings should load");

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].finding_type,
            DOCUMENT_QA_FINDING_TYPE_NEIGHBOR_CHUNK_TRANSLATION_DRIFT
        );
    }

    #[test]
    fn document_qa_persistence_rolls_back_when_a_finding_write_fails() {
        let fixture = create_runtime_fixture();
        seed_document_qa_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");

        let drafts = vec![
            QaFindingDraft {
                id: "qaf_tr21_doc_qa_001_valid".to_owned(),
                chunk_id: Some("doc_qa_001_chunk_0001".to_owned()),
                task_run_id: Some("task_chunk_0001".to_owned()),
                job_id: Some(JOB_ID.to_owned()),
                finding_type: DOCUMENT_QA_FINDING_TYPE_NEIGHBOR_CHUNK_TRANSLATION_DRIFT.to_owned(),
                severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                message: "valid".to_owned(),
                details: None,
            },
            QaFindingDraft {
                id: "qaf_tr21_doc_qa_001_invalid".to_owned(),
                chunk_id: Some("chunk_missing_001".to_owned()),
                task_run_id: None,
                job_id: Some(JOB_ID.to_owned()),
                finding_type: DOCUMENT_QA_FINDING_TYPE_PARTIAL_BLOCK_TRANSLATION.to_owned(),
                severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                message: "invalid".to_owned(),
                details: None,
            },
        ];

        let error = persist_document_consistency_findings(
            &mut connection,
            DOCUMENT_ID,
            JOB_ID,
            &drafts,
            NOW,
        )
        .expect_err("an invalid linked chunk should abort the QA persistence transaction");

        assert_eq!(error.code, "DESKTOP_COMMAND_FAILED");
        assert!(error.message.contains("could not validate"));

        let findings = QaFindingRepository::new(&mut connection)
            .list_by_document(DOCUMENT_ID)
            .expect("findings should reload after rollback");

        assert!(findings.is_empty());
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
