use rusqlite::{params, Connection};

use crate::persistence::error::PersistenceError;
use crate::translation_chunks::{
    NewTranslationChunk, NewTranslationChunkSegment, TranslationChunkSegmentSummary,
    TranslationChunkSummary,
};

pub struct TranslationChunkRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> TranslationChunkRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn replace_for_document(
        &mut self,
        document_id: &str,
        chunks: &[NewTranslationChunk],
        chunk_segments: &[NewTranslationChunkSegment],
        _built_at: i64,
    ) -> Result<(), PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The translation-chunk repository could not start the chunking transaction for document {document_id}."
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
                        "The translation-chunk repository could not clear previous chunks for document {document_id}."
                    ),
                    error,
                )
            })?;

        for chunk in chunks {
            transaction
                .execute(
                    r#"
                    INSERT INTO translation_chunks (
                      id,
                      document_id,
                      sequence,
                      builder_version,
                      strategy,
                      source_text,
                      context_before_text,
                      context_after_text,
                      start_segment_sequence,
                      end_segment_sequence,
                      segment_count,
                      source_word_count,
                      source_character_count,
                      created_at,
                      updated_at
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                    "#,
                    params![
                        chunk.id,
                        chunk.document_id,
                        chunk.sequence,
                        chunk.builder_version,
                        chunk.strategy,
                        chunk.source_text,
                        chunk.context_before_text,
                        chunk.context_after_text,
                        chunk.start_segment_sequence,
                        chunk.end_segment_sequence,
                        chunk.segment_count,
                        chunk.source_word_count,
                        chunk.source_character_count,
                        chunk.created_at,
                        chunk.updated_at
                    ],
                )
                .map_err(|error| {
                    PersistenceError::with_details(
                        format!(
                            "The translation-chunk repository could not persist chunk {} for document {document_id}.",
                            chunk.id
                        ),
                        error,
                    )
                })?;
        }

        for chunk_segment in chunk_segments {
            transaction
                .execute(
                    r#"
                    INSERT INTO translation_chunk_segments (
                      chunk_id,
                      segment_id,
                      segment_sequence,
                      position,
                      role
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    "#,
                    params![
                        chunk_segment.chunk_id,
                        chunk_segment.segment_id,
                        chunk_segment.segment_sequence,
                        chunk_segment.position,
                        chunk_segment.role
                    ],
                )
                .map_err(|error| {
                    PersistenceError::with_details(
                        format!(
                            "The translation-chunk repository could not persist segment link {} -> {}.",
                            chunk_segment.chunk_id, chunk_segment.segment_id
                        ),
                        error,
                    )
                })?;
        }

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The translation-chunk repository could not commit chunk persistence for document {document_id}."
                ),
                error,
            )
        })?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete_for_document(&mut self, document_id: &str) -> Result<(), PersistenceError> {
        self.connection
            .execute(
                "DELETE FROM translation_chunks WHERE document_id = ?1",
                [document_id],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The translation-chunk repository could not delete chunks for document {document_id}."
                    ),
                    error,
                )
            })?;

        Ok(())
    }

    pub fn list_chunks_by_document(
        &mut self,
        document_id: &str,
    ) -> Result<Vec<TranslationChunkSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  document_id,
                  sequence,
                  builder_version,
                  strategy,
                  source_text,
                  context_before_text,
                  context_after_text,
                  start_segment_sequence,
                  end_segment_sequence,
                  segment_count,
                  source_word_count,
                  source_character_count,
                  created_at,
                  updated_at
                FROM translation_chunks
                WHERE document_id = ?1
                ORDER BY sequence ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The translation-chunk repository could not prepare the chunk listing query for document {document_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([document_id], |row| {
                Ok(TranslationChunkSummary {
                    id: row.get(0)?,
                    document_id: row.get(1)?,
                    sequence: row.get(2)?,
                    builder_version: row.get(3)?,
                    strategy: row.get(4)?,
                    source_text: row.get(5)?,
                    context_before_text: row.get(6)?,
                    context_after_text: row.get(7)?,
                    start_segment_sequence: row.get(8)?,
                    end_segment_sequence: row.get(9)?,
                    segment_count: row.get(10)?,
                    source_word_count: row.get(11)?,
                    source_character_count: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The translation-chunk repository could not read chunks for document {document_id}."
                    ),
                    error,
                )
            })?;

        let mut chunks = Vec::new();

        for row in rows {
            chunks.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The translation-chunk repository could not decode a chunk row for document {document_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(chunks)
    }

    pub fn list_chunk_segments_by_document(
        &mut self,
        document_id: &str,
    ) -> Result<Vec<TranslationChunkSegmentSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  translation_chunk_segments.chunk_id,
                  translation_chunk_segments.segment_id,
                  translation_chunk_segments.segment_sequence,
                  translation_chunk_segments.position,
                  translation_chunk_segments.role
                FROM translation_chunk_segments
                INNER JOIN translation_chunks
                  ON translation_chunks.id = translation_chunk_segments.chunk_id
                WHERE translation_chunks.document_id = ?1
                ORDER BY
                  translation_chunks.sequence ASC,
                  CASE translation_chunk_segments.role
                    WHEN 'context_before' THEN 1
                    WHEN 'core' THEN 2
                    WHEN 'context_after' THEN 3
                    ELSE 4
                  END ASC,
                  translation_chunk_segments.position ASC,
                  translation_chunk_segments.segment_sequence ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The translation-chunk repository could not prepare the chunk-link listing query for document {document_id}."
                    ),
                    error,
                )
            })?;

        let rows = statement
            .query_map([document_id], |row| {
                Ok(TranslationChunkSegmentSummary {
                    chunk_id: row.get(0)?,
                    segment_id: row.get(1)?,
                    segment_sequence: row.get(2)?,
                    position: row.get(3)?,
                    role: row.get(4)?,
                })
            })
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The translation-chunk repository could not read chunk links for document {document_id}."
                    ),
                    error,
                )
            })?;

        let mut chunk_segments = Vec::new();

        for row in rows {
            chunk_segments.push(row.map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The translation-chunk repository could not decode a chunk-link row for document {document_id}."
                    ),
                    error,
                )
            })?);
        }

        Ok(chunk_segments)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::TranslationChunkRepository;
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::segments::SegmentRepository;
    use crate::projects::NewProject;
    use crate::segments::{NewSegment, SEGMENT_STATUS_PENDING_TRANSLATION};
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-tr12";

    #[test]
    fn replace_and_list_translation_chunks_round_trip() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: "prj_active_001".to_owned(),
                name: "Chunk project".to_owned(),
                description: None,
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("project should persist");

        crate::persistence::documents::DocumentRepository::new(&mut connection)
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

        SegmentRepository::new(&mut connection)
            .replace_for_document(
                "prj_active_001",
                "doc_chunk_001",
                &[
                    NewSegment {
                        id: "doc_chunk_001_seg_0001".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 1,
                        source_text: "One.".to_owned(),
                        source_word_count: 1,
                        source_character_count: 4,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "doc_chunk_001_seg_0002".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 2,
                        source_text: "Two.".to_owned(),
                        source_word_count: 1,
                        source_character_count: 4,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "doc_chunk_001_seg_0003".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 3,
                        source_text: "Three.".to_owned(),
                        source_word_count: 1,
                        source_character_count: 6,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                ],
                now,
            )
            .expect("segments should persist");

        let mut repository = TranslationChunkRepository::new(&mut connection);
        repository
            .replace_for_document(
                "doc_chunk_001",
                &[NewTranslationChunk {
                    id: "doc_chunk_001_chunk_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    builder_version: "tr12-basic-v1".to_owned(),
                    strategy: "section-aware-fixed-word-target-v1".to_owned(),
                    source_text: "One.\n\nTwo.".to_owned(),
                    context_before_text: None,
                    context_after_text: Some("Three.".to_owned()),
                    start_segment_sequence: 1,
                    end_segment_sequence: 2,
                    segment_count: 2,
                    source_word_count: 2,
                    source_character_count: 8,
                    created_at: now,
                    updated_at: now,
                }],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                ],
                now,
            )
            .expect("chunks should persist");

        let chunks = repository
            .list_chunks_by_document("doc_chunk_001")
            .expect("chunks should load");
        let chunk_segments = repository
            .list_chunk_segments_by_document("doc_chunk_001")
            .expect("chunk links should load");

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_segment_sequence, 1);
        assert_eq!(chunks[0].end_segment_sequence, 2);
        assert_eq!(chunk_segments.len(), 3);
        assert_eq!(chunk_segments[0].role, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE);
        assert_eq!(
            chunk_segments[2].role,
            TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER
        );
    }

    #[test]
    fn replace_for_document_removes_previous_chunk_set() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: "prj_active_001".to_owned(),
                name: "Chunk project".to_owned(),
                description: None,
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("project should persist");

        crate::persistence::documents::DocumentRepository::new(&mut connection)
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

        SegmentRepository::new(&mut connection)
            .replace_for_document(
                "prj_active_001",
                "doc_chunk_001",
                &[
                    NewSegment {
                        id: "doc_chunk_001_seg_0001".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 1,
                        source_text: "One.".to_owned(),
                        source_word_count: 1,
                        source_character_count: 4,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                    NewSegment {
                        id: "doc_chunk_001_seg_0002".to_owned(),
                        document_id: "doc_chunk_001".to_owned(),
                        sequence: 2,
                        source_text: "Two.".to_owned(),
                        source_word_count: 1,
                        source_character_count: 4,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    },
                ],
                now,
            )
            .expect("segments should persist");

        let mut repository = TranslationChunkRepository::new(&mut connection);
        repository
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
                    context_after_text: Some("Two.".to_owned()),
                    start_segment_sequence: 1,
                    end_segment_sequence: 1,
                    segment_count: 1,
                    source_word_count: 1,
                    source_character_count: 4,
                    created_at: now,
                    updated_at: now,
                }],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0002".to_owned(),
                        segment_sequence: 2,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                ],
                now,
            )
            .expect("initial chunks should persist");

        repository
            .replace_for_document(
                "doc_chunk_001",
                &[NewTranslationChunk {
                    id: "doc_chunk_001_chunk_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    builder_version: "tr12-basic-v1".to_owned(),
                    strategy: "section-aware-fixed-word-target-v1".to_owned(),
                    source_text: "One.\n\nTwo.".to_owned(),
                    context_before_text: None,
                    context_after_text: None,
                    start_segment_sequence: 1,
                    end_segment_sequence: 2,
                    segment_count: 2,
                    source_word_count: 2,
                    source_character_count: 8,
                    created_at: now + 1,
                    updated_at: now + 1,
                }],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_chunk_001_chunk_0001".to_owned(),
                        segment_id: "doc_chunk_001_seg_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                ],
                now + 1,
            )
            .expect("replacement chunks should persist");

        let chunks = repository
            .list_chunks_by_document("doc_chunk_001")
            .expect("chunks should load");
        let chunk_segments = repository
            .list_chunk_segments_by_document("doc_chunk_001")
            .expect("chunk links should load");

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].segment_count, 2);
        assert_eq!(chunk_segments.len(), 2);
    }
}
