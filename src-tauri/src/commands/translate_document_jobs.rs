use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use tauri::State;

use crate::commands::segments::load_segmented_document_overview;
use crate::commands::translate_chunk::translate_chunk_with_runtime_and_executor;
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::task_runs::TaskRunRepository;
use crate::persistence::translation_chunks::TranslationChunkRepository;
use crate::task_runs::{
    NewTaskRun, TaskRunSummary, TASK_RUN_STATUS_CANCELLED, TASK_RUN_STATUS_COMPLETED,
    TASK_RUN_STATUS_FAILED, TASK_RUN_STATUS_PENDING, TASK_RUN_STATUS_RUNNING,
};
use crate::translate_chunk::{
    OpenAiTranslateChunkExecutor, TranslateChunkExecutor, TranslateChunkInput,
    TRANSLATE_CHUNK_ACTION_TYPE,
};
use crate::translate_document::{
    TranslateDocumentChunkResult, TranslateDocumentInput, TranslateDocumentJobInput,
    TranslateDocumentJobStatus, TranslateDocumentResult, TRANSLATE_DOCUMENT_ACTION_TYPE,
    TRANSLATE_DOCUMENT_ACTION_VERSION, TRANSLATE_DOCUMENT_CHUNK_STATUS_CANCELLED,
    TRANSLATE_DOCUMENT_CHUNK_STATUS_COMPLETED, TRANSLATE_DOCUMENT_CHUNK_STATUS_FAILED,
    TRANSLATE_DOCUMENT_CHUNK_STATUS_PENDING, TRANSLATE_DOCUMENT_CHUNK_STATUS_RUNNING,
    TRANSLATE_DOCUMENT_STATUS_CANCELLED, TRANSLATE_DOCUMENT_STATUS_COMPLETED,
    TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS, TRANSLATE_DOCUMENT_STATUS_FAILED,
    TRANSLATE_DOCUMENT_STATUS_PENDING, TRANSLATE_DOCUMENT_STATUS_RUNNING,
};
use crate::translation_chunks::TranslationChunkSummary;

const CANCELLATION_MESSAGE: &str = "Cancellation requested by the user.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TranslateDocumentExecutionMode {
    Fresh,
    Resume,
}

#[tauri::command]
pub fn get_translate_document_job_status(
    input: TranslateDocumentJobInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<TranslateDocumentJobStatus, DesktopCommandError> {
    get_translate_document_job_status_with_runtime(input, database_runtime.inner())
}

#[tauri::command]
pub fn cancel_translate_document_job(
    input: TranslateDocumentJobInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<TranslateDocumentJobStatus, DesktopCommandError> {
    cancel_translate_document_job_with_runtime(input, database_runtime.inner())
}

#[tauri::command]
pub fn resume_translate_document_job(
    input: TranslateDocumentJobInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<TranslateDocumentResult, DesktopCommandError> {
    let executor = OpenAiTranslateChunkExecutor::from_environment()?;

    resume_translate_document_job_with_runtime_and_executor(
        input,
        database_runtime.inner(),
        &executor,
    )
}

pub(crate) fn get_translate_document_job_status_with_runtime(
    input: TranslateDocumentJobInput,
    database_runtime: &DatabaseRuntime,
) -> Result<TranslateDocumentJobStatus, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let job_id = validate_identifier(&input.job_id, "job id")?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for translate_document job status.",
            Some(error.to_string()),
        )
    })?;

    build_job_status(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        &job_id,
    )
}

pub(crate) fn cancel_translate_document_job_with_runtime(
    input: TranslateDocumentJobInput,
    database_runtime: &DatabaseRuntime,
) -> Result<TranslateDocumentJobStatus, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let job_id = validate_identifier(&input.job_id, "job id")?;
    let cancelled_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for translate_document cancellation.",
            Some(error.to_string()),
        )
    })?;
    let _ = load_document_chunks(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        cancelled_at,
    )?;
    let task_runs = list_job_task_runs_for_document(&mut connection, &document_id, &job_id)?;

    if task_runs.is_empty() {
        return Err(DesktopCommandError::validation(
            "The selected translate_document job does not exist for the active document.",
            None,
        ));
    }

    let running_document_task_runs = task_runs
        .iter()
        .filter(|task_run| {
            task_run.action_type == TRANSLATE_DOCUMENT_ACTION_TYPE
                && matches!(
                    task_run.status.as_str(),
                    TASK_RUN_STATUS_PENDING | TASK_RUN_STATUS_RUNNING
                )
        })
        .map(|task_run| task_run.id.clone())
        .collect::<Vec<_>>();

    for task_run_id in running_document_task_runs {
        TaskRunRepository::new(&mut connection)
            .mark_cancelled(&task_run_id, CANCELLATION_MESSAGE, None, cancelled_at)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not record translate_document cancellation.",
                    Some(error.to_string()),
                )
            })?;
    }

    build_job_status(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        &job_id,
    )
}

pub(crate) fn resume_translate_document_job_with_runtime_and_executor<E: TranslateChunkExecutor>(
    input: TranslateDocumentJobInput,
    database_runtime: &DatabaseRuntime,
    executor: &E,
) -> Result<TranslateDocumentResult, DesktopCommandError> {
    run_translate_document_with_runtime_and_executor(
        TranslateDocumentInput {
            project_id: input.project_id,
            document_id: input.document_id,
            job_id: Some(input.job_id),
        },
        TranslateDocumentExecutionMode::Resume,
        database_runtime,
        executor,
    )
}

pub(crate) fn build_job_status(
    connection: &mut rusqlite::Connection,
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    job_id: &str,
) -> Result<TranslateDocumentJobStatus, DesktopCommandError> {
    let task_runs = list_job_task_runs_for_document(connection, document_id, job_id)?;

    if task_runs.is_empty() {
        return Err(DesktopCommandError::validation(
            "The selected translate_document job does not exist for the active document.",
            None,
        ));
    }

    build_job_status_from_task_runs(
        connection,
        database_runtime,
        project_id,
        document_id,
        job_id,
        task_runs,
    )
}

pub(crate) fn build_job_status_if_exists(
    connection: &mut rusqlite::Connection,
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    job_id: &str,
) -> Result<Option<TranslateDocumentJobStatus>, DesktopCommandError> {
    let task_runs = list_job_task_runs_for_document(connection, document_id, job_id)?;

    if task_runs.is_empty() {
        return Ok(None);
    }

    build_job_status_from_task_runs(
        connection,
        database_runtime,
        project_id,
        document_id,
        job_id,
        task_runs,
    )
    .map(Some)
}

fn build_job_status_from_task_runs(
    connection: &mut rusqlite::Connection,
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    job_id: &str,
    task_runs: Vec<TaskRunSummary>,
) -> Result<TranslateDocumentJobStatus, DesktopCommandError> {
    let chunks = load_document_chunks(connection, database_runtime, project_id, document_id, 0)?;
    let latest_document_task_run = select_latest_document_task_run(&task_runs);
    let latest_chunk_task_runs = select_latest_chunk_task_runs(&task_runs);
    let latest_document_status = latest_document_task_run
        .as_ref()
        .map(|task_run| task_run.status.as_str());
    let last_updated_at = task_runs.iter().map(|task_run| task_run.updated_at).max();
    let mut pending_chunks = 0_i64;
    let mut running_chunks = 0_i64;
    let mut completed_chunks = 0_i64;
    let mut failed_chunks = 0_i64;
    let mut cancelled_chunks = 0_i64;
    let mut current_chunk_id = None;
    let mut current_chunk_sequence = None;
    let mut last_completed_chunk_id = None;
    let mut last_completed_chunk_sequence = None;
    let mut chunk_statuses = Vec::with_capacity(chunks.len());
    let mut error_messages = Vec::new();

    for chunk in &chunks {
        let chunk_status = match latest_chunk_task_runs.get(chunk.id.as_str()) {
            Some(task_run) => map_chunk_status(task_run),
            None if latest_document_status == Some(TASK_RUN_STATUS_CANCELLED) => {
                TRANSLATE_DOCUMENT_CHUNK_STATUS_CANCELLED.to_owned()
            }
            _ => TRANSLATE_DOCUMENT_CHUNK_STATUS_PENDING.to_owned(),
        };
        let task_run = latest_chunk_task_runs.get(chunk.id.as_str()).cloned();
        let translated_segment_count = task_run
            .as_ref()
            .map(extract_translated_segment_count)
            .unwrap_or(0);
        let error_message = task_run.as_ref().and_then(|run| run.error_message.clone());

        match chunk_status.as_str() {
            TRANSLATE_DOCUMENT_CHUNK_STATUS_PENDING => pending_chunks += 1,
            TRANSLATE_DOCUMENT_CHUNK_STATUS_RUNNING => {
                running_chunks += 1;

                if current_chunk_sequence.is_none() {
                    current_chunk_id = Some(chunk.id.clone());
                    current_chunk_sequence = Some(chunk.sequence);
                }
            }
            TRANSLATE_DOCUMENT_CHUNK_STATUS_COMPLETED => {
                completed_chunks += 1;

                if last_completed_chunk_sequence.is_none_or(|sequence| chunk.sequence > sequence) {
                    last_completed_chunk_id = Some(chunk.id.clone());
                    last_completed_chunk_sequence = Some(chunk.sequence);
                }
            }
            TRANSLATE_DOCUMENT_CHUNK_STATUS_FAILED => failed_chunks += 1,
            TRANSLATE_DOCUMENT_CHUNK_STATUS_CANCELLED => cancelled_chunks += 1,
            _ => pending_chunks += 1,
        }

        if let Some(error_message) = error_message.clone() {
            error_messages.push(error_message);
        }

        chunk_statuses.push(TranslateDocumentChunkResult {
            chunk_id: chunk.id.clone(),
            chunk_sequence: chunk.sequence,
            status: chunk_status,
            task_run,
            translated_segment_count,
            error_message,
        });
    }

    if let Some(error_message) = latest_document_task_run
        .as_ref()
        .and_then(|task_run| task_run.error_message.clone())
    {
        error_messages.push(error_message);
    }

    let mut deduplicated_error_messages = Vec::new();
    let mut seen_error_messages = HashSet::new();

    for error_message in error_messages {
        if seen_error_messages.insert(error_message.clone()) {
            deduplicated_error_messages.push(error_message);
        }
    }

    let total_chunks = i64::try_from(chunks.len()).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid translation chunk count while aggregating translate_document status.",
            Some(error.to_string()),
        )
    })?;
    let status = derive_job_status(
        latest_document_status,
        pending_chunks,
        running_chunks,
        completed_chunks,
        failed_chunks,
        cancelled_chunks,
        total_chunks,
    );

    Ok(TranslateDocumentJobStatus {
        project_id: project_id.to_owned(),
        document_id: document_id.to_owned(),
        job_id: job_id.to_owned(),
        status,
        total_chunks,
        pending_chunks,
        running_chunks,
        completed_chunks,
        failed_chunks,
        cancelled_chunks,
        current_chunk_id,
        current_chunk_sequence,
        last_completed_chunk_id,
        last_completed_chunk_sequence,
        last_updated_at,
        latest_document_task_run,
        chunk_statuses,
        task_runs,
        error_messages: deduplicated_error_messages,
    })
}

fn load_document_chunks(
    connection: &mut rusqlite::Connection,
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    timestamp: i64,
) -> Result<Vec<TranslationChunkSummary>, DesktopCommandError> {
    let _ = load_segmented_document_overview(
        connection,
        database_runtime,
        project_id,
        document_id,
        false,
        timestamp,
    )?;

    TranslationChunkRepository::new(connection)
        .list_chunks_by_document(document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load translation chunks for the selected document.",
                Some(error.to_string()),
            )
        })
}

fn list_job_task_runs_for_document(
    connection: &mut rusqlite::Connection,
    document_id: &str,
    job_id: &str,
) -> Result<Vec<TaskRunSummary>, DesktopCommandError> {
    TaskRunRepository::new(connection)
        .list_by_job_id(job_id)
        .map(|task_runs| {
            task_runs
                .into_iter()
                .filter(|task_run| task_run.document_id == document_id)
                .collect()
        })
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load task runs for the selected translate_document job.",
                Some(error.to_string()),
            )
        })
}

fn select_latest_document_task_run(task_runs: &[TaskRunSummary]) -> Option<TaskRunSummary> {
    task_runs
        .iter()
        .filter(|task_run| task_run.action_type == TRANSLATE_DOCUMENT_ACTION_TYPE)
        .cloned()
        .max_by(compare_task_runs)
}

fn select_latest_chunk_task_runs(task_runs: &[TaskRunSummary]) -> HashMap<&str, TaskRunSummary> {
    let mut latest_chunk_task_runs = HashMap::new();

    for task_run in task_runs
        .iter()
        .filter(|task_run| task_run.action_type == TRANSLATE_CHUNK_ACTION_TYPE)
    {
        let Some(chunk_id) = task_run.chunk_id.as_deref() else {
            continue;
        };

        match latest_chunk_task_runs.get(chunk_id) {
            Some(existing_task_run) if compare_task_runs(existing_task_run, task_run).is_ge() => {}
            _ => {
                latest_chunk_task_runs.insert(chunk_id, task_run.clone());
            }
        }
    }

    latest_chunk_task_runs
}

fn compare_task_runs(left: &TaskRunSummary, right: &TaskRunSummary) -> std::cmp::Ordering {
    left.created_at
        .cmp(&right.created_at)
        .then_with(|| left.updated_at.cmp(&right.updated_at))
        .then_with(|| left.id.cmp(&right.id))
}

fn map_chunk_status(task_run: &TaskRunSummary) -> String {
    match task_run.status.as_str() {
        TASK_RUN_STATUS_PENDING => TRANSLATE_DOCUMENT_CHUNK_STATUS_PENDING.to_owned(),
        TASK_RUN_STATUS_RUNNING => TRANSLATE_DOCUMENT_CHUNK_STATUS_RUNNING.to_owned(),
        TASK_RUN_STATUS_COMPLETED => TRANSLATE_DOCUMENT_CHUNK_STATUS_COMPLETED.to_owned(),
        TASK_RUN_STATUS_FAILED => TRANSLATE_DOCUMENT_CHUNK_STATUS_FAILED.to_owned(),
        TASK_RUN_STATUS_CANCELLED => TRANSLATE_DOCUMENT_CHUNK_STATUS_CANCELLED.to_owned(),
        _ => TRANSLATE_DOCUMENT_CHUNK_STATUS_PENDING.to_owned(),
    }
}

fn derive_job_status(
    latest_document_status: Option<&str>,
    pending_chunks: i64,
    running_chunks: i64,
    completed_chunks: i64,
    failed_chunks: i64,
    cancelled_chunks: i64,
    total_chunks: i64,
) -> String {
    if latest_document_status == Some(TASK_RUN_STATUS_CANCELLED) || cancelled_chunks == total_chunks
    {
        return TRANSLATE_DOCUMENT_STATUS_CANCELLED.to_owned();
    }

    if latest_document_status == Some(TASK_RUN_STATUS_RUNNING) || running_chunks > 0 {
        return TRANSLATE_DOCUMENT_STATUS_RUNNING.to_owned();
    }

    if latest_document_status == Some(TASK_RUN_STATUS_PENDING) || pending_chunks == total_chunks {
        return TRANSLATE_DOCUMENT_STATUS_PENDING.to_owned();
    }

    if completed_chunks == total_chunks {
        return TRANSLATE_DOCUMENT_STATUS_COMPLETED.to_owned();
    }

    if failed_chunks > 0 && completed_chunks > 0 {
        return TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS.to_owned();
    }

    if failed_chunks > 0 {
        return TRANSLATE_DOCUMENT_STATUS_FAILED.to_owned();
    }

    if cancelled_chunks > 0 {
        return TRANSLATE_DOCUMENT_STATUS_CANCELLED.to_owned();
    }

    if pending_chunks > 0 {
        return TRANSLATE_DOCUMENT_STATUS_PENDING.to_owned();
    }

    TRANSLATE_DOCUMENT_STATUS_PENDING.to_owned()
}

fn extract_translated_segment_count(task_run: &TaskRunSummary) -> i64 {
    task_run
        .output_payload
        .as_deref()
        .and_then(|payload| serde_json::from_str::<Value>(payload).ok())
        .and_then(|payload| {
            payload
                .get("translations")
                .and_then(Value::as_array)
                .cloned()
        })
        .and_then(|translations| i64::try_from(translations.len()).ok())
        .unwrap_or(0)
}

fn select_resumable_chunks(
    chunks: &[TranslationChunkSummary],
    status: Option<&TranslateDocumentJobStatus>,
) -> Result<Vec<TranslationChunkSummary>, DesktopCommandError> {
    let Some(status) = status else {
        return Ok(Vec::new());
    };
    let completed_chunk_ids = status
        .chunk_statuses
        .iter()
        .filter(|chunk| chunk.status == TRANSLATE_DOCUMENT_CHUNK_STATUS_COMPLETED)
        .map(|chunk| chunk.chunk_id.as_str())
        .collect::<HashSet<_>>();

    Ok(chunks
        .iter()
        .filter(|chunk| !completed_chunk_ids.contains(chunk.id.as_str()))
        .cloned()
        .collect())
}

fn serialize_document_attempt_input_payload(
    project_id: &str,
    document_id: &str,
    job_id: &str,
    selected_chunks: &[TranslationChunkSummary],
    mode: TranslateDocumentExecutionMode,
) -> Result<String, DesktopCommandError> {
    serde_json::to_string(&json!({
        "projectId": project_id,
        "documentId": document_id,
        "jobId": job_id,
        "actionVersion": TRANSLATE_DOCUMENT_ACTION_VERSION,
        "mode": match mode {
            TranslateDocumentExecutionMode::Fresh => "fresh",
            TranslateDocumentExecutionMode::Resume => "resume",
        },
        "chunkIds": selected_chunks.iter().map(|chunk| chunk.id.as_str()).collect::<Vec<_>>()
    }))
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not serialize the translate_document input payload.",
            Some(error.to_string()),
        )
    })
}

pub(crate) fn run_translate_document_with_runtime_and_executor<E: TranslateChunkExecutor>(
    input: TranslateDocumentInput,
    mode: TranslateDocumentExecutionMode,
    database_runtime: &DatabaseRuntime,
    executor: &E,
) -> Result<TranslateDocumentResult, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let started_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for translate_document.",
            Some(error.to_string()),
        )
    })?;
    let chunks = load_document_chunks(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        started_at,
    )?;

    if chunks.is_empty() {
        return Err(DesktopCommandError::validation(
            "The selected document must have persisted translation chunks before translate_document can start.",
            None,
        ));
    }

    let job_id = match mode {
        TranslateDocumentExecutionMode::Fresh => {
            normalize_optional_identifier(input.job_id, "job id")?
                .unwrap_or_else(|| generate_job_id(started_at))
        }
        TranslateDocumentExecutionMode::Resume => {
            let provided_job_id = input.job_id.ok_or_else(|| {
                DesktopCommandError::validation(
                    "The resume_translate_document_job action requires a valid job id.",
                    None,
                )
            })?;
            validate_identifier(&provided_job_id, "job id")?
        }
    };

    let existing_status = build_job_status_if_exists(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        &job_id,
    )?;

    if existing_status
        .as_ref()
        .is_some_and(|status| status.status == TRANSLATE_DOCUMENT_STATUS_RUNNING)
    {
        return Err(DesktopCommandError::validation(
            "The selected translate_document job is already running.",
            None,
        ));
    }

    if mode == TranslateDocumentExecutionMode::Resume && existing_status.is_none() {
        return Err(DesktopCommandError::validation(
            "The selected translate_document job does not exist for the active document.",
            None,
        ));
    }

    let selected_chunks = match mode {
        TranslateDocumentExecutionMode::Fresh => chunks.clone(),
        TranslateDocumentExecutionMode::Resume => {
            select_resumable_chunks(&chunks, existing_status.as_ref())?
        }
    };

    if selected_chunks.is_empty() {
        let existing_status = existing_status.ok_or_else(|| {
            DesktopCommandError::validation(
                "The selected translate_document job does not have resumable chunks.",
                None,
            )
        })?;

        return Ok(job_status_to_result(existing_status));
    }

    let task_run_id = generate_task_run_id(started_at);
    let selected_chunk_ids = selected_chunks
        .iter()
        .map(|chunk| chunk.id.clone())
        .collect::<Vec<_>>();
    let input_payload = serialize_document_attempt_input_payload(
        &project_id,
        &document_id,
        &job_id,
        &selected_chunks,
        mode,
    )?;
    let _document_task_run = TaskRunRepository::new(&mut connection)
        .create(&NewTaskRun {
            id: task_run_id.clone(),
            document_id: document_id.clone(),
            chunk_id: None,
            job_id: Some(job_id.clone()),
            action_type: TRANSLATE_DOCUMENT_ACTION_TYPE.to_owned(),
            status: TASK_RUN_STATUS_RUNNING.to_owned(),
            input_payload: Some(input_payload),
            output_payload: None,
            error_message: None,
            started_at,
            completed_at: None,
            created_at: started_at,
            updated_at: started_at,
        })
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not open a task run for translate_document.",
                Some(error.to_string()),
            )
        })?;

    let mut completed_chunks_in_attempt = 0_i64;
    let mut failed_chunks_in_attempt = 0_i64;

    for chunk in selected_chunks {
        if is_document_task_run_cancelled(&mut connection, &task_run_id)? {
            finalize_document_attempt(
                &mut connection,
                &task_run_id,
                &job_id,
                mode,
                &selected_chunk_ids,
                completed_chunks_in_attempt,
                failed_chunks_in_attempt,
                Some(CANCELLATION_MESSAGE),
            )?;
            return build_job_status(
                &mut connection,
                database_runtime,
                &project_id,
                &document_id,
                &job_id,
            )
            .map(job_status_to_result);
        }

        if translate_chunk_with_runtime_and_executor(
            TranslateChunkInput {
                project_id: project_id.clone(),
                document_id: document_id.clone(),
                chunk_id: chunk.id.clone(),
                job_id: Some(job_id.clone()),
            },
            database_runtime,
            executor,
        )
        .is_ok()
        {
            completed_chunks_in_attempt += 1;
        } else {
            failed_chunks_in_attempt += 1;
        }
    }

    let cancellation_message = if is_document_task_run_cancelled(&mut connection, &task_run_id)? {
        Some(CANCELLATION_MESSAGE)
    } else {
        None
    };
    finalize_document_attempt(
        &mut connection,
        &task_run_id,
        &job_id,
        mode,
        &selected_chunk_ids,
        completed_chunks_in_attempt,
        failed_chunks_in_attempt,
        cancellation_message,
    )?;

    build_job_status(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        &job_id,
    )
    .map(job_status_to_result)
}

fn finalize_document_attempt(
    connection: &mut rusqlite::Connection,
    task_run_id: &str,
    job_id: &str,
    mode: TranslateDocumentExecutionMode,
    selected_chunk_ids: &[String],
    completed_chunks: i64,
    failed_chunks: i64,
    cancellation_message: Option<&str>,
) -> Result<TaskRunSummary, DesktopCommandError> {
    let completed_at = current_timestamp()?;
    let attempt_status = if cancellation_message.is_some() {
        TRANSLATE_DOCUMENT_STATUS_CANCELLED
    } else if failed_chunks == 0 {
        TRANSLATE_DOCUMENT_STATUS_COMPLETED
    } else {
        TRANSLATE_DOCUMENT_STATUS_FAILED
    };
    let output_payload = serde_json::to_string(&json!({
        "actionVersion": TRANSLATE_DOCUMENT_ACTION_VERSION,
        "jobId": job_id,
        "mode": match mode {
            TranslateDocumentExecutionMode::Fresh => "fresh",
            TranslateDocumentExecutionMode::Resume => "resume",
        },
        "status": attempt_status,
        "selectedChunkIds": selected_chunk_ids,
        "completedChunks": completed_chunks,
        "failedChunks": failed_chunks
    }))
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not serialize the translate_document output payload.",
            Some(error.to_string()),
        )
    })?;

    if let Some(cancellation_message) = cancellation_message {
        TaskRunRepository::new(connection)
            .mark_cancelled(task_run_id, cancellation_message, Some(&output_payload), completed_at)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not finalize the cancelled translate_document task run.",
                    Some(error.to_string()),
                )
            })
    } else if failed_chunks == 0 {
        TaskRunRepository::new(connection)
            .mark_completed(task_run_id, &output_payload, completed_at)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not finalize the translate_document task run.",
                    Some(error.to_string()),
                )
            })
    } else {
        TaskRunRepository::new(connection)
            .mark_failed(
                task_run_id,
                &format!(
                    "translate_document attempt finished with {failed_chunks} failed chunk(s) and {completed_chunks} completed chunk(s)."
                ),
                Some(&output_payload),
                completed_at,
            )
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not finalize the failed translate_document task run.",
                    Some(error.to_string()),
                )
            })
    }
}

fn is_document_task_run_cancelled(
    connection: &mut rusqlite::Connection,
    task_run_id: &str,
) -> Result<bool, DesktopCommandError> {
    TaskRunRepository::new(connection)
        .load_by_id(task_run_id)
        .map(|task_run| {
            task_run.is_some_and(|task_run| task_run.status == TASK_RUN_STATUS_CANCELLED)
        })
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not inspect translate_document cancellation state.",
                Some(error.to_string()),
            )
        })
}

fn job_status_to_result(status: TranslateDocumentJobStatus) -> TranslateDocumentResult {
    let TranslateDocumentJobStatus {
        project_id,
        document_id,
        job_id,
        status,
        total_chunks,
        completed_chunks,
        failed_chunks,
        latest_document_task_run,
        chunk_statuses,
        error_messages,
        ..
    } = status;

    TranslateDocumentResult {
        project_id,
        document_id,
        job_id,
        status,
        action_version: TRANSLATE_DOCUMENT_ACTION_VERSION.to_owned(),
        task_run: latest_document_task_run
            .expect("job status should include a latest document task run"),
        total_chunks,
        completed_chunks,
        failed_chunks,
        chunk_results: chunk_statuses,
        error_messages,
    }
}

pub(crate) fn validate_identifier(value: &str, label: &str) -> Result<String, DesktopCommandError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(DesktopCommandError::validation(
            format!("The translate_document action requires a valid {label}."),
            None,
        ));
    }

    if !trimmed
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(
            format!("The translate_document action requires a safe persisted {label}."),
            None,
        ));
    }

    Ok(trimmed.to_owned())
}

pub(crate) fn normalize_optional_identifier(
    value: Option<String>,
    label: &str,
) -> Result<Option<String>, DesktopCommandError> {
    value
        .map(|value| validate_identifier(&value, label))
        .transpose()
}

pub(crate) fn generate_job_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "job_{}_{}",
        timestamp,
        URL_SAFE_NO_PAD.encode(random_part.to_le_bytes())
    )
}

pub(crate) fn generate_task_run_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "trun_{}_{}",
        timestamp,
        URL_SAFE_NO_PAD.encode(random_part.to_le_bytes())
    )
}

pub(crate) fn current_timestamp() -> Result<i64, DesktopCommandError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not compute the current translate_document timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid translate_document timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use tempfile::{tempdir, TempDir};

    use super::{
        cancel_translate_document_job_with_runtime, get_translate_document_job_status_with_runtime,
        resume_translate_document_job_with_runtime_and_executor,
        run_translate_document_with_runtime_and_executor, TranslateDocumentExecutionMode,
    };
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
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
    use crate::segments::{NewSegment, SEGMENT_STATUS_PENDING_TRANSLATION};
    use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_CANCELLED, TASK_RUN_STATUS_RUNNING};
    use crate::translate_chunk::{
        TranslateChunkActionRequest, TranslateChunkActionResponse, TranslateChunkExecutionFailure,
        TranslateChunkExecutor, TranslateChunkModelOutput, TranslateChunkTranslation,
    };
    use crate::translate_document::{
        TranslateDocumentInput, TranslateDocumentJobInput, TRANSLATE_DOCUMENT_ACTION_TYPE,
        TRANSLATE_DOCUMENT_STATUS_CANCELLED, TRANSLATE_DOCUMENT_STATUS_COMPLETED,
        TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS,
    };
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const PROJECT_ID: &str = "prj_active_001";
    const DOCUMENT_ID: &str = "doc_translate_001";
    const CHUNK_ID_1: &str = "doc_translate_001_chunk_0001";
    const CHUNK_ID_2: &str = "doc_translate_001_chunk_0002";

    struct RuntimeFixture {
        _temporary_directory: TempDir,
        runtime: DatabaseRuntime,
    }

    #[derive(Clone)]
    enum FakeExecutorResponse {
        Success(TranslateChunkActionResponse),
        Failure(TranslateChunkExecutionFailure),
    }

    type AfterExecuteHook = Arc<dyn Fn(&TranslateChunkActionRequest) + Send + Sync>;

    struct FakeExecutor {
        responses: HashMap<String, FakeExecutorResponse>,
        observed_chunk_ids: Arc<Mutex<Vec<String>>>,
        after_execute: Option<AfterExecuteHook>,
    }

    impl FakeExecutor {
        fn new(
            responses: HashMap<String, FakeExecutorResponse>,
            observed_chunk_ids: Arc<Mutex<Vec<String>>>,
            after_execute: Option<AfterExecuteHook>,
        ) -> Self {
            Self {
                responses,
                observed_chunk_ids,
                after_execute,
            }
        }
    }

    impl TranslateChunkExecutor for FakeExecutor {
        fn execute(
            &self,
            request: &TranslateChunkActionRequest,
        ) -> Result<TranslateChunkModelOutput, TranslateChunkExecutionFailure> {
            self.observed_chunk_ids
                .lock()
                .expect("chunk order lock should open")
                .push(request.chunk_id.clone());

            if let Some(after_execute) = &self.after_execute {
                after_execute(request);
            }

            match self.responses.get(&request.chunk_id) {
                Some(FakeExecutorResponse::Success(response)) => Ok(TranslateChunkModelOutput {
                    provider: "fake".to_owned(),
                    model: "fake-model".to_owned(),
                    raw_output: serde_json::to_string(response)
                        .expect("fake response should serialize"),
                }),
                Some(FakeExecutorResponse::Failure(error)) => Err(error.clone()),
                None => panic!("missing fake response for chunk {}", request.chunk_id),
            }
        }
    }

    #[test]
    fn get_translate_document_job_status_aggregates_chunk_progress() {
        let fixture = create_runtime_fixture();
        seed_translate_document_graph(&fixture.runtime);
        let executor = FakeExecutor::new(
            HashMap::from([
                (
                    CHUNK_ID_1.to_owned(),
                    FakeExecutorResponse::Success(success_response_for_first_chunk()),
                ),
                (
                    CHUNK_ID_2.to_owned(),
                    FakeExecutorResponse::Failure(TranslateChunkExecutionFailure {
                        message: "The OpenAI translation request could not be completed."
                            .to_owned(),
                        details: Some("network".to_owned()),
                        raw_output: None,
                    }),
                ),
            ]),
            Arc::new(Mutex::new(Vec::new())),
            None,
        );

        run_translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_status_001".to_owned()),
            },
            TranslateDocumentExecutionMode::Fresh,
            &fixture.runtime,
            &executor,
        )
        .expect("translate_document should finish");

        let status = get_translate_document_job_status_with_runtime(
            TranslateDocumentJobInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: "job_status_001".to_owned(),
            },
            &fixture.runtime,
        )
        .expect("job status should load");

        assert_eq!(
            status.status,
            TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS
        );
        assert_eq!(status.total_chunks, 2);
        assert_eq!(status.completed_chunks, 1);
        assert_eq!(status.failed_chunks, 1);
        assert_eq!(status.pending_chunks, 0);
        assert_eq!(status.running_chunks, 0);
        assert_eq!(status.cancelled_chunks, 0);
        assert_eq!(status.chunk_statuses.len(), 2);
        assert!(status
            .error_messages
            .iter()
            .any(|message| message.contains("failed chunk")));
    }

    #[test]
    fn cancel_translate_document_job_marks_running_attempt_cancelled() {
        let fixture = create_runtime_fixture();
        seed_translate_document_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "trun_cancel_001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                job_id: Some("job_cancel_001".to_owned()),
                action_type: TRANSLATE_DOCUMENT_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: None,
                output_payload: None,
                error_message: None,
                started_at: 1_900_000_100,
                completed_at: None,
                created_at: 1_900_000_100,
                updated_at: 1_900_000_100,
            })
            .expect("running document task run should persist");
        drop(connection);

        let status = cancel_translate_document_job_with_runtime(
            TranslateDocumentJobInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: "job_cancel_001".to_owned(),
            },
            &fixture.runtime,
        )
        .expect("job cancellation should succeed");

        assert_eq!(status.status, TRANSLATE_DOCUMENT_STATUS_CANCELLED);
        assert_eq!(status.cancelled_chunks, 2);
        assert_eq!(
            status
                .latest_document_task_run
                .as_ref()
                .map(|task_run| task_run.status.as_str()),
            Some(TASK_RUN_STATUS_CANCELLED)
        );
    }

    #[test]
    fn resume_translate_document_job_skips_completed_chunks() {
        let fixture = create_runtime_fixture();
        seed_translate_document_graph(&fixture.runtime);
        let first_executor = FakeExecutor::new(
            HashMap::from([
                (
                    CHUNK_ID_1.to_owned(),
                    FakeExecutorResponse::Success(success_response_for_first_chunk()),
                ),
                (
                    CHUNK_ID_2.to_owned(),
                    FakeExecutorResponse::Failure(TranslateChunkExecutionFailure {
                        message: "The OpenAI translation request could not be completed."
                            .to_owned(),
                        details: Some("network".to_owned()),
                        raw_output: None,
                    }),
                ),
            ]),
            Arc::new(Mutex::new(Vec::new())),
            None,
        );

        run_translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_resume_001".to_owned()),
            },
            TranslateDocumentExecutionMode::Fresh,
            &fixture.runtime,
            &first_executor,
        )
        .expect("first attempt should finish");

        let observed_chunk_ids = Arc::new(Mutex::new(Vec::new()));
        let resume_executor = FakeExecutor::new(
            HashMap::from([(
                CHUNK_ID_2.to_owned(),
                FakeExecutorResponse::Success(success_response_for_second_chunk()),
            )]),
            observed_chunk_ids.clone(),
            None,
        );

        let result = resume_translate_document_job_with_runtime_and_executor(
            TranslateDocumentJobInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: "job_resume_001".to_owned(),
            },
            &fixture.runtime,
            &resume_executor,
        )
        .expect("resume should finish");

        assert_eq!(result.status, TRANSLATE_DOCUMENT_STATUS_COMPLETED);
        assert_eq!(result.completed_chunks, 2);
        assert_eq!(result.failed_chunks, 0);
        assert_eq!(
            observed_chunk_ids
                .lock()
                .expect("chunk order lock should open")
                .clone(),
            vec![CHUNK_ID_2.to_owned()]
        );
    }

    #[test]
    fn translate_document_stops_after_cancellation_request() {
        let fixture = create_runtime_fixture();
        seed_translate_document_graph(&fixture.runtime);
        let observed_chunk_ids = Arc::new(Mutex::new(Vec::new()));
        let runtime_for_hook = fixture.runtime.clone();
        let after_execute: AfterExecuteHook = Arc::new(move |request| {
            if request.chunk_id != CHUNK_ID_1 {
                return;
            }

            let mut connection = runtime_for_hook
                .open_connection()
                .expect("hook connection should open");
            let task_runs = TaskRunRepository::new(&mut connection)
                .list_by_job_id("job_cancel_during_run_001")
                .expect("hook task runs should load");
            let document_task_run_id = task_runs
                .into_iter()
                .filter(|task_run| {
                    task_run.action_type == TRANSLATE_DOCUMENT_ACTION_TYPE
                        && task_run.status == TASK_RUN_STATUS_RUNNING
                })
                .map(|task_run| task_run.id)
                .next()
                .expect("running document task run should exist");

            TaskRunRepository::new(&mut connection)
                .mark_cancelled(
                    &document_task_run_id,
                    "Cancellation requested by the user.",
                    None,
                    1_900_000_500,
                )
                .expect("hook cancellation should persist");
        });
        let executor = FakeExecutor::new(
            HashMap::from([
                (
                    CHUNK_ID_1.to_owned(),
                    FakeExecutorResponse::Success(success_response_for_first_chunk()),
                ),
                (
                    CHUNK_ID_2.to_owned(),
                    FakeExecutorResponse::Success(success_response_for_second_chunk()),
                ),
            ]),
            observed_chunk_ids.clone(),
            Some(after_execute),
        );

        let result = run_translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_cancel_during_run_001".to_owned()),
            },
            TranslateDocumentExecutionMode::Fresh,
            &fixture.runtime,
            &executor,
        )
        .expect("translate_document should return after cancellation");

        assert_eq!(result.status, TRANSLATE_DOCUMENT_STATUS_CANCELLED);
        assert_eq!(
            observed_chunk_ids
                .lock()
                .expect("chunk order lock should open")
                .clone(),
            vec![CHUNK_ID_1.to_owned()]
        );

        let status = get_translate_document_job_status_with_runtime(
            TranslateDocumentJobInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: "job_cancel_during_run_001".to_owned(),
            },
            &fixture.runtime,
        )
        .expect("job status should load");

        assert_eq!(status.status, TRANSLATE_DOCUMENT_STATUS_CANCELLED);
        assert_eq!(status.completed_chunks, 1);
        assert_eq!(status.cancelled_chunks, 1);
    }

    #[test]
    fn job_status_is_isolated_by_job_id() {
        let fixture = create_runtime_fixture();
        seed_translate_document_graph(&fixture.runtime);
        let success_executor = FakeExecutor::new(
            HashMap::from([
                (
                    CHUNK_ID_1.to_owned(),
                    FakeExecutorResponse::Success(success_response_for_first_chunk()),
                ),
                (
                    CHUNK_ID_2.to_owned(),
                    FakeExecutorResponse::Success(success_response_for_second_chunk()),
                ),
            ]),
            Arc::new(Mutex::new(Vec::new())),
            None,
        );
        run_translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_isolated_ok".to_owned()),
            },
            TranslateDocumentExecutionMode::Fresh,
            &fixture.runtime,
            &success_executor,
        )
        .expect("successful job should finish");

        let mixed_executor = FakeExecutor::new(
            HashMap::from([
                (
                    CHUNK_ID_1.to_owned(),
                    FakeExecutorResponse::Success(success_response_for_first_chunk()),
                ),
                (
                    CHUNK_ID_2.to_owned(),
                    FakeExecutorResponse::Failure(TranslateChunkExecutionFailure {
                        message: "The OpenAI translation request could not be completed."
                            .to_owned(),
                        details: Some("network".to_owned()),
                        raw_output: None,
                    }),
                ),
            ]),
            Arc::new(Mutex::new(Vec::new())),
            None,
        );
        run_translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_isolated_err".to_owned()),
            },
            TranslateDocumentExecutionMode::Fresh,
            &fixture.runtime,
            &mixed_executor,
        )
        .expect("mixed job should finish");

        let success_status = get_translate_document_job_status_with_runtime(
            TranslateDocumentJobInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: "job_isolated_ok".to_owned(),
            },
            &fixture.runtime,
        )
        .expect("successful job status should load");
        let mixed_status = get_translate_document_job_status_with_runtime(
            TranslateDocumentJobInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: "job_isolated_err".to_owned(),
            },
            &fixture.runtime,
        )
        .expect("mixed job status should load");

        assert_eq!(success_status.status, TRANSLATE_DOCUMENT_STATUS_COMPLETED);
        assert_eq!(
            mixed_status.status,
            TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS
        );
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

    fn success_response_for_first_chunk() -> TranslateChunkActionResponse {
        TranslateChunkActionResponse {
            translations: vec![TranslateChunkTranslation {
                segment_id: "seg_doc_translate_001_0002".to_owned(),
                target_text: "El guardia mantiene la puerta.".to_owned(),
            }],
            notes: Some("chunk-one".to_owned()),
        }
    }

    fn success_response_for_second_chunk() -> TranslateChunkActionResponse {
        TranslateChunkActionResponse {
            translations: vec![TranslateChunkTranslation {
                segment_id: "seg_doc_translate_001_0003".to_owned(),
                target_text: "La orden permanece activa.".to_owned(),
            }],
            notes: Some("chunk-two".to_owned()),
        }
    }

    fn seed_translate_document_graph(runtime: &DatabaseRuntime) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_900_000_000_i64;

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: PROJECT_ID.to_owned(),
                name: "Translate document project".to_owned(),
                description: None,
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("project should persist");
        ProjectRepository::new(&mut connection)
            .open_project(PROJECT_ID, now + 1)
            .expect("project should become active");

        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: DOCUMENT_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
                name: "chapter-one.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "chapter-one.txt".to_owned(),
                file_size_bytes: 256,
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
                        id: "seg_doc_translate_001_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        source_text: "Chapter 1".to_owned(),
                        source_word_count: 2,
                        source_character_count: 9,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "seg_doc_translate_001_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        source_text: "The guard keeps the gate.".to_owned(),
                        source_word_count: 5,
                        source_character_count: 25,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "seg_doc_translate_001_0003".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 3,
                        source_text: "The order stays active.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 23,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "seg_doc_translate_001_0004".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 4,
                        source_text: "Closing note.".to_owned(),
                        source_word_count: 2,
                        source_character_count: 13,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                ],
                now,
            )
            .expect("segments should persist");

        DocumentSectionRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[NewDocumentSection {
                    id: "sec_doc_translate_001_0001".to_owned(),
                    document_id: DOCUMENT_ID.to_owned(),
                    sequence: 1,
                    title: "Chapter 1".to_owned(),
                    section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
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
                DOCUMENT_ID,
                &[
                    NewTranslationChunk {
                        id: CHUNK_ID_1.to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "The guard keeps the gate.".to_owned(),
                        context_before_text: Some("Chapter 1".to_owned()),
                        context_after_text: Some("The order stays active.".to_owned()),
                        start_segment_sequence: 2,
                        end_segment_sequence: 2,
                        segment_count: 1,
                        source_word_count: 5,
                        source_character_count: 25,
                        created_at: now,
                        updated_at: now,
                    },
                    NewTranslationChunk {
                        id: CHUNK_ID_2.to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "The order stays active.".to_owned(),
                        context_before_text: Some("The guard keeps the gate.".to_owned()),
                        context_after_text: Some("Closing note.".to_owned()),
                        start_segment_sequence: 3,
                        end_segment_sequence: 3,
                        segment_count: 1,
                        source_word_count: 4,
                        source_character_count: 23,
                        created_at: now,
                        updated_at: now,
                    },
                ],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID_1.to_owned(),
                        segment_id: "seg_doc_translate_001_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID_1.to_owned(),
                        segment_id: "seg_doc_translate_001_0002".to_owned(),
                        segment_sequence: 2,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID_1.to_owned(),
                        segment_id: "seg_doc_translate_001_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID_2.to_owned(),
                        segment_id: "seg_doc_translate_001_0002".to_owned(),
                        segment_sequence: 2,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID_2.to_owned(),
                        segment_id: "seg_doc_translate_001_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID_2.to_owned(),
                        segment_id: "seg_doc_translate_001_0004".to_owned(),
                        segment_sequence: 4,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                ],
            )
            .expect("translation chunks should persist");
    }
}
