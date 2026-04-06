#![cfg_attr(not(test), allow(dead_code))]

use rusqlite::{params, Connection};

use crate::persistence::error::PersistenceError;
use crate::segments::{SegmentTranslationWrite, SEGMENT_STATUS_TRANSLATED};
use crate::task_runs::{NewTaskRun, TaskRunSummary};

pub struct TaskRunRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> TaskRunRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn create(&mut self, task_run: &NewTaskRun) -> Result<TaskRunSummary, PersistenceError> {
        self.validate_chunk_document(task_run.document_id.as_str(), task_run.chunk_id.as_deref())?;

        self.connection
            .execute(
                r#"
                INSERT INTO task_runs (
                  id,
                  document_id,
                  chunk_id,
                  job_id,
                  action_type,
                  status,
                  input_payload,
                  output_payload,
                  error_message,
                  started_at,
                  completed_at,
                  created_at,
                  updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                "#,
                params![
                    task_run.id,
                    task_run.document_id,
                    task_run.chunk_id,
                    task_run.job_id,
                    task_run.action_type,
                    task_run.status,
                    task_run.input_payload,
                    task_run.output_payload,
                    task_run.error_message,
                    task_run.started_at,
                    task_run.completed_at,
                    task_run.created_at,
                    task_run.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not persist task run {}.",
                        task_run.id
                    ),
                    error,
                )
            })?;

        self.load_by_id(&task_run.id)?.ok_or_else(|| {
            PersistenceError::new(format!(
                "The task-run repository could not reload task run {} after creation.",
                task_run.id
            ))
        })
    }

    pub fn load_by_id(
        &mut self,
        task_run_id: &str,
    ) -> Result<Option<TaskRunSummary>, PersistenceError> {
        self.connection
            .query_row(
                r#"
                SELECT
                  id,
                  document_id,
                  chunk_id,
                  job_id,
                  action_type,
                  status,
                  input_payload,
                  output_payload,
                  error_message,
                  started_at,
                  completed_at,
                  created_at,
                  updated_at
                FROM task_runs
                WHERE id = ?1
                "#,
                [task_run_id],
                |row| {
                    Ok(TaskRunSummary {
                        id: row.get(0)?,
                        document_id: row.get(1)?,
                        chunk_id: row.get(2)?,
                        job_id: row.get(3)?,
                        action_type: row.get(4)?,
                        status: row.get(5)?,
                        input_payload: row.get(6)?,
                        output_payload: row.get(7)?,
                        error_message: row.get(8)?,
                        started_at: row.get(9)?,
                        completed_at: row.get(10)?,
                        created_at: row.get(11)?,
                        updated_at: row.get(12)?,
                    })
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(PersistenceError::with_details(
                    format!("The task-run repository could not load task run {task_run_id}."),
                    other,
                )),
            })
    }

    pub fn list_by_document(
        &mut self,
        document_id: &str,
    ) -> Result<Vec<TaskRunSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  chunk_id,
                  job_id,
                  action_type,
                  status,
                  input_payload,
                  output_payload,
                  error_message,
                  started_at,
                  completed_at,
                  created_at,
                  updated_at
                FROM task_runs
                WHERE document_id = ?1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not prepare the document listing query for document {document_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([document_id], |row| {
                Ok(TaskRunSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    chunk_id: row.get(2)?,
                    job_id: row.get(3)?,
                    action_type: row.get(4)?,
                    status: row.get(5)?,
                    input_payload: row.get(6)?,
                    output_payload: row.get(7)?,
                    error_message: row.get(8)?,
                    started_at: row.get(9)?,
                    completed_at: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The task-run repository could not read task runs for document {document_id}."),
                    error,
                )
            })?;

        let mut task_runs = Vec::new();

        for row in rows {
            task_runs.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not decode a task-run row for document {document_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(task_runs)
    }

    pub fn list_by_chunk(
        &mut self,
        chunk_id: &str,
    ) -> Result<Vec<TaskRunSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  chunk_id,
                  job_id,
                  action_type,
                  status,
                  input_payload,
                  output_payload,
                  error_message,
                  started_at,
                  completed_at,
                  created_at,
                  updated_at
                FROM task_runs
                WHERE chunk_id = ?1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not prepare the chunk listing query for chunk {chunk_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([chunk_id], |row| {
                Ok(TaskRunSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    chunk_id: row.get(2)?,
                    job_id: row.get(3)?,
                    action_type: row.get(4)?,
                    status: row.get(5)?,
                    input_payload: row.get(6)?,
                    output_payload: row.get(7)?,
                    error_message: row.get(8)?,
                    started_at: row.get(9)?,
                    completed_at: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not read task runs for chunk {chunk_id}."
                    ),
                    error,
                )
            })?;

        let mut task_runs = Vec::new();

        for row in rows {
            task_runs.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not decode a task-run row for chunk {chunk_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(task_runs)
    }

    pub fn list_by_job_id(
        &mut self,
        job_id: &str,
    ) -> Result<Vec<TaskRunSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  chunk_id,
                  job_id,
                  action_type,
                  status,
                  input_payload,
                  output_payload,
                  error_message,
                  started_at,
                  completed_at,
                  created_at,
                  updated_at
                FROM task_runs
                WHERE job_id = ?1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not prepare the job listing query for job {job_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([job_id], |row| {
                Ok(TaskRunSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    chunk_id: row.get(2)?,
                    job_id: row.get(3)?,
                    action_type: row.get(4)?,
                    status: row.get(5)?,
                    input_payload: row.get(6)?,
                    output_payload: row.get(7)?,
                    error_message: row.get(8)?,
                    started_at: row.get(9)?,
                    completed_at: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The task-run repository could not read task runs for job {job_id}."),
                    error,
                )
            })?;

        let mut task_runs = Vec::new();

        for row in rows {
            task_runs.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not decode a task-run row for job {job_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(task_runs)
    }

    pub fn mark_completed_with_translation_projection(
        &mut self,
        project_id: &str,
        document_id: &str,
        task_run_id: &str,
        output_payload: &str,
        segment_translations: &[SegmentTranslationWrite],
        completed_at: i64,
    ) -> Result<TaskRunSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The task-run repository could not start the completion transaction for task run {task_run_id}."
                ),
                error,
            )
        })?;

        for segment_translation in segment_translations {
            let updated_rows = transaction
                .execute(
                    r#"
                    UPDATE segments
                    SET
                      target_text = ?3,
                      status = ?4,
                      last_task_run_id = ?5,
                      updated_at = ?6
                    WHERE id = ?1 AND document_id = ?2
                    "#,
                    params![
                        segment_translation.segment_id,
                        document_id,
                        segment_translation.target_text,
                        SEGMENT_STATUS_TRANSLATED,
                        task_run_id,
                        completed_at
                    ],
                )
                .map_err(|error| {
                    PersistenceError::with_details(
                        format!(
                            "The task-run repository could not project translation output onto segment {} while finalizing task run {task_run_id}.",
                            segment_translation.segment_id
                        ),
                        error,
                    )
                })?;

            if updated_rows != 1 {
                return Err(PersistenceError::new(format!(
                    "The task-run repository could not find segment {} in document {document_id} while finalizing task run {task_run_id}.",
                    segment_translation.segment_id
                )));
            }
        }

        transaction
            .execute(
                "UPDATE documents SET updated_at = ?2 WHERE id = ?1",
                params![document_id, completed_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not update document {document_id} while finalizing task run {task_run_id}."
                    ),
                    error,
                )
            })?;

        transaction
            .execute(
                "UPDATE projects SET updated_at = ?2 WHERE id = ?1",
                params![project_id, completed_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not update project {project_id} while finalizing task run {task_run_id}."
                    ),
                    error,
                )
            })?;

        let updated_rows = transaction
            .execute(
                r#"
                UPDATE task_runs
                SET
                  status = ?2,
                  output_payload = ?3,
                  error_message = NULL,
                  completed_at = ?4,
                  updated_at = ?4
                WHERE id = ?1
                "#,
                params![
                    task_run_id,
                    crate::task_runs::TASK_RUN_STATUS_COMPLETED,
                    output_payload,
                    completed_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not finalize task run {task_run_id}."
                    ),
                    error,
                )
            })?;

        if updated_rows != 1 {
            return Err(PersistenceError::new(format!(
                "The task-run repository could not find task run {task_run_id} while finalizing translation projection."
            )));
        }

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The task-run repository could not commit the completion transaction for task run {task_run_id}."
                ),
                error,
            )
        })?;

        self.load_by_id(task_run_id)?.ok_or_else(|| {
            PersistenceError::new(format!(
                "The task-run repository could not reload task run {task_run_id} after translation finalization."
            ))
        })
    }

    pub fn mark_failed(
        &mut self,
        task_run_id: &str,
        error_message: &str,
        output_payload: Option<&str>,
        completed_at: i64,
    ) -> Result<TaskRunSummary, PersistenceError> {
        self.update_terminal_state(
            task_run_id,
            crate::task_runs::TASK_RUN_STATUS_FAILED,
            output_payload,
            Some(error_message),
            completed_at,
        )
    }

    fn validate_chunk_document(
        &self,
        document_id: &str,
        chunk_id: Option<&str>,
    ) -> Result<(), PersistenceError> {
        let Some(chunk_id) = chunk_id else {
            return Ok(());
        };

        let chunk_document_id = self
            .connection
            .query_row(
                "SELECT document_id FROM translation_chunks WHERE id = ?1",
                [chunk_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => PersistenceError::new(format!(
                    "The task-run repository could not find chunk {chunk_id} for document {document_id}."
                )),
                other => PersistenceError::with_details(
                    format!(
                        "The task-run repository could not validate chunk {chunk_id} for document {document_id}."
                    ),
                    other,
                ),
            })?;

        if chunk_document_id != document_id {
            return Err(PersistenceError::new(format!(
                "The task-run repository received chunk {chunk_id} for document {chunk_document_id}, but the task run targets document {document_id}."
            )));
        }

        Ok(())
    }

    fn update_terminal_state(
        &mut self,
        task_run_id: &str,
        status: &str,
        output_payload: Option<&str>,
        error_message: Option<&str>,
        completed_at: i64,
    ) -> Result<TaskRunSummary, PersistenceError> {
        let updated_rows = self
            .connection
            .execute(
                r#"
                UPDATE task_runs
                SET
                  status = ?2,
                  output_payload = ?3,
                  error_message = ?4,
                  completed_at = ?5,
                  updated_at = ?5
                WHERE id = ?1
                "#,
                params![
                    task_run_id,
                    status,
                    output_payload,
                    error_message,
                    completed_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The task-run repository could not update task run {task_run_id}."
                    ),
                    error,
                )
            })?;

        if updated_rows != 1 {
            return Err(PersistenceError::new(format!(
                "The task-run repository could not find task run {task_run_id} while updating its terminal state."
            )));
        }

        self.load_by_id(task_run_id)?.ok_or_else(|| {
            PersistenceError::new(format!(
                "The task-run repository could not reload task run {task_run_id} after its terminal update."
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::TaskRunRepository;
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::NewProject;
    use crate::segments::{
        NewSegment, SegmentTranslationWrite, SEGMENT_STATUS_PENDING_TRANSLATION,
        SEGMENT_STATUS_TRANSLATED,
    };
    use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_RUNNING};
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-tr13";

    #[test]
    fn create_and_list_task_runs_by_document_chunk_and_job() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_chunked_document(&mut connection, now);

        let mut repository = TaskRunRepository::new(&mut connection);
        repository
            .create(&NewTaskRun {
                id: "trun_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_chunk_001_chunk_0001".to_owned()),
                job_id: Some("job_translate_001".to_owned()),
                action_type: "translate_chunk".to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunkId\":\"doc_chunk_001_chunk_0001\"}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: now,
                completed_at: None,
                created_at: now,
                updated_at: now,
            })
            .expect("task run should persist");

        let document_runs = repository
            .list_by_document("doc_chunk_001")
            .expect("document runs should load");
        let chunk_runs = repository
            .list_by_chunk("doc_chunk_001_chunk_0001")
            .expect("chunk runs should load");
        let job_runs = repository
            .list_by_job_id("job_translate_001")
            .expect("job runs should load");

        assert_eq!(document_runs.len(), 1);
        assert_eq!(chunk_runs.len(), 1);
        assert_eq!(job_runs.len(), 1);
        assert_eq!(document_runs[0].action_type, "translate_chunk");
        assert_eq!(
            document_runs[0].chunk_id.as_deref(),
            Some("doc_chunk_001_chunk_0001")
        );
    }

    #[test]
    fn task_runs_keep_job_traceability_when_chunks_are_rebuilt() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_chunked_document(&mut connection, now);

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "trun_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_chunk_001_chunk_0001".to_owned()),
                job_id: Some("job_translate_001".to_owned()),
                action_type: "translate_chunk".to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: None,
                output_payload: None,
                error_message: None,
                started_at: now,
                completed_at: None,
                created_at: now,
                updated_at: now,
            })
            .expect("task run should persist");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document("doc_chunk_001", &[], &[])
            .expect("chunk replacement should clear the previous chunk");

        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_document("doc_chunk_001")
            .expect("task runs should reload");

        assert_eq!(task_runs.len(), 1);
        assert_eq!(task_runs[0].job_id.as_deref(), Some("job_translate_001"));
        assert_eq!(task_runs[0].chunk_id, None);
    }

    #[test]
    fn create_rejects_chunks_from_other_documents() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_chunked_document(&mut connection, now);

        let error = TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "trun_bad_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_other_001_chunk_0001".to_owned()),
                job_id: Some("job_translate_002".to_owned()),
                action_type: "translate_chunk".to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: None,
                output_payload: None,
                error_message: None,
                started_at: now,
                completed_at: None,
                created_at: now,
                updated_at: now,
            })
            .expect_err("cross-document chunks should be rejected");

        assert!(error
            .to_string()
            .contains("but the task run targets document doc_chunk_001"));
    }

    #[test]
    fn complete_with_translation_projection_updates_segments_project_and_task_run() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;
        let completed_at = now + 300;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_chunked_document(&mut connection, now);

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "trun_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_chunk_001_chunk_0001".to_owned()),
                job_id: Some("job_translate_001".to_owned()),
                action_type: "translate_chunk".to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunkId\":\"doc_chunk_001_chunk_0001\"}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: now,
                completed_at: None,
                created_at: now,
                updated_at: now,
            })
            .expect("task run should persist");

        let completed_task_run = TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                "prj_active_001",
                "doc_chunk_001",
                "trun_001",
                "{\"translations\":[]}",
                &[SegmentTranslationWrite {
                    segment_id: "doc_chunk_001_seg_0001".to_owned(),
                    target_text: "Segmento uno".to_owned(),
                }],
                completed_at,
            )
            .expect("completion should commit atomically");

        assert_eq!(
            completed_task_run.status,
            crate::task_runs::TASK_RUN_STATUS_COMPLETED
        );
        assert_eq!(completed_task_run.completed_at, Some(completed_at));

        let segments = SegmentRepository::new(&mut connection)
            .list_by_document("doc_chunk_001")
            .expect("segments should load");
        assert_eq!(segments[0].status, SEGMENT_STATUS_TRANSLATED);
        assert_eq!(segments[0].target_text.as_deref(), Some("Segmento uno"));

        let project_updated_at = connection
            .query_row(
                "SELECT updated_at FROM projects WHERE id = ?1",
                ["prj_active_001"],
                |row| row.get::<_, i64>(0),
            )
            .expect("project timestamp should load");
        assert_eq!(project_updated_at, completed_at);
    }

    #[test]
    fn completion_with_invalid_segment_rolls_back_task_run_finalization() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;
        let completed_at = now + 300;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_chunked_document(&mut connection, now);

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "trun_rollback_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_chunk_001_chunk_0001".to_owned()),
                job_id: None,
                action_type: "translate_chunk".to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: None,
                output_payload: None,
                error_message: None,
                started_at: now,
                completed_at: None,
                created_at: now,
                updated_at: now,
            })
            .expect("task run should persist");

        let error = TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                "prj_active_001",
                "doc_chunk_001",
                "trun_rollback_001",
                "{\"translations\":[]}",
                &[SegmentTranslationWrite {
                    segment_id: "seg_missing_9999".to_owned(),
                    target_text: "No debe persistir".to_owned(),
                }],
                completed_at,
            )
            .expect_err("invalid projection should abort the transaction");

        assert!(error.to_string().contains("seg_missing_9999"));

        let task_run = TaskRunRepository::new(&mut connection)
            .load_by_id("trun_rollback_001")
            .expect("task run should reload")
            .expect("task run should exist");
        assert_eq!(task_run.status, TASK_RUN_STATUS_RUNNING);
        assert_eq!(task_run.completed_at, None);
        assert_eq!(task_run.output_payload, None);

        let segments = SegmentRepository::new(&mut connection)
            .list_by_document("doc_chunk_001")
            .expect("segments should load");
        assert!(segments.iter().all(|segment| segment.target_text.is_none()));
    }

    fn seed_chunked_document(connection: &mut Connection, now: i64) {
        ProjectRepository::new(connection)
            .create(&NewProject {
                id: "prj_active_001".to_owned(),
                name: "Chunk project".to_owned(),
                description: None,
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("project should persist");

        crate::persistence::documents::DocumentRepository::new(connection)
            .create(&NewDocument {
                id: "doc_chunk_001".to_owned(),
                project_id: "prj_active_001".to_owned(),
                name: "chunked.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "ignored".to_owned(),
                file_size_bytes: 120,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: now,
                updated_at: now,
            })
            .expect("document should persist");

        crate::persistence::documents::DocumentRepository::new(connection)
            .create(&NewDocument {
                id: "doc_other_001".to_owned(),
                project_id: "prj_active_001".to_owned(),
                name: "other.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "ignored_other".to_owned(),
                file_size_bytes: 80,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: now,
                updated_at: now,
            })
            .expect("other document should persist");

        SegmentRepository::new(connection)
            .replace_for_document(
                "prj_active_001",
                "doc_chunk_001",
                &[NewSegment {
                    id: "doc_chunk_001_seg_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    source_text: "One.".to_owned(),
                    source_word_count: 1,
                    source_character_count: 4,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: now,
                    updated_at: now,
                }],
                now,
            )
            .expect("segments should persist");

        SegmentRepository::new(connection)
            .replace_for_document(
                "prj_active_001",
                "doc_other_001",
                &[NewSegment {
                    id: "doc_other_001_seg_0001".to_owned(),
                    document_id: "doc_other_001".to_owned(),
                    sequence: 1,
                    source_text: "Other.".to_owned(),
                    source_word_count: 1,
                    source_character_count: 6,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: now,
                    updated_at: now,
                }],
                now,
            )
            .expect("other segments should persist");

        TranslationChunkRepository::new(connection)
            .replace_for_document(
                "doc_chunk_001",
                &[NewTranslationChunk {
                    id: "doc_chunk_001_chunk_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    builder_version: "tr12-basic-v1".to_owned(),
                    strategy: "section-aware-fixed-word-target-v1".to_owned(),
                    source_text: "One.".to_owned(),
                    context_before_text: None,
                    context_after_text: None,
                    start_segment_sequence: 1,
                    end_segment_sequence: 1,
                    segment_count: 1,
                    source_word_count: 1,
                    source_character_count: 4,
                    created_at: now,
                    updated_at: now,
                }],
                &[NewTranslationChunkSegment {
                    chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                    segment_id: "doc_chunk_001_seg_0001".to_owned(),
                    segment_sequence: 1,
                    position: 1,
                    role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                }],
            )
            .expect("chunk should persist");

        TranslationChunkRepository::new(connection)
            .replace_for_document(
                "doc_other_001",
                &[NewTranslationChunk {
                    id: "doc_other_001_chunk_0001".to_owned(),
                    document_id: "doc_other_001".to_owned(),
                    sequence: 1,
                    builder_version: "tr12-basic-v1".to_owned(),
                    strategy: "section-aware-fixed-word-target-v1".to_owned(),
                    source_text: "Other.".to_owned(),
                    context_before_text: None,
                    context_after_text: None,
                    start_segment_sequence: 1,
                    end_segment_sequence: 1,
                    segment_count: 1,
                    source_word_count: 1,
                    source_character_count: 6,
                    created_at: now,
                    updated_at: now,
                }],
                &[NewTranslationChunkSegment {
                    chunk_id: "doc_other_001_chunk_0001".to_owned(),
                    segment_id: "doc_other_001_seg_0001".to_owned(),
                    segment_sequence: 1,
                    position: 1,
                    role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                }],
            )
            .expect("other chunk should persist");
    }
}
