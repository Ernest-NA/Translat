use rusqlite::{params, Connection};

use crate::persistence::error::PersistenceError;
use crate::qa_findings::{NewQaFinding, QaFindingSummary};

pub struct QaFindingRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> QaFindingRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn upsert(
        &mut self,
        qa_finding: &NewQaFinding,
    ) -> Result<QaFindingSummary, PersistenceError> {
        self.connection
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
                  created_at = excluded.created_at,
                  updated_at = excluded.updated_at
                "#,
                params![
                    qa_finding.id,
                    qa_finding.document_id,
                    qa_finding.chunk_id,
                    qa_finding.task_run_id,
                    qa_finding.job_id,
                    qa_finding.finding_type,
                    qa_finding.severity,
                    qa_finding.status,
                    qa_finding.message,
                    qa_finding.details,
                    qa_finding.created_at,
                    qa_finding.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not persist finding {}.",
                        qa_finding.id
                    ),
                    error,
                )
            })?;

        self.load_by_id(&qa_finding.id)?.ok_or_else(|| {
            PersistenceError::new(format!(
                "The QA-finding repository could not reload finding {} after upsert.",
                qa_finding.id
            ))
        })
    }

    pub fn load_by_id(
        &mut self,
        qa_finding_id: &str,
    ) -> Result<Option<QaFindingSummary>, PersistenceError> {
        self.connection
            .query_row(
                r#"
                SELECT
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
                FROM qa_findings
                WHERE id = ?1
                "#,
                [qa_finding_id],
                |row| {
                    Ok(QaFindingSummary {
                        id: row.get(0)?,
                        document_id: row.get(1)?,
                        chunk_id: row.get(2)?,
                        task_run_id: row.get(3)?,
                        job_id: row.get(4)?,
                        finding_type: row.get(5)?,
                        severity: row.get(6)?,
                        status: row.get(7)?,
                        message: row.get(8)?,
                        details: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                },
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(PersistenceError::with_details(
                    format!("The QA-finding repository could not load finding {qa_finding_id}."),
                    other,
                )),
            })
    }

    pub fn list_by_document(
        &mut self,
        document_id: &str,
    ) -> Result<Vec<QaFindingSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
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
                FROM qa_findings
                WHERE document_id = ?1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not prepare the document listing query for document {document_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([document_id], |row| {
                Ok(QaFindingSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    chunk_id: row.get(2)?,
                    task_run_id: row.get(3)?,
                    job_id: row.get(4)?,
                    finding_type: row.get(5)?,
                    severity: row.get(6)?,
                    status: row.get(7)?,
                    message: row.get(8)?,
                    details: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The QA-finding repository could not read findings for document {document_id}."),
                    error,
                )
            })?;

        let mut qa_findings = Vec::new();

        for row in rows {
            qa_findings.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not decode a finding row for document {document_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(qa_findings)
    }

    pub fn list_by_chunk(
        &mut self,
        chunk_id: &str,
    ) -> Result<Vec<QaFindingSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
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
                FROM qa_findings
                WHERE chunk_id = ?1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not prepare the chunk listing query for chunk {chunk_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([chunk_id], |row| {
                Ok(QaFindingSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    chunk_id: row.get(2)?,
                    task_run_id: row.get(3)?,
                    job_id: row.get(4)?,
                    finding_type: row.get(5)?,
                    severity: row.get(6)?,
                    status: row.get(7)?,
                    message: row.get(8)?,
                    details: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not read findings for chunk {chunk_id}."
                    ),
                    error,
                )
            })?;

        let mut qa_findings = Vec::new();

        for row in rows {
            qa_findings.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not decode a finding row for chunk {chunk_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(qa_findings)
    }

    pub fn list_by_task_run(
        &mut self,
        task_run_id: &str,
    ) -> Result<Vec<QaFindingSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
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
                FROM qa_findings
                WHERE task_run_id = ?1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not prepare the task-run listing query for task run {task_run_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([task_run_id], |row| {
                Ok(QaFindingSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    chunk_id: row.get(2)?,
                    task_run_id: row.get(3)?,
                    job_id: row.get(4)?,
                    finding_type: row.get(5)?,
                    severity: row.get(6)?,
                    status: row.get(7)?,
                    message: row.get(8)?,
                    details: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not read findings for task run {task_run_id}."
                    ),
                    error,
                )
            })?;

        let mut qa_findings = Vec::new();

        for row in rows {
            qa_findings.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not decode a finding row for task run {task_run_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(qa_findings)
    }

    pub fn list_by_job_id(
        &mut self,
        job_id: &str,
    ) -> Result<Vec<QaFindingSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
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
                FROM qa_findings
                WHERE job_id = ?1
                ORDER BY created_at ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not prepare the job listing query for job {job_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([job_id], |row| {
                Ok(QaFindingSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    chunk_id: row.get(2)?,
                    task_run_id: row.get(3)?,
                    job_id: row.get(4)?,
                    finding_type: row.get(5)?,
                    severity: row.get(6)?,
                    status: row.get(7)?,
                    message: row.get(8)?,
                    details: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The QA-finding repository could not read findings for job {job_id}."),
                    error,
                )
            })?;

        let mut qa_findings = Vec::new();

        for row in rows {
            qa_findings.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The QA-finding repository could not decode a finding row for job {job_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(qa_findings)
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::QaFindingRepository;
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::task_runs::TaskRunRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::NewProject;
    use crate::qa_findings::{
        NewQaFinding, QA_FINDING_SEVERITY_HIGH, QA_FINDING_SEVERITY_MEDIUM, QA_FINDING_STATUS_OPEN,
        QA_FINDING_STATUS_RESOLVED,
    };
    use crate::segments::{NewSegment, SEGMENT_STATUS_PENDING_TRANSLATION};
    use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_COMPLETED};
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-tr13";

    #[test]
    fn upsert_and_list_qa_findings_by_document_chunk_task_run_and_job() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_traceability_graph(&mut connection, now);

        let mut repository = QaFindingRepository::new(&mut connection);
        repository
            .upsert(&NewQaFinding {
                id: "qaf_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_chunk_001_chunk_0001".to_owned()),
                task_run_id: Some("trun_001".to_owned()),
                job_id: Some("job_translate_001".to_owned()),
                finding_type: "consistency".to_owned(),
                severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                status: QA_FINDING_STATUS_OPEN.to_owned(),
                message: "Terminology drift detected.".to_owned(),
                details: Some("The title translation differs from the glossary.".to_owned()),
                created_at: now,
                updated_at: now,
            })
            .expect("QA finding should persist");

        repository
            .upsert(&NewQaFinding {
                id: "qaf_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_chunk_001_chunk_0001".to_owned()),
                task_run_id: Some("trun_001".to_owned()),
                job_id: Some("job_translate_001".to_owned()),
                finding_type: "consistency".to_owned(),
                severity: QA_FINDING_SEVERITY_HIGH.to_owned(),
                status: QA_FINDING_STATUS_RESOLVED.to_owned(),
                message: "Terminology drift resolved.".to_owned(),
                details: Some("Updated after reviewer confirmation.".to_owned()),
                created_at: now,
                updated_at: now + 1,
            })
            .expect("QA finding should upsert");

        let document_findings = repository
            .list_by_document("doc_chunk_001")
            .expect("document findings should load");
        let chunk_findings = repository
            .list_by_chunk("doc_chunk_001_chunk_0001")
            .expect("chunk findings should load");
        let task_run_findings = repository
            .list_by_task_run("trun_001")
            .expect("task-run findings should load");
        let job_findings = repository
            .list_by_job_id("job_translate_001")
            .expect("job findings should load");

        assert_eq!(document_findings.len(), 1);
        assert_eq!(chunk_findings.len(), 1);
        assert_eq!(task_run_findings.len(), 1);
        assert_eq!(job_findings.len(), 1);
        assert_eq!(document_findings[0].severity, QA_FINDING_SEVERITY_HIGH);
        assert_eq!(document_findings[0].status, QA_FINDING_STATUS_RESOLVED);
    }

    #[test]
    fn qa_findings_keep_task_run_and_job_traceability_when_chunks_are_deleted() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_traceability_graph(&mut connection, now);

        QaFindingRepository::new(&mut connection)
            .upsert(&NewQaFinding {
                id: "qaf_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_chunk_001_chunk_0001".to_owned()),
                task_run_id: Some("trun_001".to_owned()),
                job_id: Some("job_translate_001".to_owned()),
                finding_type: "consistency".to_owned(),
                severity: QA_FINDING_SEVERITY_MEDIUM.to_owned(),
                status: QA_FINDING_STATUS_OPEN.to_owned(),
                message: "Terminology drift detected.".to_owned(),
                details: None,
                created_at: now,
                updated_at: now,
            })
            .expect("QA finding should persist");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document("doc_chunk_001", &[], &[])
            .expect("chunk replacement should clear the previous chunk");

        let findings = QaFindingRepository::new(&mut connection)
            .list_by_document("doc_chunk_001")
            .expect("findings should reload");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_document("doc_chunk_001")
            .expect("task runs should reload");

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].chunk_id, None);
        assert_eq!(findings[0].task_run_id.as_deref(), Some("trun_001"));
        assert_eq!(findings[0].job_id.as_deref(), Some("job_translate_001"));
        assert_eq!(task_runs.len(), 1);
        assert_eq!(task_runs[0].chunk_id, None);
    }

    fn seed_traceability_graph(connection: &mut Connection, now: i64) {
        ProjectRepository::new(connection)
            .create(&NewProject {
                id: "prj_active_001".to_owned(),
                name: "QA project".to_owned(),
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

        TaskRunRepository::new(connection)
            .create(&NewTaskRun {
                id: "trun_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: Some("doc_chunk_001_chunk_0001".to_owned()),
                job_id: Some("job_translate_001".to_owned()),
                action_type: "translate_chunk".to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: None,
                output_payload: Some("{\"result\":\"ok\"}".to_owned()),
                error_message: None,
                started_at: now,
                completed_at: Some(now + 1),
                created_at: now,
                updated_at: now + 1,
            })
            .expect("task run should persist");
    }
}
