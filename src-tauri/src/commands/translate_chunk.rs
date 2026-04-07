use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::json;
use tauri::State;

use crate::commands::segments::load_segmented_document_overview;
use crate::context_builder::{
    build_translation_context as compose_translation_context, BuildTranslationContextInput,
};
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::task_runs::TaskRunRepository;
use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_CANCELLED, TASK_RUN_STATUS_RUNNING};
use crate::translate_chunk::{
    build_action_request, parse_action_response, serialize_task_run_output,
    validate_action_response, OpenAiTranslateChunkExecutor, TranslateChunkExecutionFailure,
    TranslateChunkExecutor, TranslateChunkInput, TranslateChunkModelOutput, TranslateChunkResult,
    TRANSLATE_CHUNK_ACTION_TYPE, TRANSLATE_CHUNK_ACTION_VERSION,
    TRANSLATE_CHUNK_PROMPT_VERSION,
};

#[tauri::command]
pub fn translate_chunk(
    input: TranslateChunkInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<TranslateChunkResult, DesktopCommandError> {
    let executor = OpenAiTranslateChunkExecutor::from_environment()?;

    translate_chunk_with_runtime_and_executor(input, database_runtime.inner(), &executor)
}

pub(crate) fn translate_chunk_with_runtime_and_executor<E: TranslateChunkExecutor>(
    input: TranslateChunkInput,
    database_runtime: &DatabaseRuntime,
    executor: &E,
) -> Result<TranslateChunkResult, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let chunk_id = validate_identifier(&input.chunk_id, "chunk id")?;
    let job_id = normalize_optional_identifier(input.job_id, "job id")?;
    let started_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for translate_chunk.",
            Some(error.to_string()),
        )
    })?;
    let segment_overview = load_segmented_document_overview(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        false,
        0,
    )?;
    let context = compose_translation_context(
        &mut connection,
        &segment_overview,
        &BuildTranslationContextInput {
            project_id: project_id.clone(),
            document_id: document_id.clone(),
            chunk_id: chunk_id.clone(),
            action_scope: "translation".to_owned(),
        },
    )?;
    let action_request = build_action_request(&context);
    let input_payload = serde_json::to_string(&action_request).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not serialize the translate_chunk input payload.",
            Some(error.to_string()),
        )
    })?;
    let task_run_id = generate_task_run_id(started_at);
    let _ = TaskRunRepository::new(&mut connection)
        .create(&NewTaskRun {
            id: task_run_id.clone(),
            document_id: document_id.clone(),
            chunk_id: Some(chunk_id.clone()),
            job_id: job_id.clone(),
            action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
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
                "The desktop shell could not open a task run for translate_chunk.",
                Some(error.to_string()),
            )
        })?;

    if is_task_run_cancelled(&mut connection, &task_run_id)? {
        return Err(cancellation_error());
    }

    let model_output = match executor.execute(&action_request) {
        Ok(model_output) => model_output,
        Err(execution_error) => {
            let desktop_error = DesktopCommandError::internal(
                execution_error.message.clone(),
                execution_error.details.clone(),
            );
            best_effort_mark_task_run_failed(
                &mut connection,
                &task_run_id,
                &desktop_error.message,
                failure_output_payload(&execution_error),
                current_timestamp().unwrap_or(started_at),
            );
            return Err(desktop_error);
        }
    };

    let action_response = match parse_action_response(&model_output.raw_output) {
        Ok(action_response) => action_response,
        Err(error) => {
            best_effort_mark_task_run_failed(
                &mut connection,
                &task_run_id,
                &error.message,
                Some(raw_model_output_payload(&model_output)),
                current_timestamp().unwrap_or(started_at),
            );
            return Err(error);
        }
    };
    let validated_segments = match validate_action_response(&action_request, &action_response) {
        Ok(validated_segments) => validated_segments,
        Err(error) => {
            best_effort_mark_task_run_failed(
                &mut connection,
                &task_run_id,
                &error.message,
                Some(raw_model_output_payload(&model_output)),
                current_timestamp().unwrap_or(started_at),
            );
            return Err(error);
        }
    };

    if is_task_run_cancelled(&mut connection, &task_run_id)? {
        return Err(cancellation_error());
    }

    let completed_at = current_timestamp()?;
    let segment_writes = validated_segments
        .iter()
        .map(|segment| segment.to_segment_write())
        .collect::<Vec<_>>();
    let output_payload =
        serialize_task_run_output(&model_output.provider, &model_output.model, &action_response)?;

    let task_run = TaskRunRepository::new(&mut connection)
        .mark_completed_with_translation_projection(
            &project_id,
            &document_id,
            &task_run_id,
            &output_payload,
            &segment_writes,
            completed_at,
        )
        .map_err(|error| {
            let desktop_error = DesktopCommandError::internal(
                "The desktop shell could not finalize translated output for translate_chunk.",
                Some(error.to_string()),
            );
            best_effort_mark_task_run_failed(
                &mut connection,
                &task_run_id,
                &desktop_error.message,
                Some(raw_model_output_payload(&model_output)),
                completed_at,
            );
            desktop_error
        })?;

    Ok(TranslateChunkResult {
        project_id,
        document_id,
        chunk_id,
        task_run,
        provider: model_output.provider,
        model: model_output.model,
        action_version: TRANSLATE_CHUNK_ACTION_VERSION.to_owned(),
        prompt_version: TRANSLATE_CHUNK_PROMPT_VERSION.to_owned(),
        translated_segments: validated_segments
            .iter()
            .map(|segment| segment.to_summary())
            .collect(),
    })
}

fn best_effort_mark_task_run_failed(
    connection: &mut rusqlite::Connection,
    task_run_id: &str,
    error_message: &str,
    output_payload: Option<String>,
    failed_at: i64,
) {
    let _ = TaskRunRepository::new(connection).mark_failed(
        task_run_id,
        error_message,
        output_payload.as_deref(),
        failed_at,
    );
}

fn raw_model_output_payload(model_output: &TranslateChunkModelOutput) -> String {
    json!({
        "provider": model_output.provider,
        "model": model_output.model,
        "rawOutput": model_output.raw_output
    })
    .to_string()
}

fn failure_output_payload(execution_error: &TranslateChunkExecutionFailure) -> Option<String> {
    execution_error.raw_output.as_ref().map(|raw_output| {
        json!({
            "rawOutput": raw_output
        })
        .to_string()
    })
}

fn normalize_optional_identifier(
    value: Option<String>,
    label: &str,
) -> Result<Option<String>, DesktopCommandError> {
    value.map(|value| validate_identifier(&value, label)).transpose()
}

fn validate_identifier(value: &str, label: &str) -> Result<String, DesktopCommandError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(DesktopCommandError::validation(
            format!("The translate_chunk action requires a valid {label}."),
            None,
        ));
    }

    if !trimmed
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(
            format!("The translate_chunk action requires a safe persisted {label}."),
            None,
        ));
    }

    Ok(trimmed.to_owned())
}

fn is_task_run_cancelled(
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
                "The desktop shell could not inspect translate_chunk cancellation state.",
                Some(error.to_string()),
            )
        })
}

fn cancellation_error() -> DesktopCommandError {
    DesktopCommandError::internal(
        "The translate_chunk action was cancelled before translated output could be finalized.",
        None,
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
                    "The desktop shell could not compute the current translate_chunk timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid translate_chunk timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use tempfile::{tempdir, TempDir};

    use super::translate_chunk_with_runtime_and_executor;
    use crate::chapter_contexts::{NewChapterContext, CHAPTER_CONTEXT_SCOPE_DOCUMENT};
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::glossaries::{NewGlossary, GLOSSARY_STATUS_ACTIVE};
    use crate::glossary_entries::{NewGlossaryEntry, GLOSSARY_ENTRY_STATUS_ACTIVE};
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::chapter_contexts::ChapterContextRepository;
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::glossaries::GlossaryRepository;
    use crate::persistence::glossary_entries::GlossaryEntryRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::rule_sets::{RuleRepository, RuleSetRepository};
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::persistence::sections::DocumentSectionRepository;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::style_profiles::StyleProfileRepository;
    use crate::persistence::task_runs::TaskRunRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::{NewProject, ProjectEditorialDefaultsChanges};
    use crate::rule_sets::{
        NewRule, NewRuleSet, RULE_ACTION_SCOPE_QA, RULE_ACTION_SCOPE_TRANSLATION,
        RULE_SET_STATUS_ACTIVE, RULE_SEVERITY_HIGH, RULE_SEVERITY_LOW,
        RULE_TYPE_CONSISTENCY, RULE_TYPE_PREFERENCE,
    };
    use crate::sections::{NewDocumentSection, DOCUMENT_SECTION_TYPE_CHAPTER};
    use crate::segments::{
        NewSegment, SEGMENT_STATUS_PENDING_TRANSLATION, SEGMENT_STATUS_TRANSLATED,
    };
    use crate::style_profiles::{
        NewStyleProfile, STYLE_PROFILE_FORMALITY_FORMAL, STYLE_PROFILE_STATUS_ACTIVE,
        STYLE_PROFILE_TONE_TECHNICAL, STYLE_PROFILE_TREATMENT_USTED,
    };
    use crate::task_runs::{
        TaskRunSummary, TASK_RUN_STATUS_COMPLETED, TASK_RUN_STATUS_FAILED,
    };
    use crate::translate_chunk::{
        TranslateChunkActionRequest, TranslateChunkActionResponse,
        TranslateChunkExecutionFailure, TranslateChunkExecutor, TranslateChunkInput,
        TranslateChunkModelOutput, TranslateChunkTranslation,
    };
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const PROJECT_ID: &str = "prj_active_001";
    const DOCUMENT_ID: &str = "doc_chunk_001";
    const OTHER_DOCUMENT_ID: &str = "doc_other_001";
    const CHUNK_ID: &str = "doc_chunk_001_chunk_0001";
    const OTHER_CHUNK_ID: &str = "doc_other_001_chunk_0001";

    struct RuntimeFixture {
        _temporary_directory: TempDir,
        runtime: DatabaseRuntime,
        database_path: PathBuf,
        encryption_key_path: PathBuf,
    }

    #[derive(Clone)]
    enum FakeExecutorResponse {
        Success(TranslateChunkActionResponse),
        Raw(String),
        Failure(TranslateChunkExecutionFailure),
    }

    struct FakeExecutor {
        runtime: Option<DatabaseRuntime>,
        observed_requests: Arc<Mutex<Vec<TranslateChunkActionRequest>>>,
        observed_task_runs: Arc<Mutex<Vec<TaskRunSummary>>>,
        response: FakeExecutorResponse,
    }

    impl FakeExecutor {
        fn success(
            runtime: DatabaseRuntime,
            response: TranslateChunkActionResponse,
            observed_requests: Arc<Mutex<Vec<TranslateChunkActionRequest>>>,
            observed_task_runs: Arc<Mutex<Vec<TaskRunSummary>>>,
        ) -> Self {
            Self {
                runtime: Some(runtime),
                observed_requests,
                observed_task_runs,
                response: FakeExecutorResponse::Success(response),
            }
        }

        fn raw(
            runtime: DatabaseRuntime,
            raw_output: &str,
            observed_requests: Arc<Mutex<Vec<TranslateChunkActionRequest>>>,
            observed_task_runs: Arc<Mutex<Vec<TaskRunSummary>>>,
        ) -> Self {
            Self {
                runtime: Some(runtime),
                observed_requests,
                observed_task_runs,
                response: FakeExecutorResponse::Raw(raw_output.to_owned()),
            }
        }

        fn failure(
            runtime: DatabaseRuntime,
            failure: TranslateChunkExecutionFailure,
            observed_requests: Arc<Mutex<Vec<TranslateChunkActionRequest>>>,
            observed_task_runs: Arc<Mutex<Vec<TaskRunSummary>>>,
        ) -> Self {
            Self {
                runtime: Some(runtime),
                observed_requests,
                observed_task_runs,
                response: FakeExecutorResponse::Failure(failure),
            }
        }

        fn inert() -> Self {
            Self {
                runtime: None,
                observed_requests: Arc::new(Mutex::new(Vec::new())),
                observed_task_runs: Arc::new(Mutex::new(Vec::new())),
                response: FakeExecutorResponse::Raw(String::new()),
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
                .expect("executor requests lock should open")
                .push(request.clone());

            if let Some(runtime) = &self.runtime {
                let mut connection = runtime
                    .open_connection()
                    .expect("executor connection should open");
                let task_runs = TaskRunRepository::new(&mut connection)
                    .list_by_chunk(&request.chunk_id)
                    .expect("executor should load chunk task runs");
                self.observed_task_runs
                    .lock()
                    .expect("executor task-run lock should open")
                    .extend(task_runs);
            }

            match &self.response {
                FakeExecutorResponse::Success(response) => Ok(TranslateChunkModelOutput {
                    provider: "fake".to_owned(),
                    model: "fake-model".to_owned(),
                    raw_output: serde_json::to_string(response)
                        .expect("fake executor response should serialize"),
                }),
                FakeExecutorResponse::Raw(raw_output) => Ok(TranslateChunkModelOutput {
                    provider: "fake".to_owned(),
                    model: "fake-model".to_owned(),
                    raw_output: raw_output.clone(),
                }),
                FakeExecutorResponse::Failure(failure) => Err(failure.clone()),
            }
        }
    }

    #[test]
    fn translate_chunk_rejects_invalid_identifiers() {
        let fixture = create_runtime_fixture();

        let error = translate_chunk_with_runtime_and_executor(
            TranslateChunkInput {
                project_id: " ".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: CHUNK_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &FakeExecutor::inert(),
        )
        .expect_err("invalid ids should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("project id"));
    }

    #[test]
    fn translate_chunk_rejects_chunks_outside_the_selected_document() {
        let fixture = create_runtime_fixture();
        seed_translate_chunk_graph(&fixture.runtime);

        let observed_requests = Arc::new(Mutex::new(Vec::new()));
        let observed_task_runs = Arc::new(Mutex::new(Vec::new()));
        let executor = FakeExecutor::success(
            clone_runtime(&fixture),
            successful_action_response(),
            observed_requests.clone(),
            observed_task_runs,
        );

        let error = translate_chunk_with_runtime_and_executor(
            TranslateChunkInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: OTHER_CHUNK_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &executor,
        )
        .expect_err("a chunk from another document should be rejected");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("does not exist in the active document"));
        assert!(
            observed_requests
                .lock()
                .expect("requests lock should open")
                .is_empty()
        );
    }

    #[test]
    fn translate_chunk_creates_completed_task_run_and_persists_segments() {
        let fixture = create_runtime_fixture();
        seed_translate_chunk_graph(&fixture.runtime);
        let observed_requests = Arc::new(Mutex::new(Vec::new()));
        let observed_task_runs = Arc::new(Mutex::new(Vec::new()));
        let executor = FakeExecutor::success(
            clone_runtime(&fixture),
            successful_action_response(),
            observed_requests.clone(),
            observed_task_runs.clone(),
        );

        let result = translate_chunk_with_runtime_and_executor(
            TranslateChunkInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: CHUNK_ID.to_owned(),
                job_id: Some("job_translate_001".to_owned()),
            },
            &fixture.runtime,
            &executor,
        )
        .expect("translate_chunk should succeed");

        assert_eq!(result.project_id, PROJECT_ID);
        assert_eq!(result.document_id, DOCUMENT_ID);
        assert_eq!(result.chunk_id, CHUNK_ID);
        assert_eq!(result.provider, "fake");
        assert_eq!(result.model, "fake-model");
        assert_eq!(result.task_run.status, TASK_RUN_STATUS_COMPLETED);
        assert_eq!(result.translated_segments.len(), 2);

        let captured_requests = observed_requests
            .lock()
            .expect("requests lock should open");
        assert_eq!(captured_requests.len(), 1);
        assert_eq!(captured_requests[0].chunk_sequence, 1);
        assert_eq!(captured_requests[0].glossary_entries.len(), 1);
        assert_eq!(captured_requests[0].glossary_entries[0].target_term, "Mandato");
        assert_eq!(
            captured_requests[0]
                .style_profile
                .as_ref()
                .expect("style profile should resolve")
                .name,
            "Technical style"
        );
        assert_eq!(captured_requests[0].rules.len(), 1);
        assert_eq!(captured_requests[0].rules[0].name, "Keep honorifics stable");
        assert_eq!(captured_requests[0].accumulated_contexts.len(), 1);
        assert_eq!(
            captured_requests[0].accumulated_contexts[0].context_text,
            "Narration remains formal and procedural."
        );
        assert_eq!(captured_requests[0].segments.len(), 2);
        assert_eq!(captured_requests[0].segments[0].segment_id, "seg_doc_chunk_001_0002");
        drop(captured_requests);

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let segments = SegmentRepository::new(&mut connection)
            .list_by_document(DOCUMENT_ID)
            .expect("segments should load");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_chunk(CHUNK_ID)
            .expect("task runs should load");

        assert_eq!(task_runs.len(), 1);
        assert_eq!(task_runs[0].status, TASK_RUN_STATUS_COMPLETED);
        assert!(task_runs[0].output_payload.is_some());
        assert_eq!(task_runs[0].job_id.as_deref(), Some("job_translate_001"));

        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].status, SEGMENT_STATUS_PENDING_TRANSLATION);
        assert_eq!(segments[0].target_text, None);
        assert_eq!(segments[1].status, SEGMENT_STATUS_TRANSLATED);
        assert_eq!(
            segments[1].target_text.as_deref(),
            Some("Mantenga el mandato activo.")
        );
        assert_eq!(segments[2].status, SEGMENT_STATUS_TRANSLATED);
        assert_eq!(
            segments[2].target_text.as_deref(),
            Some("La escolta protege la puerta.")
        );
        assert_eq!(segments[3].status, SEGMENT_STATUS_PENDING_TRANSLATION);
        assert_eq!(segments[3].target_text, None);
    }

    #[test]
    fn translate_chunk_marks_task_run_failed_on_executor_error() {
        let fixture = create_runtime_fixture();
        seed_translate_chunk_graph(&fixture.runtime);
        let observed_requests = Arc::new(Mutex::new(Vec::new()));
        let observed_task_runs = Arc::new(Mutex::new(Vec::new()));
        let executor = FakeExecutor::failure(
            clone_runtime(&fixture),
            TranslateChunkExecutionFailure {
                message: "The fake executor failed upstream.".to_owned(),
                details: Some("network timeout".to_owned()),
                raw_output: Some("{\"error\":\"timeout\"}".to_owned()),
            },
            observed_requests,
            observed_task_runs,
        );

        let error = translate_chunk_with_runtime_and_executor(
            TranslateChunkInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: CHUNK_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &executor,
        )
        .expect_err("executor failures should surface");

        assert_eq!(error.code, "DESKTOP_COMMAND_FAILED");
        assert!(error.message.contains("failed upstream"));

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_chunk(CHUNK_ID)
            .expect("task runs should load");
        let segments = SegmentRepository::new(&mut connection)
            .list_by_document(DOCUMENT_ID)
            .expect("segments should load");

        assert_eq!(task_runs.len(), 1);
        assert_eq!(task_runs[0].status, TASK_RUN_STATUS_FAILED);
        assert_eq!(
            task_runs[0].error_message.as_deref(),
            Some("The fake executor failed upstream.")
        );
        assert!(task_runs[0]
            .output_payload
            .as_deref()
            .expect("failed run should keep raw output")
            .contains("timeout"));
        assert!(segments.iter().all(|segment| segment.target_text.is_none()));
    }

    #[test]
    fn translate_chunk_marks_task_run_failed_on_invalid_output() {
        let fixture = create_runtime_fixture();
        seed_translate_chunk_graph(&fixture.runtime);
        let observed_requests = Arc::new(Mutex::new(Vec::new()));
        let observed_task_runs = Arc::new(Mutex::new(Vec::new()));
        let executor = FakeExecutor::raw(
            clone_runtime(&fixture),
            "not-json",
            observed_requests,
            observed_task_runs,
        );

        let error = translate_chunk_with_runtime_and_executor(
            TranslateChunkInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: CHUNK_ID.to_owned(),
                job_id: None,
            },
            &fixture.runtime,
            &executor,
        )
        .expect_err("invalid model output should fail validation");

        assert_eq!(error.code, "INVALID_INPUT");
        assert!(error.message.contains("valid JSON"));

        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_chunk(CHUNK_ID)
            .expect("task runs should load");
        let segments = SegmentRepository::new(&mut connection)
            .list_by_document(DOCUMENT_ID)
            .expect("segments should load");

        assert_eq!(task_runs.len(), 1);
        assert_eq!(task_runs[0].status, TASK_RUN_STATUS_FAILED);
        assert!(task_runs[0]
            .output_payload
            .as_deref()
            .expect("failed run should keep raw output")
            .contains("not-json"));
        assert!(segments.iter().all(|segment| segment.target_text.is_none()));
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
            database_path,
            encryption_key_path,
        }
    }

    fn clone_runtime(fixture: &RuntimeFixture) -> DatabaseRuntime {
        DatabaseRuntime::new(
            fixture.database_path.clone(),
            fixture.encryption_key_path.clone(),
        )
    }

    fn successful_action_response() -> TranslateChunkActionResponse {
        TranslateChunkActionResponse {
            translations: vec![
                TranslateChunkTranslation {
                    segment_id: "seg_doc_chunk_001_0002".to_owned(),
                    target_text: "Mantenga el mandato activo.".to_owned(),
                },
                TranslateChunkTranslation {
                    segment_id: "seg_doc_chunk_001_0003".to_owned(),
                    target_text: "La escolta protege la puerta.".to_owned(),
                },
            ],
            notes: Some("Terminology preserved.".to_owned()),
        }
    }

    fn seed_translate_chunk_graph(runtime: &DatabaseRuntime) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_900_000_000_i64;

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: PROJECT_ID.to_owned(),
                name: "Translate project".to_owned(),
                description: None,
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("project should persist");
        ProjectRepository::new(&mut connection)
            .open_project(PROJECT_ID, now + 1)
            .expect("project should become active");

        GlossaryRepository::new(&mut connection)
            .create(&NewGlossary {
                id: "gls_project_001".to_owned(),
                name: "Project glossary".to_owned(),
                description: None,
                project_id: Some(PROJECT_ID.to_owned()),
                status: GLOSSARY_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("glossary should persist");
        GlossaryEntryRepository::new(&mut connection)
            .create(&NewGlossaryEntry {
                id: "gle_project_001".to_owned(),
                glossary_id: "gls_project_001".to_owned(),
                source_term: "Order".to_owned(),
                target_term: "Mandato".to_owned(),
                context_note: Some("Prefer the ritual meaning.".to_owned()),
                status: GLOSSARY_ENTRY_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                source_variants: vec!["Orders".to_owned()],
                target_variants: vec!["Mandatos".to_owned()],
                forbidden_terms: vec!["Orden".to_owned()],
            })
            .expect("glossary entry should persist");

        StyleProfileRepository::new(&mut connection)
            .create(&NewStyleProfile {
                id: "stp_project_001".to_owned(),
                name: "Technical style".to_owned(),
                description: None,
                tone: STYLE_PROFILE_TONE_TECHNICAL.to_owned(),
                formality: STYLE_PROFILE_FORMALITY_FORMAL.to_owned(),
                treatment_preference: STYLE_PROFILE_TREATMENT_USTED.to_owned(),
                consistency_instructions: Some("Keep command verbs stable.".to_owned()),
                editorial_notes: Some("Avoid softening military register.".to_owned()),
                status: STYLE_PROFILE_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("style profile should persist");

        RuleSetRepository::new(&mut connection)
            .create(&NewRuleSet {
                id: "rset_project_001".to_owned(),
                name: "Project rules".to_owned(),
                description: None,
                status: RULE_SET_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("rule set should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_project_translation_001".to_owned(),
                rule_set_id: "rset_project_001".to_owned(),
                action_scope: RULE_ACTION_SCOPE_TRANSLATION.to_owned(),
                rule_type: RULE_TYPE_CONSISTENCY.to_owned(),
                severity: RULE_SEVERITY_HIGH.to_owned(),
                name: "Keep honorifics stable".to_owned(),
                description: Some("Do not alternate honorific choices.".to_owned()),
                guidance: "Use the same honorific strategy across the chunk.".to_owned(),
                is_enabled: true,
                created_at: now,
                updated_at: now,
            })
            .expect("translation rule should persist");
        RuleRepository::new(&mut connection)
            .create(&NewRule {
                id: "rul_project_qa_001".to_owned(),
                rule_set_id: "rset_project_001".to_owned(),
                action_scope: RULE_ACTION_SCOPE_QA.to_owned(),
                rule_type: RULE_TYPE_PREFERENCE.to_owned(),
                severity: RULE_SEVERITY_LOW.to_owned(),
                name: "QA-only reminder".to_owned(),
                description: None,
                guidance: "This rule should not appear in translation.".to_owned(),
                is_enabled: true,
                created_at: now + 1,
                updated_at: now + 1,
            })
            .expect("qa rule should persist");

        ProjectRepository::new(&mut connection)
            .update_editorial_defaults(&ProjectEditorialDefaultsChanges {
                project_id: PROJECT_ID.to_owned(),
                default_glossary_id: Some("gls_project_001".to_owned()),
                default_style_profile_id: Some("stp_project_001".to_owned()),
                default_rule_set_id: Some("rset_project_001".to_owned()),
                updated_at: now + 2,
            })
            .expect("project defaults should update");

        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: DOCUMENT_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
                name: "chapter-one.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "chapter-one.txt".to_owned(),
                file_size_bytes: 128,
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
                        id: "seg_doc_chunk_001_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        source_text: "Order the guard to hold.".to_owned(),
                        source_word_count: 5,
                        source_character_count: 24,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "seg_doc_chunk_001_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        source_text: "Keep the order active.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 22,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "seg_doc_chunk_001_0003".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 3,
                        source_text: "The escort guards the gate.".to_owned(),
                        source_word_count: 5,
                        source_character_count: 27,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "seg_doc_chunk_001_0004".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 4,
                        source_text: "End the watch at dawn.".to_owned(),
                        source_word_count: 5,
                        source_character_count: 22,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                ],
                now + 3,
            )
            .expect("segments should persist");

        DocumentSectionRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[NewDocumentSection {
                    id: "sec_doc_chunk_001_chapter_01".to_owned(),
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
                &[NewTranslationChunk {
                    id: CHUNK_ID.to_owned(),
                    document_id: DOCUMENT_ID.to_owned(),
                    sequence: 1,
                    builder_version: "tr12-builder-v1".to_owned(),
                    strategy: "word_target_approx_60".to_owned(),
                    source_text: "Keep the order active.\nThe escort guards the gate.".to_owned(),
                    context_before_text: Some("Order the guard to hold.".to_owned()),
                    context_after_text: Some("End the watch at dawn.".to_owned()),
                    start_segment_sequence: 2,
                    end_segment_sequence: 3,
                    segment_count: 2,
                    source_word_count: 9,
                    source_character_count: 49,
                    created_at: now,
                    updated_at: now,
                }],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID.to_owned(),
                        segment_id: "seg_doc_chunk_001_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID.to_owned(),
                        segment_id: "seg_doc_chunk_001_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID.to_owned(),
                        segment_id: "seg_doc_chunk_001_0003".to_owned(),
                        segment_sequence: 3,
                        position: 3,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: CHUNK_ID.to_owned(),
                        segment_id: "seg_doc_chunk_001_0004".to_owned(),
                        segment_sequence: 4,
                        position: 4,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                ],
            )
            .expect("translation chunk should persist");

        ChapterContextRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[NewChapterContext {
                    id: "ctx_doc_chunk_001_0001".to_owned(),
                    document_id: DOCUMENT_ID.to_owned(),
                    section_id: None,
                    task_run_id: None,
                    scope_type: CHAPTER_CONTEXT_SCOPE_DOCUMENT.to_owned(),
                    start_segment_sequence: 1,
                    end_segment_sequence: 4,
                    context_text: "Narration remains formal and procedural.".to_owned(),
                    source_summary: Some("Chapter summary".to_owned()),
                    context_word_count: 5,
                    context_character_count: 39,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect("chapter context should persist");

        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: OTHER_DOCUMENT_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
                name: "appendix.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "appendix.txt".to_owned(),
                file_size_bytes: 32,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: now,
                updated_at: now,
            })
            .expect("second document should persist");

        SegmentRepository::new(&mut connection)
            .replace_for_document(
                PROJECT_ID,
                OTHER_DOCUMENT_ID,
                &[NewSegment {
                    id: "seg_doc_other_001_0001".to_owned(),
                    document_id: OTHER_DOCUMENT_ID.to_owned(),
                    sequence: 1,
                    source_text: "Secondary material.".to_owned(),
                    source_word_count: 2,
                    source_character_count: 19,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: now,
                    updated_at: now,
                }],
                now + 4,
            )
            .expect("second-document segments should persist");

        DocumentSectionRepository::new(&mut connection)
            .replace_for_document(
                OTHER_DOCUMENT_ID,
                &[NewDocumentSection {
                    id: "sec_doc_other_001_0001".to_owned(),
                    document_id: OTHER_DOCUMENT_ID.to_owned(),
                    sequence: 1,
                    title: "Appendix".to_owned(),
                    section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                    level: 1,
                    start_segment_sequence: 1,
                    end_segment_sequence: 1,
                    segment_count: 1,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect("second-document sections should persist");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document(
                OTHER_DOCUMENT_ID,
                &[NewTranslationChunk {
                    id: OTHER_CHUNK_ID.to_owned(),
                    document_id: OTHER_DOCUMENT_ID.to_owned(),
                    sequence: 1,
                    builder_version: "tr12-builder-v1".to_owned(),
                    strategy: "word_target_approx_60".to_owned(),
                    source_text: "Secondary material.".to_owned(),
                    context_before_text: None,
                    context_after_text: None,
                    start_segment_sequence: 1,
                    end_segment_sequence: 1,
                    segment_count: 1,
                    source_word_count: 2,
                    source_character_count: 19,
                    created_at: now,
                    updated_at: now,
                }],
                &[NewTranslationChunkSegment {
                    chunk_id: OTHER_CHUNK_ID.to_owned(),
                    segment_id: "seg_doc_other_001_0001".to_owned(),
                    segment_sequence: 1,
                    position: 1,
                    role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                }],
            )
            .expect("second-document chunk should persist");
    }
}
