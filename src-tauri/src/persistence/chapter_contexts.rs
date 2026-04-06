#![cfg_attr(not(test), allow(dead_code))]

use rusqlite::{params, Connection};

use crate::chapter_contexts::{ChapterContextSummary, NewChapterContext};
use crate::persistence::error::PersistenceError;

pub struct ChapterContextRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> ChapterContextRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn replace_for_document(
        &mut self,
        document_id: &str,
        chapter_contexts: &[NewChapterContext],
    ) -> Result<(), PersistenceError> {
        for chapter_context in chapter_contexts {
            self.validate_context_document_alignment(document_id, chapter_context)?;
        }

        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The chapter-context repository could not start the replacement transaction for document {document_id}."
                ),
                error,
            )
        })?;

        transaction
            .execute("DELETE FROM chapter_contexts WHERE document_id = ?1", [document_id])
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not clear previous contexts for document {document_id}."
                    ),
                    error,
                )
            })?;

        for chapter_context in chapter_contexts {
            transaction
                .execute(
                    r#"
                    INSERT INTO chapter_contexts (
                      id,
                      document_id,
                      section_id,
                      task_run_id,
                      scope_type,
                      start_segment_sequence,
                      end_segment_sequence,
                      context_text,
                      source_summary,
                      context_word_count,
                      context_character_count,
                      created_at,
                      updated_at
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                    "#,
                    params![
                        chapter_context.id,
                        document_id,
                        chapter_context.section_id,
                        chapter_context.task_run_id,
                        chapter_context.scope_type,
                        chapter_context.start_segment_sequence,
                        chapter_context.end_segment_sequence,
                        chapter_context.context_text,
                        chapter_context.source_summary,
                        chapter_context.context_word_count,
                        chapter_context.context_character_count,
                        chapter_context.created_at,
                        chapter_context.updated_at
                    ],
                )
                .map_err(|error| {
                    PersistenceError::with_details(
                        format!(
                            "The chapter-context repository could not persist context {} for document {document_id}.",
                            chapter_context.id
                        ),
                        error,
                    )
                })?;
        }

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The chapter-context repository could not commit context replacement for document {document_id}."
                ),
                error,
            )
        })?;

        Ok(())
    }

    pub fn list_by_document(
        &mut self,
        document_id: &str,
    ) -> Result<Vec<ChapterContextSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  section_id,
                  task_run_id,
                  scope_type,
                  start_segment_sequence,
                  end_segment_sequence,
                  context_text,
                  source_summary,
                  context_word_count,
                  context_character_count,
                  created_at,
                  updated_at
                FROM chapter_contexts
                WHERE document_id = ?1
                ORDER BY start_segment_sequence ASC, end_segment_sequence ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not prepare the document listing query for document {document_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([document_id], |row| {
                Ok(ChapterContextSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    section_id: row.get(2)?,
                    task_run_id: row.get(3)?,
                    scope_type: row.get(4)?,
                    start_segment_sequence: row.get(5)?,
                    end_segment_sequence: row.get(6)?,
                    context_text: row.get(7)?,
                    source_summary: row.get(8)?,
                    context_word_count: row.get(9)?,
                    context_character_count: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not read contexts for document {document_id}."
                    ),
                    error,
                )
            })?;

        let mut chapter_contexts = Vec::new();

        for row in rows {
            chapter_contexts.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not decode a context row for document {document_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(chapter_contexts)
    }

    pub fn list_by_section(
        &mut self,
        section_id: &str,
    ) -> Result<Vec<ChapterContextSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  section_id,
                  task_run_id,
                  scope_type,
                  start_segment_sequence,
                  end_segment_sequence,
                  context_text,
                  source_summary,
                  context_word_count,
                  context_character_count,
                  created_at,
                  updated_at
                FROM chapter_contexts
                WHERE section_id = ?1
                ORDER BY start_segment_sequence ASC, end_segment_sequence ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not prepare the section listing query for section {section_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([section_id], |row| {
                Ok(ChapterContextSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    section_id: row.get(2)?,
                    task_run_id: row.get(3)?,
                    scope_type: row.get(4)?,
                    start_segment_sequence: row.get(5)?,
                    end_segment_sequence: row.get(6)?,
                    context_text: row.get(7)?,
                    source_summary: row.get(8)?,
                    context_word_count: row.get(9)?,
                    context_character_count: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not read contexts for section {section_id}."
                    ),
                    error,
                )
            })?;

        let mut chapter_contexts = Vec::new();

        for row in rows {
            chapter_contexts.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not decode a context row for section {section_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(chapter_contexts)
    }

    pub fn list_by_task_run(
        &mut self,
        task_run_id: &str,
    ) -> Result<Vec<ChapterContextSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  section_id,
                  task_run_id,
                  scope_type,
                  start_segment_sequence,
                  end_segment_sequence,
                  context_text,
                  source_summary,
                  context_word_count,
                  context_character_count,
                  created_at,
                  updated_at
                FROM chapter_contexts
                WHERE task_run_id = ?1
                ORDER BY start_segment_sequence ASC, end_segment_sequence ASC, id ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not prepare the task-run listing query for task run {task_run_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([task_run_id], |row| {
                Ok(ChapterContextSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    section_id: row.get(2)?,
                    task_run_id: row.get(3)?,
                    scope_type: row.get(4)?,
                    start_segment_sequence: row.get(5)?,
                    end_segment_sequence: row.get(6)?,
                    context_text: row.get(7)?,
                    source_summary: row.get(8)?,
                    context_word_count: row.get(9)?,
                    context_character_count: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not read contexts for task run {task_run_id}."
                    ),
                    error,
                )
            })?;

        let mut chapter_contexts = Vec::new();

        for row in rows {
            chapter_contexts.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not decode a context row for task run {task_run_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(chapter_contexts)
    }

    fn validate_context_document_alignment(
        &self,
        document_id: &str,
        chapter_context: &NewChapterContext,
    ) -> Result<(), PersistenceError> {
        if chapter_context.document_id != document_id {
            return Err(PersistenceError::new(format!(
                "The chapter-context repository received context {} for document {}, but replacement was requested for document {}.",
                chapter_context.id, chapter_context.document_id, document_id
            )));
        }

        self.validate_linked_document(
            "section",
            chapter_context.id.as_str(),
            document_id,
            chapter_context.section_id.as_deref(),
            "SELECT document_id FROM document_sections WHERE id = ?1",
        )?;
        self.validate_linked_document(
            "task run",
            chapter_context.id.as_str(),
            document_id,
            chapter_context.task_run_id.as_deref(),
            "SELECT document_id FROM task_runs WHERE id = ?1",
        )?;

        Ok(())
    }

    fn validate_linked_document(
        &self,
        linked_entity: &str,
        context_id: &str,
        document_id: &str,
        linked_id: Option<&str>,
        query: &str,
    ) -> Result<(), PersistenceError> {
        let Some(linked_id) = linked_id else {
            return Ok(());
        };

        let linked_document_id = self
            .connection
            .query_row(query, [linked_id], |row| row.get::<_, String>(0))
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => PersistenceError::new(format!(
                    "The chapter-context repository could not find {linked_entity} {linked_id} for context {context_id}."
                )),
                other => PersistenceError::with_details(
                    format!(
                        "The chapter-context repository could not validate {linked_entity} {linked_id} for context {context_id}."
                    ),
                    other,
                ),
            })?;

        if linked_document_id != document_id {
            return Err(PersistenceError::new(format!(
                "The chapter-context repository received {linked_entity} {linked_id} for document {linked_document_id}, but context {context_id} targets document {document_id}."
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::ChapterContextRepository;
    use crate::chapter_contexts::{
        NewChapterContext, CHAPTER_CONTEXT_SCOPE_CHAPTER, CHAPTER_CONTEXT_SCOPE_DOCUMENT,
    };
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::sections::DocumentSectionRepository;
    use crate::persistence::task_runs::TaskRunRepository;
    use crate::projects::NewProject;
    use crate::sections::{NewDocumentSection, DOCUMENT_SECTION_TYPE_CHAPTER};
    use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_COMPLETED};

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-tr13";

    #[test]
    fn replace_and_list_chapter_contexts_by_document_section_and_task_run() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_document_context_graph(&mut connection, now);

        ChapterContextRepository::new(&mut connection)
            .replace_for_document(
                "doc_chunk_001",
                &[
                    NewChapterContext {
                        id: "ctx_001".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        section_id: Some("doc_chunk_001_sec_0001".to_owned()),
                        task_run_id: Some("trun_001".to_owned()),
                        scope_type: CHAPTER_CONTEXT_SCOPE_CHAPTER.to_owned(),
                        start_segment_sequence: 1,
                        end_segment_sequence: 2,
                        context_text: "Opening chapter context.".to_owned(),
                        source_summary: Some("Derived from the chapter lead.".to_owned()),
                        context_word_count: 3,
                        context_character_count: 25,
                        created_at: now,
                        updated_at: now,
                    },
                    NewChapterContext {
                        id: "ctx_002".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        section_id: None,
                        task_run_id: None,
                        scope_type: CHAPTER_CONTEXT_SCOPE_DOCUMENT.to_owned(),
                        start_segment_sequence: 1,
                        end_segment_sequence: 4,
                        context_text: "Document-wide tone memory.".to_owned(),
                        source_summary: None,
                        context_word_count: 3,
                        context_character_count: 26,
                        created_at: now,
                        updated_at: now,
                    },
                ],
            )
            .expect("chapter contexts should persist");

        let document_contexts = ChapterContextRepository::new(&mut connection)
            .list_by_document("doc_chunk_001")
            .expect("document contexts should load");
        let section_contexts = ChapterContextRepository::new(&mut connection)
            .list_by_section("doc_chunk_001_sec_0001")
            .expect("section contexts should load");
        let task_run_contexts = ChapterContextRepository::new(&mut connection)
            .list_by_task_run("trun_001")
            .expect("task-run contexts should load");

        assert_eq!(document_contexts.len(), 2);
        assert_eq!(section_contexts.len(), 1);
        assert_eq!(task_run_contexts.len(), 1);
        assert_eq!(
            section_contexts[0].scope_type,
            CHAPTER_CONTEXT_SCOPE_CHAPTER
        );
        assert_eq!(
            task_run_contexts[0].task_run_id.as_deref(),
            Some("trun_001")
        );
    }

    #[test]
    fn replace_for_document_removes_previous_chapter_contexts() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_document_context_graph(&mut connection, now);

        let mut repository = ChapterContextRepository::new(&mut connection);
        repository
            .replace_for_document(
                "doc_chunk_001",
                &[NewChapterContext {
                    id: "ctx_001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    section_id: Some("doc_chunk_001_sec_0001".to_owned()),
                    task_run_id: None,
                    scope_type: CHAPTER_CONTEXT_SCOPE_CHAPTER.to_owned(),
                    start_segment_sequence: 1,
                    end_segment_sequence: 2,
                    context_text: "First context.".to_owned(),
                    source_summary: None,
                    context_word_count: 2,
                    context_character_count: 14,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect("initial contexts should persist");
        repository
            .replace_for_document(
                "doc_chunk_001",
                &[NewChapterContext {
                    id: "ctx_002".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    section_id: None,
                    task_run_id: None,
                    scope_type: CHAPTER_CONTEXT_SCOPE_DOCUMENT.to_owned(),
                    start_segment_sequence: 1,
                    end_segment_sequence: 4,
                    context_text: "Replacement context.".to_owned(),
                    source_summary: None,
                    context_word_count: 2,
                    context_character_count: 20,
                    created_at: now + 1,
                    updated_at: now + 1,
                }],
            )
            .expect("replacement contexts should persist");

        let document_contexts = repository
            .list_by_document("doc_chunk_001")
            .expect("contexts should reload");

        assert_eq!(document_contexts.len(), 1);
        assert_eq!(document_contexts[0].id, "ctx_002");
    }

    #[test]
    fn replace_for_document_rejects_cross_document_context_rows() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_document_context_graph(&mut connection, now);

        let error = ChapterContextRepository::new(&mut connection)
            .replace_for_document(
                "doc_chunk_001",
                &[NewChapterContext {
                    id: "ctx_bad_001".to_owned(),
                    document_id: "doc_other_001".to_owned(),
                    section_id: None,
                    task_run_id: None,
                    scope_type: CHAPTER_CONTEXT_SCOPE_DOCUMENT.to_owned(),
                    start_segment_sequence: 1,
                    end_segment_sequence: 4,
                    context_text: "Mismatched document context.".to_owned(),
                    source_summary: None,
                    context_word_count: 3,
                    context_character_count: 27,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect_err("cross-document contexts should be rejected");

        assert!(error
            .to_string()
            .contains("replacement was requested for document doc_chunk_001"));

        let document_contexts = ChapterContextRepository::new(&mut connection)
            .list_by_document("doc_chunk_001")
            .expect("contexts should remain empty");

        assert!(document_contexts.is_empty());
    }

    #[test]
    fn replace_for_document_rejects_cross_document_section_links() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_document_context_graph(&mut connection, now);

        let error = ChapterContextRepository::new(&mut connection)
            .replace_for_document(
                "doc_chunk_001",
                &[NewChapterContext {
                    id: "ctx_bad_002".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    section_id: Some("doc_other_001_sec_0001".to_owned()),
                    task_run_id: None,
                    scope_type: CHAPTER_CONTEXT_SCOPE_CHAPTER.to_owned(),
                    start_segment_sequence: 1,
                    end_segment_sequence: 2,
                    context_text: "Mismatched section context.".to_owned(),
                    source_summary: None,
                    context_word_count: 3,
                    context_character_count: 27,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect_err("cross-document section links should be rejected");

        assert!(error
            .to_string()
            .contains("but context ctx_bad_002 targets document doc_chunk_001"));
    }

    #[test]
    fn replace_for_document_rejects_cross_document_task_run_links() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        seed_document_context_graph(&mut connection, now);

        let error = ChapterContextRepository::new(&mut connection)
            .replace_for_document(
                "doc_chunk_001",
                &[NewChapterContext {
                    id: "ctx_bad_003".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    section_id: None,
                    task_run_id: Some("trun_other_001".to_owned()),
                    scope_type: CHAPTER_CONTEXT_SCOPE_CHAPTER.to_owned(),
                    start_segment_sequence: 1,
                    end_segment_sequence: 2,
                    context_text: "Mismatched run context.".to_owned(),
                    source_summary: None,
                    context_word_count: 3,
                    context_character_count: 22,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect_err("cross-document task-run links should be rejected");

        assert!(error
            .to_string()
            .contains("but context ctx_bad_003 targets document doc_chunk_001"));
    }

    fn seed_document_context_graph(connection: &mut Connection, now: i64) {
        ProjectRepository::new(connection)
            .create(&NewProject {
                id: "prj_active_001".to_owned(),
                name: "Context project".to_owned(),
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
                name: "chaptered.txt".to_owned(),
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
                file_size_bytes: 64,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: now,
                updated_at: now,
            })
            .expect("other document should persist");

        DocumentSectionRepository::new(connection)
            .replace_for_document(
                "doc_chunk_001",
                &[NewDocumentSection {
                    id: "doc_chunk_001_sec_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    title: "Chapter 1".to_owned(),
                    section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                    level: 1,
                    start_segment_sequence: 1,
                    end_segment_sequence: 2,
                    segment_count: 2,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect("section should persist");

        DocumentSectionRepository::new(connection)
            .replace_for_document(
                "doc_other_001",
                &[NewDocumentSection {
                    id: "doc_other_001_sec_0001".to_owned(),
                    document_id: "doc_other_001".to_owned(),
                    sequence: 1,
                    title: "Chapter X".to_owned(),
                    section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                    level: 1,
                    start_segment_sequence: 1,
                    end_segment_sequence: 1,
                    segment_count: 1,
                    created_at: now,
                    updated_at: now,
                }],
            )
            .expect("other section should persist");

        TaskRunRepository::new(connection)
            .create(&NewTaskRun {
                id: "trun_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                chunk_id: None,
                job_id: Some("job_context_001".to_owned()),
                action_type: "build_context".to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: None,
                output_payload: Some("{\"sectionId\":\"doc_chunk_001_sec_0001\"}".to_owned()),
                error_message: None,
                started_at: now,
                completed_at: Some(now + 1),
                created_at: now,
                updated_at: now + 1,
            })
            .expect("task run should persist");

        TaskRunRepository::new(connection)
            .create(&NewTaskRun {
                id: "trun_other_001".to_owned(),
                document_id: "doc_other_001".to_owned(),
                chunk_id: None,
                job_id: Some("job_context_002".to_owned()),
                action_type: "build_context".to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: None,
                output_payload: Some("{\"sectionId\":\"doc_other_001_sec_0001\"}".to_owned()),
                error_message: None,
                started_at: now,
                completed_at: Some(now + 1),
                created_at: now,
                updated_at: now + 1,
            })
            .expect("other task run should persist");
    }
}
