use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::json;
use tauri::State;

use crate::commands::segments::load_segmented_document_overview;
use crate::commands::translate_chunk::translate_chunk_with_runtime_and_executor;
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::task_runs::TaskRunRepository;
use crate::persistence::translation_chunks::TranslationChunkRepository;
use crate::task_runs::{NewTaskRun, TaskRunSummary, TASK_RUN_STATUS_RUNNING};
use crate::translate_chunk::{
    OpenAiTranslateChunkExecutor, TranslateChunkExecutor, TranslateChunkInput,
    TRANSLATE_CHUNK_ACTION_TYPE,
};
use crate::translate_document::{
    TranslateDocumentChunkResult, TranslateDocumentInput, TranslateDocumentResult,
    TRANSLATE_DOCUMENT_ACTION_TYPE, TRANSLATE_DOCUMENT_ACTION_VERSION,
    TRANSLATE_DOCUMENT_CHUNK_STATUS_COMPLETED, TRANSLATE_DOCUMENT_CHUNK_STATUS_FAILED,
    TRANSLATE_DOCUMENT_STATUS_COMPLETED, TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS,
    TRANSLATE_DOCUMENT_STATUS_FAILED,
};

#[tauri::command]
pub fn translate_document(
    input: TranslateDocumentInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<TranslateDocumentResult, DesktopCommandError> {
    let executor = OpenAiTranslateChunkExecutor::from_environment()?;

    translate_document_with_runtime_and_executor(input, database_runtime.inner(), &executor)
}

pub(crate) fn translate_document_with_runtime_and_executor<E: TranslateChunkExecutor>(
    input: TranslateDocumentInput,
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
    let _ = load_segmented_document_overview(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        false,
        started_at,
    )?;
    let chunks = TranslationChunkRepository::new(&mut connection)
        .list_chunks_by_document(&document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load translation chunks for translate_document.",
                Some(error.to_string()),
            )
        })?;

    if chunks.is_empty() {
        return Err(DesktopCommandError::validation(
            "The selected document must have persisted translation chunks before translate_document can start.",
            None,
        ));
    }

    let job_id = normalize_optional_identifier(input.job_id, "job id")?
        .unwrap_or_else(|| generate_job_id(started_at));
    let task_run_id = generate_task_run_id(started_at);
    let input_payload = serde_json::to_string(&json!({
        "projectId": project_id,
        "documentId": document_id,
        "jobId": job_id,
        "actionVersion": TRANSLATE_DOCUMENT_ACTION_VERSION,
        "chunkIds": chunks.iter().map(|chunk| chunk.id.as_str()).collect::<Vec<_>>()
    }))
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not serialize the translate_document input payload.",
            Some(error.to_string()),
        )
    })?;
    let _ = TaskRunRepository::new(&mut connection)
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

    let total_chunks = i64::try_from(chunks.len()).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid translation chunk count for translate_document.",
            Some(error.to_string()),
        )
    })?;
    let mut completed_chunks = 0_i64;
    let mut failed_chunks = 0_i64;
    let mut chunk_results = Vec::with_capacity(chunks.len());
    let mut error_messages = Vec::new();

    for chunk in chunks {
        match translate_chunk_with_runtime_and_executor(
            TranslateChunkInput {
                project_id: project_id.clone(),
                document_id: document_id.clone(),
                chunk_id: chunk.id.clone(),
                job_id: Some(job_id.clone()),
            },
            database_runtime,
            executor,
        ) {
            Ok(result) => {
                completed_chunks += 1;
                let translated_segment_count =
                    i64::try_from(result.translated_segments.len()).map_err(|error| {
                        DesktopCommandError::internal(
                            "The desktop shell produced an invalid translated segment count for translate_document.",
                            Some(error.to_string()),
                        )
                    })?;
                chunk_results.push(TranslateDocumentChunkResult {
                    chunk_id: chunk.id,
                    chunk_sequence: chunk.sequence,
                    status: TRANSLATE_DOCUMENT_CHUNK_STATUS_COMPLETED.to_owned(),
                    task_run: Some(result.task_run),
                    translated_segment_count,
                    error_message: None,
                });
            }
            Err(error) => {
                failed_chunks += 1;
                error_messages.push(format!(
                    "Chunk {} failed: {}",
                    chunk.sequence, error.message
                ));
                chunk_results.push(TranslateDocumentChunkResult {
                    chunk_id: chunk.id.clone(),
                    chunk_sequence: chunk.sequence,
                    status: TRANSLATE_DOCUMENT_CHUNK_STATUS_FAILED.to_owned(),
                    task_run: load_latest_chunk_task_run(database_runtime, &chunk.id, &job_id)?,
                    translated_segment_count: 0,
                    error_message: Some(error.message),
                });
            }
        }
    }

    let status = if failed_chunks == 0 {
        TRANSLATE_DOCUMENT_STATUS_COMPLETED.to_owned()
    } else if completed_chunks == 0 {
        TRANSLATE_DOCUMENT_STATUS_FAILED.to_owned()
    } else {
        TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS.to_owned()
    };
    let completed_at = current_timestamp()?;
    let output_payload = serde_json::to_string(&json!({
        "actionVersion": TRANSLATE_DOCUMENT_ACTION_VERSION,
        "jobId": job_id,
        "status": status,
        "totalChunks": total_chunks,
        "completedChunks": completed_chunks,
        "failedChunks": failed_chunks,
        "chunkResults": chunk_results,
        "errorMessages": error_messages
    }))
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not serialize the translate_document output payload.",
            Some(error.to_string()),
        )
    })?;
    let task_run = if failed_chunks == 0 {
        TaskRunRepository::new(&mut connection)
            .mark_completed(&task_run_id, &output_payload, completed_at)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not finalize the translate_document task run.",
                    Some(error.to_string()),
                )
            })?
    } else {
        TaskRunRepository::new(&mut connection)
            .mark_failed(
                &task_run_id,
                &format!(
                    "translate_document finished with {failed_chunks} failed chunk(s) out of {total_chunks}."
                ),
                Some(&output_payload),
                completed_at,
            )
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not finalize the failed translate_document task run.",
                    Some(error.to_string()),
                )
            })?
    };

    Ok(TranslateDocumentResult {
        project_id,
        document_id,
        job_id,
        status,
        action_version: TRANSLATE_DOCUMENT_ACTION_VERSION.to_owned(),
        task_run,
        total_chunks,
        completed_chunks,
        failed_chunks,
        chunk_results,
        error_messages,
    })
}

fn load_latest_chunk_task_run(
    database_runtime: &DatabaseRuntime,
    chunk_id: &str,
    job_id: &str,
) -> Result<Option<TaskRunSummary>, DesktopCommandError> {
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not reopen the encrypted database to inspect translate_document chunk task runs.",
            Some(error.to_string()),
        )
    })?;

    TaskRunRepository::new(&mut connection)
        .list_by_chunk(chunk_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not inspect chunk task runs for translate_document.",
                Some(error.to_string()),
            )
        })
        .map(|task_runs| {
            task_runs.into_iter().rev().find(|task_run| {
                task_run.job_id.as_deref() == Some(job_id)
                    && task_run.action_type == TRANSLATE_CHUNK_ACTION_TYPE
            })
        })
}

fn normalize_optional_identifier(
    value: Option<String>,
    label: &str,
) -> Result<Option<String>, DesktopCommandError> {
    value
        .map(|value| validate_identifier(&value, label))
        .transpose()
}

fn validate_identifier(value: &str, label: &str) -> Result<String, DesktopCommandError> {
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

fn generate_job_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "job_{}_{}",
        timestamp,
        URL_SAFE_NO_PAD.encode(random_part.to_le_bytes())
    )
}

fn generate_task_run_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "trun_{}_{}",
        timestamp,
        URL_SAFE_NO_PAD.encode(random_part.to_le_bytes())
    )
}

fn current_timestamp() -> Result<i64, DesktopCommandError> {
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

    use super::translate_document_with_runtime_and_executor;
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
    use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_COMPLETED, TASK_RUN_STATUS_FAILED};
    use crate::translate_chunk::{
        TranslateChunkActionRequest, TranslateChunkActionResponse, TranslateChunkExecutionFailure,
        TranslateChunkExecutor, TranslateChunkModelOutput, TranslateChunkTranslation,
        TRANSLATE_CHUNK_ACTION_TYPE,
    };
    use crate::translate_document::{
        TranslateDocumentInput, TRANSLATE_DOCUMENT_ACTION_TYPE,
        TRANSLATE_DOCUMENT_STATUS_COMPLETED, TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS,
    };
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const PROJECT_ID: &str = "prj_active_001";
    const DOCUMENT_ID: &str = "doc_translate_001";
    const EMPTY_DOCUMENT_ID: &str = "doc_empty_001";
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

    struct FakeExecutor {
        responses: HashMap<String, FakeExecutorResponse>,
        observed_chunk_ids: Arc<Mutex<Vec<String>>>,
    }

    impl FakeExecutor {
        fn new(
            responses: HashMap<String, FakeExecutorResponse>,
            observed_chunk_ids: Arc<Mutex<Vec<String>>>,
        ) -> Self {
            Self {
                responses,
                observed_chunk_ids,
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
    fn translate_document_rejects_invalid_identifiers() {
        let fixture = create_runtime_fixture();

        let error = translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: " ".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &FakeExecutor::new(HashMap::new(), Arc::new(Mutex::new(Vec::new()))),
        )
        .expect_err("invalid ids should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("project id"));
    }

    #[test]
    fn translate_document_rejects_documents_without_chunks() {
        let fixture = create_runtime_fixture();
        seed_translate_document_graph(&fixture.runtime);

        let error = translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: EMPTY_DOCUMENT_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &FakeExecutor::new(HashMap::new(), Arc::new(Mutex::new(Vec::new()))),
        )
        .expect_err("documents without chunks should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("persisted translation chunks"));
    }

    #[test]
    fn translate_document_runs_chunks_in_order_and_groups_task_runs_by_job() {
        let fixture = create_runtime_fixture();
        seed_translate_document_graph(&fixture.runtime);
        let observed_chunk_ids = Arc::new(Mutex::new(Vec::new()));
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
        );

        let result = translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_translate_doc_001".to_owned()),
            },
            &fixture.runtime,
            &executor,
        )
        .expect("translate_document should succeed");

        assert_eq!(result.status, TRANSLATE_DOCUMENT_STATUS_COMPLETED);
        assert_eq!(result.job_id, "job_translate_doc_001");
        assert_eq!(result.total_chunks, 2);
        assert_eq!(result.completed_chunks, 2);
        assert_eq!(result.failed_chunks, 0);
        assert_eq!(
            observed_chunk_ids
                .lock()
                .expect("chunk order lock should open")
                .clone(),
            vec![CHUNK_ID_1.to_owned(), CHUNK_ID_2.to_owned()]
        );

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let job_runs = TaskRunRepository::new(&mut connection)
            .list_by_job_id("job_translate_doc_001")
            .expect("job task runs should load");

        assert_eq!(job_runs.len(), 3);
        assert_eq!(
            job_runs
                .iter()
                .filter(|task_run| task_run.action_type == TRANSLATE_DOCUMENT_ACTION_TYPE)
                .count(),
            1
        );
        assert_eq!(
            job_runs
                .iter()
                .filter(|task_run| task_run.action_type == TRANSLATE_CHUNK_ACTION_TYPE)
                .count(),
            2
        );
        assert!(job_runs.iter().any(|task_run| {
            task_run.action_type == TRANSLATE_DOCUMENT_ACTION_TYPE
                && task_run.status == TASK_RUN_STATUS_COMPLETED
        }));
        assert!(job_runs
            .iter()
            .all(|task_run| { task_run.job_id.as_deref() == Some("job_translate_doc_001") }));
    }

    #[test]
    fn translate_document_returns_partial_failures_and_records_chunk_errors() {
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
        );

        let result = translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_translate_doc_002".to_owned()),
            },
            &fixture.runtime,
            &executor,
        )
        .expect("partial failures should still return an aggregated result");

        assert_eq!(
            result.status,
            TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS
        );
        assert_eq!(result.total_chunks, 2);
        assert_eq!(result.completed_chunks, 1);
        assert_eq!(result.failed_chunks, 1);
        assert_eq!(result.error_messages.len(), 1);
        assert_eq!(result.task_run.status, TASK_RUN_STATUS_FAILED);
        assert_eq!(
            result
                .chunk_results
                .iter()
                .find(|chunk| chunk.chunk_id == CHUNK_ID_2)
                .and_then(|chunk| chunk.task_run.as_ref())
                .map(|task_run| task_run.status.as_str()),
            Some(TASK_RUN_STATUS_FAILED)
        );

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let job_runs = TaskRunRepository::new(&mut connection)
            .list_by_job_id("job_translate_doc_002")
            .expect("job task runs should load");

        assert_eq!(job_runs.len(), 3);
        assert_eq!(
            job_runs
                .iter()
                .filter(|task_run| task_run.status == TASK_RUN_STATUS_FAILED)
                .count(),
            2
        );
        assert!(job_runs.iter().any(|task_run| {
            task_run.action_type == TRANSLATE_DOCUMENT_ACTION_TYPE
                && task_run.error_message.as_deref()
                    == Some("translate_document finished with 1 failed chunk(s) out of 2.")
        }));
    }

    #[test]
    fn translate_document_keeps_existing_task_runs_isolated_by_job_id() {
        let fixture = create_runtime_fixture();
        seed_translate_document_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "trun_existing_001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some(CHUNK_ID_1.to_owned()),
                job_id: Some("job_previous_attempt".to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: None,
                output_payload: Some("{\"translations\":[]}".to_owned()),
                error_message: None,
                started_at: 1_900_000_100,
                completed_at: Some(1_900_000_101),
                created_at: 1_900_000_100,
                updated_at: 1_900_000_101,
            })
            .expect("existing task run should persist");
        drop(connection);

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
            Arc::new(Mutex::new(Vec::new())),
        );
        let result = translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_translate_doc_003".to_owned()),
            },
            &fixture.runtime,
            &executor,
        )
        .expect("translate_document should succeed");

        assert_eq!(result.status, TRANSLATE_DOCUMENT_STATUS_COMPLETED);

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let prior_runs = TaskRunRepository::new(&mut connection)
            .list_by_job_id("job_previous_attempt")
            .expect("prior job task runs should load");
        let current_runs = TaskRunRepository::new(&mut connection)
            .list_by_job_id("job_translate_doc_003")
            .expect("current job task runs should load");

        assert_eq!(prior_runs.len(), 1);
        assert_eq!(current_runs.len(), 3);
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
        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: EMPTY_DOCUMENT_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
                name: "empty.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "empty.txt".to_owned(),
                file_size_bytes: 32,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: now,
                updated_at: now,
            })
            .expect("empty document should persist");

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
        SegmentRepository::new(&mut connection)
            .replace_for_document(
                PROJECT_ID,
                EMPTY_DOCUMENT_ID,
                &[NewSegment {
                    id: "seg_doc_empty_001_0001".to_owned(),
                    document_id: EMPTY_DOCUMENT_ID.to_owned(),
                    sequence: 1,
                    source_text: "Empty baseline.".to_owned(),
                    source_word_count: 2,
                    source_character_count: 15,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: now,
                    updated_at: now,
                }],
                now,
            )
            .expect("empty document segments should persist");

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
        DocumentSectionRepository::new(&mut connection)
            .replace_for_document(
                EMPTY_DOCUMENT_ID,
                &[NewDocumentSection {
                    id: "sec_doc_empty_001_0001".to_owned(),
                    document_id: EMPTY_DOCUMENT_ID.to_owned(),
                    sequence: 1,
                    title: "Empty".to_owned(),
                    section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                    level: 1,
                    start_segment_sequence: 1,
                    end_segment_sequence: 1,
                    segment_count: 1,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect("empty sections should persist");

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
