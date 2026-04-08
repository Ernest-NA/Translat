use tauri::State;

use crate::commands::translate_document_jobs::{
    run_translate_document_with_runtime_and_executor, TranslateDocumentExecutionMode,
};
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::translate_chunk::{OpenAiTranslateChunkExecutor, TranslateChunkExecutor};
use crate::translate_document::{TranslateDocumentInput, TranslateDocumentResult};

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
    run_translate_document_with_runtime_and_executor(
        input,
        TranslateDocumentExecutionMode::Fresh,
        database_runtime,
        executor,
    )
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
        assert!(!result.error_messages.is_empty());
        assert!(result.error_messages.iter().any(|message| {
            message.contains("OpenAI translation request could not be completed")
                || message.contains("failed chunk")
        }));
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
                && task_run
                    .error_message
                    .as_deref()
                    .is_some_and(|message| message.contains("failed chunk"))
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
