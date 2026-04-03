use rusqlite::{params, Connection};

use crate::persistence::error::PersistenceError;
use crate::sections::{DocumentSectionSummary, NewDocumentSection};

pub struct DocumentSectionRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> DocumentSectionRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn replace_for_document(
        &mut self,
        document_id: &str,
        sections: &[NewDocumentSection],
    ) -> Result<(), PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The document-section repository could not start the persistence transaction for document {document_id}."
                ),
                error,
            )
        })?;

        transaction
            .execute(
                "DELETE FROM document_sections WHERE document_id = ?1",
                [document_id],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document-section repository could not clear previous sections for document {document_id}."
                    ),
                    error,
                )
            })?;

        for section in sections {
            transaction
                .execute(
                    r#"
                    INSERT INTO document_sections (
                      id,
                      document_id,
                      sequence,
                      title,
                      section_type,
                      level,
                      start_segment_sequence,
                      end_segment_sequence,
                      segment_count,
                      created_at,
                      updated_at
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                    "#,
                    params![
                        section.id,
                        section.document_id,
                        section.sequence,
                        section.title,
                        section.section_type,
                        section.level,
                        section.start_segment_sequence,
                        section.end_segment_sequence,
                        section.segment_count,
                        section.created_at,
                        section.updated_at
                    ],
                )
                .map_err(|error| {
                    PersistenceError::with_details(
                        format!(
                            "The document-section repository could not persist section {} for document {document_id}.",
                            section.id
                        ),
                        error,
                    )
                })?;
        }

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The document-section repository could not commit section persistence for document {document_id}."
                ),
                error,
            )
        })?;

        Ok(())
    }

    pub fn list_by_document(
        &mut self,
        document_id: &str,
    ) -> Result<Vec<DocumentSectionSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  sequence,
                  title,
                  section_type,
                  level,
                  start_segment_sequence,
                  end_segment_sequence,
                  segment_count,
                  created_at,
                  updated_at
                FROM document_sections
                WHERE document_id = ?1
                ORDER BY sequence ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document-section repository could not prepare the listing query for document {document_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([document_id], |row| {
                Ok(DocumentSectionSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    sequence: row.get(2)?,
                    title: row.get(3)?,
                    section_type: row.get(4)?,
                    level: row.get(5)?,
                    start_segment_sequence: row.get(6)?,
                    end_segment_sequence: row.get(7)?,
                    segment_count: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document-section repository could not read sections for document {document_id}."
                    ),
                    error,
                )
            })?;

        let mut sections = Vec::new();

        for row in rows {
            sections.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The document-section repository could not decode a section row for document {document_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(sections)
    }
}
