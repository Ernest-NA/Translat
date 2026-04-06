use rusqlite::{params, Connection};

use crate::documents::DOCUMENT_STATUS_SEGMENTED;
use crate::persistence::error::PersistenceError;
use crate::segments::{NewSegment, SegmentSummary};

pub struct SegmentRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> SegmentRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn replace_for_document(
        &mut self,
        project_id: &str,
        document_id: &str,
        segments: &[NewSegment],
        processed_at: i64,
    ) -> Result<(), PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The segment repository could not start the segmentation transaction for document {document_id}."
                ),
                error,
            )
        })?;

        transaction
            .execute(
                "DELETE FROM translation_chunks WHERE document_id = ?1",
                [document_id],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The segment repository could not clear previous translation chunks for document {document_id}."
                    ),
                    error,
                )
            })?;

        transaction
            .execute("DELETE FROM segments WHERE document_id = ?1", [document_id])
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The segment repository could not clear previous segments for document {document_id}."
                    ),
                    error,
                )
            })?;

        for segment in segments {
            transaction
                .execute(
                    r#"
                    INSERT INTO segments (
                      id,
                      document_id,
                      sequence,
                      source_text,
                      source_word_count,
                      source_character_count,
                      status,
                      created_at,
                      updated_at
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    "#,
                    params![
                        segment.id,
                        segment.document_id,
                        segment.sequence,
                        segment.source_text,
                        segment.source_word_count,
                        segment.source_character_count,
                        segment.status,
                        segment.created_at,
                        segment.updated_at
                    ],
                )
                .map_err(|error| {
                    PersistenceError::with_details(
                        format!(
                            "The segment repository could not persist segment {} for document {document_id}.",
                            segment.id
                        ),
                        error,
                    )
                })?;
        }

        let updated_documents = transaction
            .execute(
                "UPDATE documents SET status = ?3, updated_at = ?4 WHERE id = ?1 AND project_id = ?2",
                params![
                    document_id,
                    project_id,
                    DOCUMENT_STATUS_SEGMENTED,
                    processed_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The segment repository could not update document {document_id} after segmentation."
                    ),
                    error,
                )
            })?;

        if updated_documents != 1 {
            return Err(PersistenceError::new(format!(
                "The segment repository could not find document {document_id} in project {project_id} while committing segmentation."
            )));
        }

        transaction
            .execute(
                "UPDATE projects SET updated_at = ?2 WHERE id = ?1",
                params![project_id, processed_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The segment repository could not update project {project_id} after segmentation."
                    ),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The segment repository could not commit segmentation for document {document_id}."
                ),
                error,
            )
        })?;

        Ok(())
    }

    pub fn list_by_document(
        &mut self,
        document_id: &str,
    ) -> Result<Vec<SegmentSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  sequence,
                  source_text,
                  source_word_count,
                  source_character_count,
                  status,
                  created_at,
                  updated_at
                FROM segments
                WHERE document_id = ?1
                ORDER BY sequence ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The segment repository could not prepare the segment listing query for document {document_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([document_id], |row| {
                Ok(SegmentSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    sequence: row.get(2)?,
                    source_text: row.get(3)?,
                    target_text: None,
                    source_word_count: row.get(4)?,
                    source_character_count: row.get(5)?,
                    status: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The segment repository could not read the segments for document {document_id}."
                    ),
                    error,
                )
            })?;

        let mut segments = Vec::new();

        for row in rows {
            segments.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The segment repository could not decode a segment row for document {document_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(segments)
    }
}
