use std::collections::{BTreeSet, HashMap, HashSet};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Map, Value};
use tauri::State;

use crate::commands::reconstructed_documents::{
    current_timestamp, load_reconstructed_document, validate_identifier,
};
use crate::error::DesktopCommandError;
use crate::finding_review::{
    InspectQaFindingInput, QaFindingChunkAnchor, QaFindingRetranslationResult,
    QaFindingReviewContext, RetranslateChunkFromQaFindingInput,
};
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::documents::DocumentRepository;
use crate::persistence::qa_findings::QaFindingRepository;
use crate::persistence::task_runs::TaskRunRepository;
use crate::persistence::translation_chunks::TranslationChunkRepository;
use crate::qa_findings::QaFindingSummary;
use crate::reconstructed_documents::{
    ReconstructedDocument, ReconstructedDocumentBlock, ReconstructedDocumentChunkTrace,
    ReconstructedSegment,
};
use crate::rule_sets::RULE_ACTION_SCOPE_RETRANSLATION;
use crate::task_runs::TaskRunSummary;
use crate::translate_chunk::{
    OpenAiTranslateChunkExecutor, TranslateChunkExecutor, TranslateChunkInput, TranslateChunkResult,
};
use crate::translation_chunks::{TranslationChunkSegmentSummary, TranslationChunkSummary};

const REVIEW_ACTION_KIND_RETRANSLATE_CHUNK: &str = "retranslate_chunk";
const RESOLUTION_KIND_UNRESOLVED: &str = "unresolved";

#[derive(Debug, Clone)]
struct AnchorCandidate {
    chunk_id: String,
    resolution_kind: &'static str,
    resolution_message: String,
}

#[derive(Debug, Clone)]
struct ResolvedAnchor {
    anchor: QaFindingChunkAnchor,
    chunk: Option<TranslationChunkSummary>,
    chunk_segments: Vec<TranslationChunkSegmentSummary>,
    latest_chunk_task_run: Option<TaskRunSummary>,
    related_block: Option<ReconstructedDocumentBlock>,
    related_segments: Vec<ReconstructedSegment>,
}

#[tauri::command]
pub fn inspect_qa_finding(
    input: InspectQaFindingInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<QaFindingReviewContext, DesktopCommandError> {
    inspect_qa_finding_with_runtime(input, database_runtime.inner())
}

#[tauri::command]
pub fn retranslate_chunk_from_qa_finding(
    input: RetranslateChunkFromQaFindingInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<QaFindingRetranslationResult, DesktopCommandError> {
    let executor = OpenAiTranslateChunkExecutor::from_environment()?;

    retranslate_chunk_from_qa_finding_with_runtime_and_executor(
        input,
        database_runtime.inner(),
        &executor,
    )
}

pub(crate) fn inspect_qa_finding_with_runtime(
    input: InspectQaFindingInput,
    database_runtime: &DatabaseRuntime,
) -> Result<QaFindingReviewContext, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let finding_id = validate_identifier(&input.finding_id, "finding id")?;
    let inspected_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for finding review.",
            Some(error.to_string()),
        )
    })?;
    validate_document_scope(&mut connection, &project_id, &document_id)?;
    let finding = load_document_finding(&mut connection, &document_id, &finding_id)?;
    let reconstructed_document = load_reconstructed_document(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        inspected_at,
    )?;
    let resolved_anchor =
        resolve_finding_anchor(&mut connection, &finding, &reconstructed_document)?;

    Ok(QaFindingReviewContext {
        project_id,
        document_id,
        finding,
        anchor: resolved_anchor.anchor,
        chunk: resolved_anchor.chunk,
        chunk_segments: resolved_anchor.chunk_segments,
        latest_chunk_task_run: resolved_anchor.latest_chunk_task_run,
        latest_document_task_run: reconstructed_document.trace.latest_document_task_run,
        related_block: resolved_anchor.related_block,
        related_segments: resolved_anchor.related_segments,
    })
}

pub(crate) fn retranslate_chunk_from_qa_finding_with_runtime_and_executor<
    E: TranslateChunkExecutor,
>(
    input: RetranslateChunkFromQaFindingInput,
    database_runtime: &DatabaseRuntime,
    executor: &E,
) -> Result<QaFindingRetranslationResult, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let finding_id = validate_identifier(&input.finding_id, "finding id")?;
    let correction_job_id = input
        .job_id
        .map(|value| validate_identifier(&value, "job id"))
        .transpose()?;
    let triggered_at = current_timestamp()?;

    let inspection = inspect_qa_finding_with_runtime(
        InspectQaFindingInput {
            project_id: project_id.clone(),
            document_id: document_id.clone(),
            finding_id: finding_id.clone(),
        },
        database_runtime,
    )?;
    let chunk_id = inspection.anchor.chunk_id.clone().ok_or_else(|| {
        DesktopCommandError::validation(
            "The selected QA finding does not resolve to a current chunk that can be retranslated.",
            None,
        )
    })?;
    let correction_job_id =
        correction_job_id.unwrap_or_else(|| generate_review_job_id(&finding_id, triggered_at));

    let translate_result =
        crate::commands::translate_chunk::translate_chunk_with_runtime_and_executor_for_scope(
            TranslateChunkInput {
                project_id: project_id.clone(),
                document_id: document_id.clone(),
                chunk_id,
                job_id: Some(correction_job_id.clone()),
            },
            database_runtime,
            executor,
            RULE_ACTION_SCOPE_RETRANSLATION,
        )?;
    let (finding, review_action_persisted, review_action_warning) =
        match database_runtime.open_connection() {
            Ok(mut connection) => match append_review_action_to_finding(
                &mut connection,
                &inspection.finding.id,
                &inspection.anchor,
                &translate_result,
                triggered_at,
            ) {
                Ok(finding) => (finding, true, None),
                Err(error) => (
                    inspection.finding.clone(),
                    false,
                    Some(format!(
                        "The chunk was retranslated, but the review action could not be attached to the QA finding: {}",
                        error.message
                    )),
                ),
            },
            Err(error) => (
                inspection.finding.clone(),
                false,
                Some(format!(
                    "The chunk was retranslated, but the desktop shell could not reopen the encrypted database to append review metadata: {error}"
                )),
            ),
        };

    Ok(QaFindingRetranslationResult {
        project_id,
        document_id,
        finding,
        anchor: inspection.anchor,
        correction_job_id,
        review_action_persisted,
        review_action_warning,
        translate_result,
    })
}

fn append_review_action_to_finding(
    connection: &mut rusqlite::Connection,
    finding_id: &str,
    anchor: &QaFindingChunkAnchor,
    translate_result: &TranslateChunkResult,
    updated_at: i64,
) -> Result<QaFindingSummary, DesktopCommandError> {
    let latest_finding = QaFindingRepository::new(connection)
        .load_by_id(finding_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not reload the latest QA finding state before appending review metadata.",
                Some(error.to_string()),
            )
        })?
        .ok_or_else(|| {
            DesktopCommandError::validation(
                "The selected QA finding no longer exists, so review metadata could not be appended.",
                None,
            )
        })?;
    let next_details = merge_review_action_details(
        latest_finding.details.as_deref(),
        json!({
            "kind": REVIEW_ACTION_KIND_RETRANSLATE_CHUNK,
            "triggeredAt": updated_at,
            "findingId": latest_finding.id,
            "resolvedChunkId": anchor.chunk_id,
            "resolvedChunkSequence": anchor.chunk_sequence,
            "resolutionKind": anchor.resolution_kind,
            "taskRunId": translate_result.task_run.id,
            "jobId": translate_result.task_run.job_id,
            "provider": translate_result.provider,
            "model": translate_result.model,
            "translatedSegmentIds": translate_result
                .translated_segments
                .iter()
                .map(|segment| segment.segment_id.clone())
                .collect::<Vec<_>>(),
        }),
    )?;

    QaFindingRepository::new(connection)
        .update_details(finding_id, next_details.as_deref(), updated_at)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not persist review action metadata for the selected QA finding.",
                Some(error.to_string()),
            )
        })?
        .ok_or_else(|| {
            DesktopCommandError::validation(
                "The selected QA finding no longer exists, so review metadata could not be appended.",
                None,
            )
        })
}

fn merge_review_action_details(
    current_details: Option<&str>,
    review_action: Value,
) -> Result<Option<String>, DesktopCommandError> {
    let mut root = match current_details {
        Some(details) => match serde_json::from_str::<Value>(details) {
            Ok(Value::Object(object)) => Value::Object(object),
            Ok(other) => {
                let mut object = Map::new();
                object.insert("originalDetails".to_owned(), other);
                Value::Object(object)
            }
            Err(_) => {
                let mut object = Map::new();
                object.insert(
                    "legacyDetails".to_owned(),
                    Value::String(details.to_owned()),
                );
                Value::Object(object)
            }
        },
        None => Value::Object(Map::new()),
    };

    let Some(root_object) = root.as_object_mut() else {
        return Err(DesktopCommandError::internal(
            "The desktop shell produced invalid review metadata while updating a QA finding.",
            None,
        ));
    };

    match root_object.get_mut("reviewActions") {
        Some(Value::Array(actions)) => actions.push(review_action),
        Some(existing) => {
            let previous = existing.clone();
            *existing = Value::Array(vec![previous, review_action]);
        }
        None => {
            root_object.insert(
                "reviewActions".to_owned(),
                Value::Array(vec![review_action]),
            );
        }
    }

    serde_json::to_string(&root).map(Some).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not serialize review action metadata for a QA finding.",
            Some(error.to_string()),
        )
    })
}

fn resolve_finding_anchor(
    connection: &mut rusqlite::Connection,
    finding: &QaFindingSummary,
    reconstructed_document: &ReconstructedDocument,
) -> Result<ResolvedAnchor, DesktopCommandError> {
    let mut chunk_repository = TranslationChunkRepository::new(connection);
    let chunks = chunk_repository
        .list_chunks_by_document(&finding.document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load current chunks for finding review.",
                Some(error.to_string()),
            )
        })?;
    let chunk_segments = chunk_repository
        .list_chunk_segments_by_document(&finding.document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load current chunk links for finding review.",
                Some(error.to_string()),
            )
        })?;
    let chunk_map = chunks
        .iter()
        .map(|chunk| (chunk.id.as_str(), chunk))
        .collect::<HashMap<_, _>>();
    let chunk_segments_by_chunk = chunk_segments.iter().fold(
        HashMap::<&str, Vec<TranslationChunkSegmentSummary>>::new(),
        |mut grouped, chunk_segment| {
            grouped
                .entry(chunk_segment.chunk_id.as_str())
                .or_default()
                .push(chunk_segment.clone());
            grouped
        },
    );
    let trace_by_chunk = reconstructed_document
        .trace
        .chunks
        .iter()
        .map(|trace| (trace.chunk_id.as_str(), trace))
        .collect::<HashMap<_, _>>();
    let segment_map = reconstructed_document
        .blocks
        .iter()
        .flat_map(|block| block.segments.iter())
        .map(|segment| (segment.id.as_str(), segment))
        .collect::<HashMap<_, _>>();
    let block_map = reconstructed_document
        .blocks
        .iter()
        .map(|block| (block.id.as_str(), block))
        .collect::<HashMap<_, _>>();
    let parsed_details = parse_finding_details(finding.details.as_deref());
    let related_segment_ids = collect_related_segment_ids(parsed_details.as_ref(), &segment_map);
    let related_block_id = resolve_related_block_id(
        parsed_details.as_ref(),
        &related_segment_ids,
        reconstructed_document,
    );
    let candidates = collect_anchor_candidates(
        connection,
        finding,
        parsed_details.as_ref(),
        &related_segment_ids,
        related_block_id.as_deref(),
        &segment_map,
        &block_map,
    )?;

    for candidate in candidates {
        let Some(chunk) = chunk_map.get(candidate.chunk_id.as_str()) else {
            continue;
        };
        let related_segments = build_related_segments(
            &related_segment_ids,
            &segment_map,
            trace_by_chunk.get(chunk.id.as_str()),
        );
        let related_block = related_block_id
            .as_deref()
            .and_then(|block_id| block_map.get(block_id))
            .map(|block| (*block).clone())
            .or_else(|| {
                related_segments.first().and_then(|segment| {
                    reconstructed_document
                        .blocks
                        .iter()
                        .find(|block| block.segment_ids.iter().any(|id| id == &segment.id))
                        .cloned()
                })
            });

        return Ok(ResolvedAnchor {
            anchor: QaFindingChunkAnchor {
                finding_id: finding.id.clone(),
                chunk_id: Some(chunk.id.clone()),
                chunk_sequence: Some(chunk.sequence),
                resolution_kind: candidate.resolution_kind.to_owned(),
                resolution_message: candidate.resolution_message,
                can_retranslate: true,
            },
            chunk: Some((*chunk).clone()),
            chunk_segments: chunk_segments_by_chunk
                .get(chunk.id.as_str())
                .cloned()
                .unwrap_or_default(),
            latest_chunk_task_run: trace_by_chunk
                .get(chunk.id.as_str())
                .and_then(|trace| trace.latest_task_run.clone()),
            related_block,
            related_segments,
        });
    }

    Ok(ResolvedAnchor {
        anchor: QaFindingChunkAnchor {
            finding_id: finding.id.clone(),
            chunk_id: None,
            chunk_sequence: None,
            resolution_kind: RESOLUTION_KIND_UNRESOLVED.to_owned(),
            resolution_message:
                "This QA finding no longer maps to a current chunk. Inspect the persisted trace or rerun chunk QA after rebuilding the document state."
                    .to_owned(),
            can_retranslate: false,
        },
        chunk: None,
        chunk_segments: Vec::new(),
        latest_chunk_task_run: None,
        related_block: related_block_id
            .as_deref()
            .and_then(|block_id| block_map.get(block_id))
            .map(|block| (*block).clone()),
        related_segments: build_related_segments(&related_segment_ids, &segment_map, None),
    })
}

fn collect_anchor_candidates(
    connection: &mut rusqlite::Connection,
    finding: &QaFindingSummary,
    parsed_details: Option<&Value>,
    related_segment_ids: &[String],
    related_block_id: Option<&str>,
    segment_map: &HashMap<&str, &ReconstructedSegment>,
    block_map: &HashMap<&str, &ReconstructedDocumentBlock>,
) -> Result<Vec<AnchorCandidate>, DesktopCommandError> {
    let mut candidates = Vec::new();

    if let Some(chunk_id) = finding.chunk_id.as_deref() {
        candidates.push(AnchorCandidate {
            chunk_id: chunk_id.to_owned(),
            resolution_kind: "finding_chunk_id",
            resolution_message:
                "The QA finding already points to a persisted chunk in the current document."
                    .to_owned(),
        });
    }

    if let Some(task_run_id) = finding.task_run_id.as_deref() {
        let task_run = TaskRunRepository::new(connection)
            .load_by_id(task_run_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not reload the task run linked to a QA finding.",
                    Some(error.to_string()),
                )
            })?;

        if let Some(chunk_id) = task_run.and_then(|task_run| task_run.chunk_id) {
            candidates.push(AnchorCandidate {
                chunk_id,
                resolution_kind: "finding_task_run_chunk",
                resolution_message:
                    "The QA finding was re-anchored through the persisted task run linked to the finding."
                        .to_owned(),
            });
        }
    }

    for (key, resolution_kind, resolution_message) in [
        (
            "chunkId",
            "details_chunk_id",
            "The QA finding details include a chunk id that still exists in the current document.",
        ),
        (
            "currentChunkId",
            "details_current_chunk",
            "The QA finding details identify the current affected chunk explicitly.",
        ),
        (
            "primaryChunkId",
            "details_primary_chunk",
            "The QA finding details expose a primary chunk anchor for the affected text.",
        ),
        (
            "previousChunkId",
            "details_previous_chunk",
            "The QA finding details still point to a neighboring previous chunk.",
        ),
        (
            "originalChunkId",
            "details_original_chunk",
            "The QA finding details keep the original chunk id produced by the earlier run.",
        ),
    ] {
        if let Some(chunk_id) = parsed_details.and_then(|details| details_string(details, key)) {
            candidates.push(AnchorCandidate {
                chunk_id,
                resolution_kind,
                resolution_message: resolution_message.to_owned(),
            });
        }
    }

    for chunk_id in details_review_action_strings(parsed_details, "resolvedChunkId") {
        candidates.push(AnchorCandidate {
            chunk_id,
            resolution_kind: "review_action_resolved_chunk",
            resolution_message:
                "The QA finding was re-anchored through the latest persisted review action chunk id."
                    .to_owned(),
        });
    }

    for chunk_id in details_review_action_strings(parsed_details, "chunkId") {
        candidates.push(AnchorCandidate {
            chunk_id,
            resolution_kind: "review_action_chunk",
            resolution_message:
                "The QA finding was re-anchored through a chunk id stored in persisted review actions."
                    .to_owned(),
        });
    }

    for task_run_id in details_review_action_strings(parsed_details, "taskRunId") {
        let task_run = TaskRunRepository::new(connection)
            .load_by_id(&task_run_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not reload a review-action task run while resolving a QA finding anchor.",
                    Some(error.to_string()),
                )
            })?;

        if let Some(chunk_id) = task_run.and_then(|task_run| task_run.chunk_id) {
            candidates.push(AnchorCandidate {
                chunk_id,
                resolution_kind: "review_action_task_run_chunk",
                resolution_message:
                    "The QA finding was re-anchored through the persisted task run referenced by a review action."
                        .to_owned(),
            });
        }
    }

    for (key, resolution_kind, resolution_message) in [
        (
            "primaryChunkIds",
            "details_primary_chunk_ids",
            "The QA finding details expose current block-level primary chunks.",
        ),
        (
            "relatedChunkIds",
            "details_related_chunk_ids",
            "The QA finding details expose related chunks for the affected text.",
        ),
    ] {
        for chunk_id in parsed_details
            .map(|details| details_string_array(details, key))
            .unwrap_or_default()
            .into_iter()
            .rev()
        {
            candidates.push(AnchorCandidate {
                chunk_id,
                resolution_kind,
                resolution_message: resolution_message.to_owned(),
            });
        }
    }

    for segment_id in related_segment_ids {
        let Some(segment) = segment_map.get(segment_id.as_str()) else {
            continue;
        };

        if let Some(chunk_id) = segment.primary_chunk_id.as_ref() {
            candidates.push(AnchorCandidate {
                chunk_id: chunk_id.clone(),
                resolution_kind: "segment_primary_chunk",
                resolution_message:
                    "The QA finding was re-anchored through the primary chunk of an affected reconstructed segment."
                        .to_owned(),
            });
        }

        for chunk_id in segment.related_chunk_ids.iter().rev() {
            candidates.push(AnchorCandidate {
                chunk_id: chunk_id.clone(),
                resolution_kind: "segment_related_chunk",
                resolution_message:
                    "The QA finding was re-anchored through a related chunk attached to an affected reconstructed segment."
                        .to_owned(),
            });
        }
    }

    if let Some(block_id) = related_block_id {
        if let Some(block) = block_map.get(block_id) {
            for chunk_id in block.primary_chunk_ids.iter().rev() {
                candidates.push(AnchorCandidate {
                    chunk_id: chunk_id.clone(),
                    resolution_kind: "block_primary_chunk",
                    resolution_message:
                        "The QA finding was re-anchored through the reconstructed document block that contains the affected text."
                            .to_owned(),
                });
            }
        }
    }

    let mut seen = HashSet::new();

    Ok(candidates
        .into_iter()
        .filter(|candidate| seen.insert(candidate.chunk_id.clone()))
        .collect())
}

fn build_related_segments(
    related_segment_ids: &[String],
    segment_map: &HashMap<&str, &ReconstructedSegment>,
    chunk_trace: Option<&&ReconstructedDocumentChunkTrace>,
) -> Vec<ReconstructedSegment> {
    let mut segment_ids = if related_segment_ids.is_empty() {
        chunk_trace
            .map(|trace| trace.core_segment_ids.clone())
            .unwrap_or_default()
    } else {
        related_segment_ids.to_vec()
    };
    let mut seen = HashSet::new();
    segment_ids.retain(|segment_id| seen.insert(segment_id.clone()));

    let mut segments = segment_ids
        .into_iter()
        .filter_map(|segment_id| segment_map.get(segment_id.as_str()).cloned().cloned())
        .collect::<Vec<_>>();
    segments.sort_by_key(|segment| segment.sequence);
    segments
}

fn collect_related_segment_ids(
    parsed_details: Option<&Value>,
    segment_map: &HashMap<&str, &ReconstructedSegment>,
) -> Vec<String> {
    let mut segment_ids = BTreeSet::new();

    if let Some(parsed_details) = parsed_details {
        for key in [
            "segmentId",
            "previousSegmentId",
            "currentSegmentId",
            "translatedSegmentId",
        ] {
            if let Some(segment_id) = details_string(parsed_details, key) {
                segment_ids.insert(segment_id);
            }
        }

        for key in ["segmentIds", "fallbackSegmentIds", "translatedSegmentIds"] {
            for segment_id in details_string_array(parsed_details, key) {
                segment_ids.insert(segment_id);
            }
        }

        for segment_id in details_review_action_string_arrays(parsed_details, "translatedSegmentIds")
        {
            segment_ids.insert(segment_id);
        }
    }

    segment_ids.retain(|segment_id| segment_map.contains_key(segment_id.as_str()));
    segment_ids.into_iter().collect()
}

fn resolve_related_block_id(
    parsed_details: Option<&Value>,
    related_segment_ids: &[String],
    reconstructed_document: &ReconstructedDocument,
) -> Option<String> {
    if let Some(block_id) = parsed_details.and_then(|details| details_string(details, "blockId")) {
        return Some(block_id);
    }

    related_segment_ids.first().and_then(|segment_id| {
        reconstructed_document
            .blocks
            .iter()
            .find(|block| block.segment_ids.iter().any(|id| id == segment_id))
            .map(|block| block.id.clone())
    })
}

fn parse_finding_details(details: Option<&str>) -> Option<Value> {
    details.and_then(|details| serde_json::from_str::<Value>(details).ok())
}

fn details_string(details: &Value, key: &str) -> Option<String> {
    details
        .as_object()
        .and_then(|object| object.get(key))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn details_string_array(details: &Value, key: &str) -> Vec<String> {
    details
        .as_object()
        .and_then(|object| object.get(key))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn details_review_actions(details: Option<&Value>) -> Vec<&Value> {
    details
        .and_then(Value::as_object)
        .and_then(|object| object.get("reviewActions"))
        .and_then(Value::as_array)
        .map(|actions| actions.iter().rev().collect::<Vec<_>>())
        .unwrap_or_default()
}

fn details_review_action_strings(details: Option<&Value>, key: &str) -> Vec<String> {
    details_review_actions(details)
        .into_iter()
        .filter_map(|review_action| details_string(review_action, key))
        .collect()
}

fn details_review_action_string_arrays(details: &Value, key: &str) -> Vec<String> {
    details_review_actions(Some(details))
        .into_iter()
        .flat_map(|review_action| details_string_array(review_action, key))
        .collect()
}

fn generate_review_job_id(finding_id: &str, timestamp: i64) -> String {
    let finding_suffix = finding_id
        .chars()
        .rev()
        .filter(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
        .take(24)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    let random_part = rand::random::<u64>();

    format!(
        "review_chunk_{}_{}_{}",
        timestamp,
        URL_SAFE_NO_PAD.encode(random_part.to_le_bytes()),
        finding_suffix
    )
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
                "The desktop shell could not validate the selected document for finding review.",
                Some(error.to_string()),
            )
        })?;

    if document.is_some() {
        return Ok(());
    }

    Err(DesktopCommandError::validation(
        "The selected document does not exist for the active project.",
        None,
    ))
}

fn load_document_finding(
    connection: &mut rusqlite::Connection,
    document_id: &str,
    finding_id: &str,
) -> Result<QaFindingSummary, DesktopCommandError> {
    let finding = QaFindingRepository::new(connection)
        .load_by_id(finding_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load the selected QA finding.",
                Some(error.to_string()),
            )
        })?
        .ok_or_else(|| {
            DesktopCommandError::validation("The selected QA finding does not exist.", None)
        })?;

    if finding.document_id != document_id {
        return Err(DesktopCommandError::validation(
            "The selected QA finding does not belong to the active document.",
            None,
        ));
    }

    Ok(finding)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use serde_json::{json, Value};
    use rusqlite::params;
    use tempfile::{tempdir, TempDir};

    use super::{
        generate_review_job_id, inspect_qa_finding_with_runtime,
        retranslate_chunk_from_qa_finding_with_runtime_and_executor,
    };
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::finding_review::{InspectQaFindingInput, RetranslateChunkFromQaFindingInput};
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::qa_findings::QaFindingRepository;
    use crate::persistence::rule_sets::{RuleRepository, RuleSetRepository};
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::task_runs::TaskRunRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::{NewProject, ProjectEditorialDefaultsChanges};
    use crate::qa_findings::{
        NewQaFinding, QA_FINDING_SEVERITY_MEDIUM, QA_FINDING_STATUS_OPEN,
        QA_FINDING_STATUS_RESOLVED,
    };
    use crate::rule_sets::{
        NewRule, NewRuleSet, RULE_ACTION_SCOPE_RETRANSLATION, RULE_ACTION_SCOPE_TRANSLATION,
        RULE_SET_STATUS_ACTIVE, RULE_SEVERITY_MEDIUM, RULE_TYPE_CONSISTENCY,
    };
    use crate::segments::{
        NewSegment, SegmentTranslationWrite, SEGMENT_STATUS_PENDING_TRANSLATION,
    };
    use crate::task_runs::NewTaskRun;
    use crate::translate_chunk::{
        TranslateChunkActionRequest, TranslateChunkActionResponse, TranslateChunkExecutionFailure,
        TranslateChunkExecutor, TranslateChunkModelOutput, TranslateChunkTranslation,
    };
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const PROJECT_ID: &str = "prj_review_001";
    const DOCUMENT_ID: &str = "doc_review_001";
    const CHUNK_ID: &str = "doc_review_001_chunk_0001";
    const FINDING_ID: &str = "qaf_review_001";

    struct RuntimeFixture {
        _temporary_directory: TempDir,
        runtime: DatabaseRuntime,
    }

    struct FakeExecutor {
        observed_requests: Arc<Mutex<Vec<TranslateChunkActionRequest>>>,
        runtime: Option<DatabaseRuntime>,
        mutation: ExecuteMutation,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ExecuteMutation {
        None,
        OverwriteFindingState,
        DeleteFinding,
    }

    impl FakeExecutor {
        fn new(observed_requests: Arc<Mutex<Vec<TranslateChunkActionRequest>>>) -> Self {
            Self {
                observed_requests,
                runtime: None,
                mutation: ExecuteMutation::None,
            }
        }

        fn with_mutation(
            observed_requests: Arc<Mutex<Vec<TranslateChunkActionRequest>>>,
            runtime: DatabaseRuntime,
            mutation: ExecuteMutation,
        ) -> Self {
            Self {
                observed_requests,
                runtime: Some(runtime),
                mutation,
            }
        }
    }

    impl TranslateChunkExecutor for FakeExecutor {
        fn execute(
            &self,
            request: &TranslateChunkActionRequest,
        ) -> Result<TranslateChunkModelOutput, TranslateChunkExecutionFailure> {
            self.observed_requests
                .lock()
                .expect("requests lock should open")
                .push(request.clone());

            if let Some(runtime) = self.runtime.as_ref() {
                apply_executor_mutation(runtime, self.mutation);
            }

            Ok(TranslateChunkModelOutput {
                provider: "fake".to_owned(),
                model: "fake-model".to_owned(),
                raw_output: serde_json::to_string(&TranslateChunkActionResponse {
                    translations: vec![
                        TranslateChunkTranslation {
                            segment_id: "seg_review_0001".to_owned(),
                            target_text: "Mantened el portal cerrado.".to_owned(),
                        },
                        TranslateChunkTranslation {
                            segment_id: "seg_review_0002".to_owned(),
                            target_text: "Proteged la nave interior.".to_owned(),
                        },
                    ],
                    notes: Some("Finding-driven retry.".to_owned()),
                })
                .expect("fake response should serialize"),
            })
        }
    }

    #[test]
    fn generate_review_job_id_is_unique_within_the_same_second() {
        let first = generate_review_job_id(FINDING_ID, 1_900_100_123);
        let second = generate_review_job_id(FINDING_ID, 1_900_100_123);

        assert_ne!(first, second);
        assert!(first.starts_with("review_chunk_1900100123_"));
        assert!(second.starts_with("review_chunk_1900100123_"));
    }

    #[test]
    fn inspect_qa_finding_rejects_unknown_findings() {
        let fixture = create_runtime_fixture();
        seed_review_graph(&fixture.runtime);

        let error = inspect_qa_finding_with_runtime(
            InspectQaFindingInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                finding_id: "qaf_missing_001".to_owned(),
            },
            &fixture.runtime,
        )
        .expect_err("unknown finding ids should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("does not exist"));
    }

    #[test]
    fn inspect_qa_finding_resolves_chunk_through_segment_context() {
        let fixture = create_runtime_fixture();
        seed_review_graph(&fixture.runtime);

        let context = inspect_qa_finding_with_runtime(
            InspectQaFindingInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                finding_id: "qaf_review_segment_anchor".to_owned(),
            },
            &fixture.runtime,
        )
        .expect("finding inspection should succeed");

        assert_eq!(context.anchor.chunk_id.as_deref(), Some(CHUNK_ID));
        assert_eq!(context.anchor.resolution_kind, "segment_primary_chunk");
        assert!(context.anchor.can_retranslate);
        assert_eq!(context.related_segments.len(), 1);
        assert_eq!(context.related_segments[0].id, "seg_review_0002");
    }

    #[test]
    fn retranslate_chunk_from_qa_finding_runs_translate_chunk_and_persists_review_action() {
        let fixture = create_runtime_fixture();
        seed_review_graph(&fixture.runtime);
        let observed_requests = Arc::new(Mutex::new(Vec::new()));

        let result = retranslate_chunk_from_qa_finding_with_runtime_and_executor(
            RetranslateChunkFromQaFindingInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                finding_id: FINDING_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &FakeExecutor::new(observed_requests.clone()),
        )
        .expect("finding-driven retranslation should succeed");

        assert_eq!(result.anchor.chunk_id.as_deref(), Some(CHUNK_ID));
        assert_eq!(result.translate_result.chunk_id, CHUNK_ID);
        assert_eq!(result.finding.status, QA_FINDING_STATUS_OPEN);
        assert!(result.correction_job_id.starts_with("review_chunk_"));
        assert_eq!(
            observed_requests
                .lock()
                .expect("requests lock should open")
                .len(),
            1
        );
        let captured_requests = observed_requests.lock().expect("requests lock should open");
        assert_eq!(captured_requests[0].rules.len(), 1);
        assert_eq!(
            captured_requests[0].rules[0].name,
            "Keep ritual wording stable on correction"
        );
        assert_eq!(
            captured_requests[0].rules[0].guidance,
            "Preserve the established ritual register when correcting a chunk."
        );
        drop(captured_requests);

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let finding = QaFindingRepository::new(&mut connection)
            .load_by_id(FINDING_ID)
            .expect("finding should reload")
            .expect("finding should exist");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_chunk(CHUNK_ID)
            .expect("chunk task runs should reload");
        let details = serde_json::from_str::<Value>(
            finding
                .details
                .as_deref()
                .expect("review metadata should persist"),
        )
        .expect("finding details should remain JSON");

        assert_eq!(task_runs.len(), 2);
        assert!(task_runs
            .iter()
            .any(|task_run| task_run.job_id.as_deref() == Some(result.correction_job_id.as_str())));
        assert_eq!(
            details["reviewActions"]
                .as_array()
                .expect("review actions should be an array")
                .len(),
            1
        );
        assert_eq!(
            details["reviewActions"][0]["taskRunId"].as_str(),
            Some(result.translate_result.task_run.id.as_str())
        );
        assert!(result.review_action_persisted);
        assert_eq!(result.review_action_warning, None);
    }

    #[test]
    fn review_action_append_reloads_latest_finding_state_before_merging_details() {
        let fixture = create_runtime_fixture();
        seed_review_graph(&fixture.runtime);
        let observed_requests = Arc::new(Mutex::new(Vec::new()));

        let result = retranslate_chunk_from_qa_finding_with_runtime_and_executor(
            RetranslateChunkFromQaFindingInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                finding_id: FINDING_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &FakeExecutor::with_mutation(
                observed_requests,
                fixture.runtime.clone(),
                ExecuteMutation::OverwriteFindingState,
            ),
        )
        .expect("finding-driven retranslation should succeed");

        let details = serde_json::from_str::<Value>(
            result
                .finding
                .details
                .as_deref()
                .expect("updated finding details should persist"),
        )
        .expect("finding details should remain JSON");

        assert_eq!(result.finding.status, QA_FINDING_STATUS_RESOLVED);
        assert_eq!(result.finding.chunk_id, None);
        assert_eq!(details["manualStatus"].as_str(), Some("reviewed"));
        assert_eq!(
            details["reviewActions"][0]["taskRunId"].as_str(),
            Some(result.translate_result.task_run.id.as_str())
        );
        assert!(result.review_action_persisted);
        assert_eq!(result.review_action_warning, None);
    }

    #[test]
    fn retranslation_returns_success_when_review_action_metadata_cannot_persist() {
        let fixture = create_runtime_fixture();
        seed_review_graph(&fixture.runtime);
        let observed_requests = Arc::new(Mutex::new(Vec::new()));

        let result = retranslate_chunk_from_qa_finding_with_runtime_and_executor(
            RetranslateChunkFromQaFindingInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                finding_id: FINDING_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &FakeExecutor::with_mutation(
                observed_requests,
                fixture.runtime.clone(),
                ExecuteMutation::DeleteFinding,
            ),
        )
        .expect("chunk retranslation should still succeed");

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_chunk(CHUNK_ID)
            .expect("chunk task runs should reload");

        assert_eq!(result.finding.id, FINDING_ID);
        assert!(!result.review_action_persisted);
        assert!(result
            .review_action_warning
            .as_deref()
            .is_some_and(|warning| warning.contains("no longer exists")));
        assert_eq!(task_runs.len(), 2);
    }

    fn apply_executor_mutation(runtime: &DatabaseRuntime, mutation: ExecuteMutation) {
        match mutation {
            ExecuteMutation::None => {}
            ExecuteMutation::OverwriteFindingState => {
                let mut connection = runtime
                    .open_connection()
                    .expect("database connection should open");
                let mut repository = QaFindingRepository::new(&mut connection);
                let existing = repository
                    .load_by_id(FINDING_ID)
                    .expect("finding should reload")
                    .expect("finding should exist");

                repository
                    .upsert(&NewQaFinding {
                        id: existing.id,
                        document_id: existing.document_id,
                        chunk_id: None,
                        task_run_id: existing.task_run_id,
                        job_id: existing.job_id,
                        finding_type: existing.finding_type,
                        severity: existing.severity,
                        status: QA_FINDING_STATUS_RESOLVED.to_owned(),
                        message: existing.message,
                        details: Some("{\"manualStatus\":\"reviewed\"}".to_owned()),
                        created_at: existing.created_at,
                        updated_at: existing.updated_at + 50,
                    })
                    .expect("manual finding update should persist");
            }
            ExecuteMutation::DeleteFinding => {
                let connection = runtime
                    .open_connection()
                    .expect("database connection should open");
                connection
                    .execute("DELETE FROM qa_findings WHERE id = ?1", params![FINDING_ID])
                    .expect("finding delete should persist");
            }
        }
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

    fn seed_review_graph(runtime: &DatabaseRuntime) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_900_100_000_i64;

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: PROJECT_ID.to_owned(),
                name: "Review project".to_owned(),
                description: None,
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("project should persist");
        ProjectRepository::new(&mut connection)
            .open_project(PROJECT_ID, now + 1)
            .expect("project should become active");
        RuleSetRepository::new(&mut connection)
            .create(&NewRuleSet {
                id: "rset_review_001".to_owned(),
                name: "Review rules".to_owned(),
                description: None,
                status: RULE_SET_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("rule set should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_review_translation_001".to_owned(),
                rule_set_id: "rset_review_001".to_owned(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
                rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                severity: RULE_SEVERITY_MEDIUM.to_owned(),
                name: "Keep baseline translation wording".to_owned(),
                description: Some("Should not be used by corrective retranslation.".to_owned()),
                guidance: "Use the default translation wording for first-pass chunk output."
                    .to_owned(),
                is_enabled: true,
                created_at: now,
                updated_at: now,
            })
            .expect("translation rule should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_review_retranslation_001".to_owned(),
                rule_set_id: "rset_review_001".to_owned(),
                action_scope: RULE_ACTION_SCOPE_RETRANSLATION.to_owned(),
                rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                severity: RULE_SEVERITY_MEDIUM.to_owned(),
                name: "Keep ritual wording stable on correction".to_owned(),
                description: Some(
                    "This rule should be attached only to finding-driven corrective runs."
                        .to_owned(),
                ),
                guidance:
                    "Preserve the established ritual register when correcting a chunk."
                        .to_owned(),
                is_enabled: true,
                created_at: now + 1,
                updated_at: now + 1,
            })
            .expect("retranslation rule should persist");
        ProjectRepository::new(&mut connection)
            .update_editorial_defaults(&ProjectEditorialDefaultsChanges {
                project_id: PROJECT_ID.to_owned(),
                default_glossary_id: None,
                default_style_profile_id: None,
                default_rule_set_id: Some("rset_review_001".to_owned()),
                updated_at: now + 2,
            })
            .expect("project editorial defaults should persist");

        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: DOCUMENT_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
                name: "review-doc.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "review-doc.txt".to_owned(),
                file_size_bytes: 96,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: now,
                updated_at: now,
            })
            .expect("document should persist");

        SegmentRepository::new(&mut connection)
            .replace_for_document(
                PROJECT_ID,
                DOCUMENT_ID,
                &[
                    NewSegment {
                        id: "seg_review_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        source_text: "Keep the gate sealed.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 21,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "seg_review_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        source_text: "Protect the inner nave.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 23,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "seg_review_0003".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 3,
                        source_text: "The beacon remains lit.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 24,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                ],
                now,
            )
            .expect("segments should persist");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[NewTranslationChunk {
                    id: CHUNK_ID.to_owned(),
                    document_id: DOCUMENT_ID.to_owned(),
                    sequence: 1,
                    builder_version: "tr12-basic-v1".to_owned(),
                    strategy: "fixed".to_owned(),
                    source_text: "Keep the gate sealed.\n\nProtect the inner nave.".to_owned(),
                    context_before_text: None,
                    context_after_text: Some("The beacon remains lit.".to_owned()),
                    start_segment_sequence: 1,
                    end_segment_sequence: 2,
                    segment_count: 2,
                    source_word_count: 8,
                    source_character_count: 44,
                    created_at: now,
                    updated_at: now,
                }],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID.to_owned(),
                        segment_id: "seg_review_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID.to_owned(),
                        segment_id: "seg_review_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                ],
            )
            .expect("chunks should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_review_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some(CHUNK_ID.to_owned()),
                job_id: Some("job_review_seed_001".to_owned()),
                action_type: "translate_chunk".to_owned(),
                status: "running".to_owned(),
                input_payload: Some("{\"seed\":true}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: now + 10,
                completed_at: None,
                created_at: now + 10,
                updated_at: now + 10,
            })
            .expect("task run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_review_0001",
                "{\"seed\":true}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_review_0001".to_owned(),
                        target_text: "Mantened el portón sellado.".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_review_0002".to_owned(),
                        target_text: "Proteged la nave interior.".to_owned(),
                    },
                ],
                now + 20,
            )
            .expect("task run should project translations");

        QaFindingRepository::new(&mut connection)
            .upsert(&NewQaFinding {
                id: FINDING_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some(CHUNK_ID.to_owned()),
                task_run_id: Some("task_review_0001".to_owned()),
                job_id: Some("job_review_seed_001".to_owned()),
                finding_type: "neighbor_chunk_translation_drift".to_owned(),
                severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                status: QA_FINDING_STATUS_OPEN.to_owned(),
                message: "Drift detected across neighboring chunks.".to_owned(),
                details: Some(
                    json!({
                        "segmentIds": ["seg_review_0001", "seg_review_0002"]
                    })
                    .to_string(),
                ),
                created_at: now + 30,
                updated_at: now + 30,
            })
            .expect("finding should persist");
        QaFindingRepository::new(&mut connection)
            .upsert(&NewQaFinding {
                id: "qaf_review_segment_anchor".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                task_run_id: None,
                job_id: None,
                finding_type: "source_fallback_segment".to_owned(),
                severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                status: QA_FINDING_STATUS_OPEN.to_owned(),
                message: "Fallback segment detected.".to_owned(),
                details: Some(
                    json!({
                        "segmentId": "seg_review_0002"
                    })
                    .to_string(),
                ),
                created_at: now + 31,
                updated_at: now + 31,
            })
            .expect("segment-anchored finding should persist");
    }
}
