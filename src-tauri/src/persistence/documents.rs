use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::documents::{DocumentSummary, NewDocument, ProjectDocumentsOverview};
use crate::persistence::error::PersistenceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredDocumentRecord {
    pub document_id: String,
    pub stored_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentProcessingRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub source_kind: String,
    pub format: String,
    pub mime_type: Option<String>,
    pub file_size_bytes: i64,
    pub status: String,
    pub stored_path: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct DocumentRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> DocumentRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn create(
        &mut self,
        new_document: &NewDocument,
    ) -> Result<DocumentSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The document repository could not start the document import transaction.",
                error,
            )
        })?;

        transaction
            .execute(
                r#"
                INSERT INTO documents (
                  id,
                  project_id,
                  name,
                  source_kind,
                  format,
                  mime_type,
                  stored_path,
                  file_size_bytes,
                  status,
                  created_at,
                  updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    new_document.id,
                    new_document.project_id,
                    new_document.name,
                    new_document.source_kind,
                    new_document.format,
                    new_document.mime_type,
                    new_document.stored_path,
                    new_document.file_size_bytes,
                    new_document.status,
                    new_document.created_at,
                    new_document.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The document repository could not persist the imported document.",
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The document repository could not commit the document import transaction.",
                error,
            )
        })?;

        Ok(DocumentSummary {
            id: new_document.id.clone(),
            project_id: new_document.project_id.clone(),
            name: new_document.name.clone(),
            source_kind: new_document.source_kind.clone(),
            format: new_document.format.clone(),
            mime_type: new_document.mime_type.clone(),
            file_size_bytes: new_document.file_size_bytes,
            status: new_document.status.clone(),
            segment_count: 0,
            created_at: new_document.created_at,
            updated_at: new_document.updated_at,
        })
    }

    pub fn list_by_project(
        &mut self,
        project_id: &str,
    ) -> Result<Vec<DocumentSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  project_id,
                  name,
                  source_kind,
                  format,
                  mime_type,
                  file_size_bytes,
                  status,
                  (
                    SELECT COUNT(*)
                    FROM segments
                    WHERE document_id = documents.id
                  ) AS segment_count,
                  created_at,
                  updated_at
                FROM documents
                WHERE project_id = ?1
                ORDER BY created_at DESC, name COLLATE NOCASE ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not prepare the listing query for project {project_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([project_id], map_document_summary)
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not read the document list for project {project_id}."
                    ),
                    error,
                )
            })?;

        let mut documents = Vec::new();

        for row in rows {
            documents.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not decode a document row for project {project_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(documents)
    }

    #[cfg(test)]
    pub fn list_stored_paths_by_project(
        &mut self,
        project_id: &str,
    ) -> Result<Vec<String>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare("SELECT stored_path FROM documents WHERE project_id = ?1")
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not prepare the stored-path query for project {project_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([project_id], |row| row.get::<_, String>(0))
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not read stored paths for project {project_id}."
                    ),
                    error,
                )
            })?;

        let mut stored_paths = Vec::new();

        for row in rows {
            stored_paths.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not decode a stored path for project {project_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(stored_paths)
    }

    pub fn load_processing_record(
        &mut self,
        project_id: &str,
        document_id: &str,
    ) -> Result<Option<DocumentProcessingRecord>, PersistenceError> {
        self.connection
            .query_row(
                r#"
                SELECT
                  id,
                  project_id,
                  name,
                  source_kind,
                  format,
                  mime_type,
                  file_size_bytes,
                  status,
                  stored_path,
                  created_at,
                  updated_at
                FROM documents
                WHERE project_id = ?1 AND id = ?2
                "#,
                params![project_id, document_id],
                |row| {
                    Ok(DocumentProcessingRecord {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        name: row.get(2)?,
                        source_kind: row.get(3)?,
                        format: row.get(4)?,
                        mime_type: row.get(5)?,
                        file_size_bytes: row.get(6)?,
                        status: row.get(7)?,
                        stored_path: row.get(8)?,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    })
                },
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not load processing state for document {document_id}."
                    ),
                    error,
                )
            })
    }

    pub fn list_storage_records_by_project(
        &mut self,
        project_id: &str,
    ) -> Result<Vec<StoredDocumentRecord>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare("SELECT id, stored_path FROM documents WHERE project_id = ?1")
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not prepare the storage-record query for project {project_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([project_id], |row| {
                Ok(StoredDocumentRecord {
                    document_id: row.get(0)?,
                    stored_path: row.get(1)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not read storage records for project {project_id}."
                    ),
                    error,
                )
            })?;

        let mut storage_records = Vec::new();

        for row in rows {
            storage_records.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not decode a storage record for project {project_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(storage_records)
    }

    pub fn update_stored_path(
        &mut self,
        document_id: &str,
        project_id: &str,
        stored_path: &str,
        updated_at: i64,
    ) -> Result<(), PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The document repository could not start finalization for document {document_id}."
                ),
                error,
            )
        })?;

        transaction
            .execute(
                "UPDATE documents SET stored_path = ?2, updated_at = ?3 WHERE id = ?1",
                params![document_id, stored_path, updated_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not update the stored path for document {document_id}."
                    ),
                    error,
                )
            })?;

        transaction
            .execute(
                "UPDATE projects SET updated_at = ?2 WHERE id = ?1",
                params![project_id, updated_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not update project {project_id} after document finalization."
                    ),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The document repository could not commit finalization for document {document_id}."
                ),
                error,
            )
        })?;

        Ok(())
    }

    pub fn delete_by_id(&mut self, document_id: &str) -> Result<(), PersistenceError> {
        self.connection
            .execute("DELETE FROM documents WHERE id = ?1", [document_id])
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not delete document {document_id}."
                    ),
                    error,
                )
            })?;

        Ok(())
    }

    pub fn load_overview(
        &mut self,
        project_id: &str,
    ) -> Result<ProjectDocumentsOverview, PersistenceError> {
        Ok(ProjectDocumentsOverview {
            project_id: project_id.to_owned(),
            documents: self.list_by_project(project_id)?,
        })
    }

    pub fn load_summary(
        &mut self,
        project_id: &str,
        document_id: &str,
    ) -> Result<Option<DocumentSummary>, PersistenceError> {
        self.connection
            .query_row(
                r#"
                SELECT
                  id,
                  project_id,
                  name,
                  source_kind,
                  format,
                  mime_type,
                  file_size_bytes,
                  status,
                  (
                    SELECT COUNT(*)
                    FROM segments
                    WHERE document_id = documents.id
                  ) AS segment_count,
                  created_at,
                  updated_at
                FROM documents
                WHERE project_id = ?1 AND id = ?2
                "#,
                params![project_id, document_id],
                map_document_summary,
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document repository could not reload document {document_id} for project {project_id}."
                    ),
                    error,
                )
            })
    }
}

fn map_document_summary(row: &Row<'_>) -> rusqlite::Result<DocumentSummary> {
    Ok(DocumentSummary {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        source_kind: row.get(3)?,
        format: row.get(4)?,
        mime_type: row.get(5)?,
        file_size_bytes: row.get(6)?,
        status: row.get(7)?,
        segment_count: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::DocumentRepository;
    use crate::documents::{
        NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_IMPORTED,
    };
    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::persistence::projects::ProjectRepository;
    use crate::projects::NewProject;

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-c2";

    fn sample_project(id: &str, name: &str, now: i64) -> NewProject {
        NewProject {
            id: id.to_owned(),
            name: name.to_owned(),
            description: Some(format!("Project container for {name}.")),
            created_at: now,
            updated_at: now,
            last_opened_at: now,
        }
    }

    fn sample_document(id: &str, project_id: &str, name: &str, now: i64) -> NewDocument {
        NewDocument {
            id: id.to_owned(),
            project_id: project_id.to_owned(),
            name: name.to_owned(),
            source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
            format: "txt".to_owned(),
            mime_type: Some("text/plain".to_owned()),
            stored_path: format!("C:\\Translat\\documents\\{project_id}\\{id}__{name}"),
            file_size_bytes: 128,
            status: DOCUMENT_STATUS_IMPORTED.to_owned(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn create_and_list_documents_round_trip() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        {
            let mut project_repository = ProjectRepository::new(&mut connection);
            project_repository
                .create(&sample_project("prj_docs_001", "Docs workspace", now))
                .expect("project should be created");
        }

        let mut repository = DocumentRepository::new(&mut connection);
        let created_document = repository
            .create(&sample_document(
                "doc_test_001",
                "prj_docs_001",
                "source.txt",
                now + 60,
            ))
            .expect("document should be created");
        let overview = repository
            .load_overview("prj_docs_001")
            .expect("document overview should load");

        assert_eq!(created_document.project_id, "prj_docs_001");
        assert_eq!(overview.project_id, "prj_docs_001");
        assert_eq!(overview.documents, vec![created_document]);
    }

    #[test]
    fn documents_survive_reopen_and_stay_scoped_to_project() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let created_at = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        {
            let mut first_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
                .expect("database connection should open");
            {
                let mut project_repository = ProjectRepository::new(&mut first_connection);
                project_repository
                    .create(&sample_project("prj_docs_001", "Project one", created_at))
                    .expect("first project should be created");
                project_repository
                    .create(&sample_project("prj_docs_002", "Project two", created_at + 1))
                    .expect("second project should be created");
            }

            let mut document_repository = DocumentRepository::new(&mut first_connection);
            document_repository
                .create(&sample_document(
                    "doc_test_001",
                    "prj_docs_001",
                    "chapter-a.txt",
                    created_at + 60,
                ))
                .expect("first document should be created");
            document_repository
                .create(&sample_document(
                    "doc_test_002",
                    "prj_docs_002",
                    "chapter-b.txt",
                    created_at + 120,
                ))
                .expect("second document should be created");
        }

        let mut second_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should reopen");
        let mut repository = DocumentRepository::new(&mut second_connection);
        let first_project_overview = repository
            .load_overview("prj_docs_001")
            .expect("first project overview should load");
        let second_project_overview = repository
            .load_overview("prj_docs_002")
            .expect("second project overview should load");

        assert_eq!(first_project_overview.documents.len(), 1);
        assert_eq!(first_project_overview.documents[0].id, "doc_test_001");
        assert_eq!(second_project_overview.documents.len(), 1);
        assert_eq!(second_project_overview.documents[0].id, "doc_test_002");
    }
}
