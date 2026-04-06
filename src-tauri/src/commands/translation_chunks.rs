use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::commands::segments::list_document_segments_with_runtime;
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::translation_chunks::TranslationChunkRepository;
use crate::sections::{DocumentSectionSummary, DOCUMENT_SECTION_TYPE_DOCUMENT};
use crate::segments::{ListDocumentSegmentsInput, SegmentSummary};
use crate::translation_chunks::{
    BuildDocumentTranslationChunksInput, DocumentTranslationChunksOverview,
    ListDocumentTranslationChunksInput, NewTranslationChunk, NewTranslationChunkSegment,
    TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER, TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE,
    TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
};

const CHUNK_BUILDER_VERSION: &str = "tr12-basic-v1";
const CHUNK_STRATEGY: &str = "section-aware-fixed-word-target-v1";
const CHUNK_CORE_TARGET_WORDS: i64 = 160;

#[tauri::command]
pub fn build_document_translation_chunks(
    input: BuildDocumentTranslationChunksInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<DocumentTranslationChunksOverview, DesktopCommandError> {
    build_document_translation_chunks_with_runtime(input, database_runtime.inner())
}

#[tauri::command]
pub fn list_document_translation_chunks(
    input: ListDocumentTranslationChunksInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<DocumentTranslationChunksOverview, DesktopCommandError> {
    list_document_translation_chunks_with_runtime(input, database_runtime.inner())
}

pub(crate) fn build_document_translation_chunks_with_runtime(
    input: BuildDocumentTranslationChunksInput,
    database_runtime: &DatabaseRuntime,
) -> Result<DocumentTranslationChunksOverview, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let segment_overview = list_document_segments_with_runtime(
        ListDocumentSegmentsInput {
            project_id: project_id.clone(),
            document_id: document_id.clone(),
        },
        database_runtime,
    )?;
    let built_at = current_timestamp()?;
    let (chunks, chunk_segments) = build_translation_chunks(
        &document_id,
        &segment_overview.segments,
        &segment_overview.sections,
        built_at,
    )?;

    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for chunk building.",
            Some(error.to_string()),
        )
    })?;

    TranslationChunkRepository::new(&mut connection)
        .replace_for_document(&document_id, &chunks, &chunk_segments, built_at)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not persist translation chunks for the selected document.",
                Some(error.to_string()),
            )
        })?;

    load_translation_chunk_overview(&mut connection, &project_id, &document_id)
}

pub(crate) fn list_document_translation_chunks_with_runtime(
    input: ListDocumentTranslationChunksInput,
    database_runtime: &DatabaseRuntime,
) -> Result<DocumentTranslationChunksOverview, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;

    let _ = list_document_segments_with_runtime(
        ListDocumentSegmentsInput {
            project_id: project_id.clone(),
            document_id: document_id.clone(),
        },
        database_runtime,
    )?;

    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for chunk listing.",
            Some(error.to_string()),
        )
    })?;

    load_translation_chunk_overview(&mut connection, &project_id, &document_id)
}

fn load_translation_chunk_overview(
    connection: &mut rusqlite::Connection,
    project_id: &str,
    document_id: &str,
) -> Result<DocumentTranslationChunksOverview, DesktopCommandError> {
    let mut repository = TranslationChunkRepository::new(connection);
    let chunks = repository
        .list_chunks_by_document(document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load translation chunks for the selected document.",
                Some(error.to_string()),
            )
        })?;
    let chunk_segments = repository
        .list_chunk_segments_by_document(document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load translation chunk links for the selected document.",
                Some(error.to_string()),
            )
        })?;

    Ok(DocumentTranslationChunksOverview {
        project_id: project_id.to_owned(),
        document_id: document_id.to_owned(),
        chunks,
        chunk_segments,
    })
}

fn build_translation_chunks(
    document_id: &str,
    segments: &[SegmentSummary],
    sections: &[DocumentSectionSummary],
    built_at: i64,
) -> Result<(Vec<NewTranslationChunk>, Vec<NewTranslationChunkSegment>), DesktopCommandError> {
    if segments.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let chunk_scopes = build_chunk_scopes(segments, sections);
    let mut chunks = Vec::new();
    let mut chunk_segments = Vec::new();
    let mut next_sequence = 1_i64;

    for scope in chunk_scopes {
        let scope_segments: Vec<&SegmentSummary> = segments
            .iter()
            .filter(|segment| {
                segment.sequence >= scope.start_segment_sequence
                    && segment.sequence <= scope.end_segment_sequence
            })
            .collect();

        let mut next_index = 0_usize;

        while next_index < scope_segments.len() {
            let chunk_start_index = next_index;
            let mut accumulated_word_count = 0_i64;

            while next_index < scope_segments.len() {
                let candidate = scope_segments[next_index];
                let next_word_total = accumulated_word_count + candidate.source_word_count;

                if next_index > chunk_start_index && next_word_total > CHUNK_CORE_TARGET_WORDS {
                    break;
                }

                accumulated_word_count = next_word_total;
                next_index += 1;
            }

            let core_segments = &scope_segments[chunk_start_index..next_index];
            let context_before = if chunk_start_index > 0 {
                Some(scope_segments[chunk_start_index - 1])
            } else {
                None
            };
            let context_after = if next_index < scope_segments.len() {
                Some(scope_segments[next_index])
            } else {
                None
            };
            let chunk_id = format!("{document_id}_chunk_{next_sequence:04}");
            let start_segment_sequence = core_segments
                .first()
                .map(|segment| segment.sequence)
                .ok_or_else(|| {
                    DesktopCommandError::internal(
                        "The desktop shell produced an empty chunk core during chunk building.",
                        None,
                    )
                })?;
            let end_segment_sequence =
                core_segments
                    .last()
                    .map(|segment| segment.sequence)
                    .ok_or_else(|| {
                        DesktopCommandError::internal(
                            "The desktop shell produced an invalid chunk boundary during chunk building.",
                            None,
                        )
                    })?;

            chunks.push(NewTranslationChunk {
                id: chunk_id.clone(),
                document_id: document_id.to_owned(),
                sequence: next_sequence,
                builder_version: CHUNK_BUILDER_VERSION.to_owned(),
                strategy: CHUNK_STRATEGY.to_owned(),
                source_text: join_segment_texts(core_segments),
                context_before_text: context_before.map(|segment| segment.source_text.clone()),
                context_after_text: context_after.map(|segment| segment.source_text.clone()),
                start_segment_sequence,
                end_segment_sequence,
                segment_count: i64::try_from(core_segments.len()).map_err(|error| {
                    DesktopCommandError::internal(
                        "The desktop shell produced an invalid chunk segment count.",
                        Some(error.to_string()),
                    )
                })?,
                source_word_count: core_segments
                    .iter()
                    .map(|segment| segment.source_word_count)
                    .sum(),
                source_character_count: core_segments
                    .iter()
                    .map(|segment| segment.source_character_count)
                    .sum(),
                created_at: built_at,
                updated_at: built_at,
            });

            if let Some(segment) = context_before {
                chunk_segments.push(NewTranslationChunkSegment {
                    chunk_id: chunk_id.clone(),
                    segment_id: segment.id.clone(),
                    segment_sequence: segment.sequence,
                    position: 1,
                    role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_BEFORE.to_owned(),
                });
            }

            for (index, segment) in core_segments.iter().enumerate() {
                chunk_segments.push(NewTranslationChunkSegment {
                    chunk_id: chunk_id.clone(),
                    segment_id: segment.id.clone(),
                    segment_sequence: segment.sequence,
                    position: i64::try_from(index + 1).map_err(|error| {
                        DesktopCommandError::internal(
                            "The desktop shell produced an invalid core position while building translation chunks.",
                            Some(error.to_string()),
                        )
                    })?,
                    role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                });
            }

            if let Some(segment) = context_after {
                chunk_segments.push(NewTranslationChunkSegment {
                    chunk_id,
                    segment_id: segment.id.clone(),
                    segment_sequence: segment.sequence,
                    position: 1,
                    role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                });
            }

            next_sequence += 1;
        }
    }

    Ok((chunks, chunk_segments))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChunkScope {
    start_segment_sequence: i64,
    end_segment_sequence: i64,
}

fn build_chunk_scopes(
    segments: &[SegmentSummary],
    sections: &[DocumentSectionSummary],
) -> Vec<ChunkScope> {
    let Some(first_segment_sequence) = segments.first().map(|segment| segment.sequence) else {
        return Vec::new();
    };
    let Some(last_segment_sequence) = segments.last().map(|segment| segment.sequence) else {
        return Vec::new();
    };

    if !has_useful_structure(sections) {
        return vec![ChunkScope {
            start_segment_sequence: first_segment_sequence,
            end_segment_sequence: last_segment_sequence,
        }];
    }

    let mut scopes = Vec::new();

    for section in sections {
        let start_segment_sequence = section.start_segment_sequence.max(first_segment_sequence);
        let end_segment_sequence = section.end_segment_sequence.min(last_segment_sequence);

        if start_segment_sequence <= end_segment_sequence {
            scopes.push(ChunkScope {
                start_segment_sequence,
                end_segment_sequence,
            });
        }
    }

    if scopes.is_empty() {
        vec![ChunkScope {
            start_segment_sequence: first_segment_sequence,
            end_segment_sequence: last_segment_sequence,
        }]
    } else {
        scopes
    }
}

fn has_useful_structure(sections: &[DocumentSectionSummary]) -> bool {
    if sections.is_empty() {
        return false;
    }

    sections.len() > 1
        || sections
            .iter()
            .any(|section| section.section_type != DOCUMENT_SECTION_TYPE_DOCUMENT)
}

fn join_segment_texts(segments: &[&SegmentSummary]) -> String {
    segments
        .iter()
        .map(|segment| segment.source_text.trim())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn current_timestamp() -> Result<i64, DesktopCommandError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not read the system clock while building translation chunks.",
                Some(error.to_string()),
            )
        })?;

    i64::try_from(duration.as_secs()).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid chunk timestamp.",
            Some(error.to_string()),
        )
    })
}

fn validate_identifier(value: &str, label: &str) -> Result<String, DesktopCommandError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(DesktopCommandError::validation(
            format!("The translation chunk flow requires a valid {label}."),
            None,
        ));
    }

    if !trimmed
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(
            format!("The translation chunk flow requires a safe persisted {label}."),
            None,
        ));
    }

    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{
        build_document_translation_chunks_with_runtime, build_translation_chunks,
        list_document_translation_chunks_with_runtime, CHUNK_BUILDER_VERSION, CHUNK_STRATEGY,
    };
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::persistence::sections::DocumentSectionRepository;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::NewProject;
    use crate::sections::{
        DocumentSectionSummary, NewDocumentSection, DOCUMENT_SECTION_TYPE_CHAPTER,
        DOCUMENT_SECTION_TYPE_SECTION,
    };
    use crate::segments::{NewSegment, SegmentSummary, SEGMENT_STATUS_PENDING_TRANSLATION};
    use crate::translation_chunks::{
        BuildDocumentTranslationChunksInput, ListDocumentTranslationChunksInput,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    fn create_runtime() -> (tempfile::TempDir, DatabaseRuntime, String) {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key =
            load_or_create_encryption_key(&encryption_key_path).expect("key should persist");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        (temporary_directory, runtime, encryption_key)
    }

    fn seed_segmented_document(
        runtime: &DatabaseRuntime,
        _encryption_key: &str,
        sections: &[NewDocumentSection],
        segments: &[NewSegment],
    ) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let now = 1_743_517_200_i64;

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
        ProjectRepository::new(&mut connection)
            .open_project("prj_active_001", now)
            .expect("project should become active");

        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: "doc_chunk_001".to_owned(),
                project_id: "prj_active_001".to_owned(),
                name: "chunked.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "ignored".to_owned(),
                file_size_bytes: 512,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: now,
                updated_at: now,
            })
            .expect("document should persist");

        SegmentRepository::new(&mut connection)
            .replace_for_document("prj_active_001", "doc_chunk_001", segments, now)
            .expect("segments should persist");

        if !sections.is_empty() {
            DocumentSectionRepository::new(&mut connection)
                .replace_for_document("doc_chunk_001", sections)
                .expect("sections should persist");
        }
    }

    #[test]
    fn build_document_translation_chunks_is_deterministic_and_persisted() {
        let (_temp_dir, runtime, encryption_key) = create_runtime();
        seed_segmented_document(
            &runtime,
            &encryption_key,
            &[],
            &[
                NewSegment {
                    id: "doc_chunk_001_seg_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    source_text: "First sentence.".to_owned(),
                    source_word_count: 2,
                    source_character_count: 15,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_200,
                    updated_at: 1_743_517_200,
                },
                NewSegment {
                    id: "doc_chunk_001_seg_0002".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 2,
                    source_text: "Second sentence.".to_owned(),
                    source_word_count: 2,
                    source_character_count: 16,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_200,
                    updated_at: 1_743_517_200,
                },
                NewSegment {
                    id: "doc_chunk_001_seg_0003".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 3,
                    source_text: "Third sentence.".to_owned(),
                    source_word_count: 2,
                    source_character_count: 15,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_200,
                    updated_at: 1_743_517_200,
                },
            ],
        );

        let first_overview = build_document_translation_chunks_with_runtime(
            BuildDocumentTranslationChunksInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
            },
            &runtime,
        )
        .expect("chunk building should succeed");
        let second_overview = build_document_translation_chunks_with_runtime(
            BuildDocumentTranslationChunksInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
            },
            &runtime,
        )
        .expect("rebuilding chunks should succeed");

        assert_eq!(first_overview.chunks.len(), 1);
        assert_eq!(second_overview.chunks.len(), 1);
        assert_eq!(first_overview.chunks[0].id, "doc_chunk_001_chunk_0001");
        assert_eq!(second_overview.chunks[0].id, "doc_chunk_001_chunk_0001");
        assert_eq!(
            second_overview.chunks[0].builder_version,
            CHUNK_BUILDER_VERSION
        );
        assert_eq!(second_overview.chunks[0].strategy, CHUNK_STRATEGY);
        assert_eq!(second_overview.chunks[0].segment_count, 3);
        assert_eq!(second_overview.chunk_segments.len(), 3);
    }

    #[test]
    fn build_translation_chunks_respects_sections_and_context_overlap() {
        let segments = vec![
            SegmentSummary {
                id: "seg_0001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                sequence: 1,
                source_text: "Chapter 1".to_owned(),
                target_text: None,
                source_word_count: 2,
                source_character_count: 9,
                status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                created_at: 1,
                updated_at: 1,
            },
            SegmentSummary {
                id: "seg_0002".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                sequence: 2,
                source_text: "Opening detail.".to_owned(),
                target_text: None,
                source_word_count: 2,
                source_character_count: 15,
                status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                created_at: 1,
                updated_at: 1,
            },
            SegmentSummary {
                id: "seg_0003".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                sequence: 3,
                source_text: "Section 2".to_owned(),
                target_text: None,
                source_word_count: 2,
                source_character_count: 9,
                status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                created_at: 1,
                updated_at: 1,
            },
            SegmentSummary {
                id: "seg_0004".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                sequence: 4,
                source_text: "Scoped detail.".to_owned(),
                target_text: None,
                source_word_count: 2,
                source_character_count: 14,
                status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                created_at: 1,
                updated_at: 1,
            },
        ];
        let sections = vec![
            DocumentSectionSummary {
                id: "doc_chunk_001_sec_0001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                sequence: 1,
                title: "Chapter 1".to_owned(),
                section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                level: 1,
                start_segment_sequence: 1,
                end_segment_sequence: 2,
                segment_count: 2,
                created_at: 1,
                updated_at: 1,
            },
            DocumentSectionSummary {
                id: "doc_chunk_001_sec_0002".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
                sequence: 2,
                title: "Section 2".to_owned(),
                section_type: DOCUMENT_SECTION_TYPE_SECTION.to_owned(),
                level: 2,
                start_segment_sequence: 3,
                end_segment_sequence: 4,
                segment_count: 2,
                created_at: 1,
                updated_at: 1,
            },
        ];

        let (chunks, chunk_segments) =
            build_translation_chunks("doc_chunk_001", &segments, &sections, 10)
                .expect("chunk building should succeed");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].start_segment_sequence, 1);
        assert_eq!(chunks[0].end_segment_sequence, 2);
        assert_eq!(chunks[1].start_segment_sequence, 3);
        assert_eq!(chunks[1].end_segment_sequence, 4);
        assert_eq!(chunks[0].context_after_text, None);
        assert_eq!(chunks[1].context_before_text, None);
        assert_eq!(
            chunk_segments
                .iter()
                .filter(|chunk_segment| chunk_segment.role == TRANSLATION_CHUNK_SEGMENT_ROLE_CORE)
                .count(),
            4
        );
    }

    #[test]
    fn list_document_translation_chunks_returns_links_in_stable_order() {
        let (_temp_dir, runtime, encryption_key) = create_runtime();
        seed_segmented_document(
            &runtime,
            &encryption_key,
            &[],
            &[
                NewSegment {
                    id: "doc_chunk_001_seg_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    source_text: "One.".to_owned(),
                    source_word_count: 1,
                    source_character_count: 4,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_200,
                    updated_at: 1_743_517_200,
                },
                NewSegment {
                    id: "doc_chunk_001_seg_0002".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 2,
                    source_text: "Two.".to_owned(),
                    source_word_count: 1,
                    source_character_count: 4,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_200,
                    updated_at: 1_743_517_200,
                },
                NewSegment {
                    id: "doc_chunk_001_seg_0003".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 3,
                    source_text: "Three.".to_owned(),
                    source_word_count: 1,
                    source_character_count: 6,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_200,
                    updated_at: 1_743_517_200,
                },
            ],
        );

        build_document_translation_chunks_with_runtime(
            BuildDocumentTranslationChunksInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
            },
            &runtime,
        )
        .expect("chunk building should succeed");

        let overview = list_document_translation_chunks_with_runtime(
            ListDocumentTranslationChunksInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
            },
            &runtime,
        )
        .expect("chunk listing should succeed");

        assert_eq!(overview.chunks.len(), 1);
        assert_eq!(overview.chunk_segments.len(), 3);
        assert_eq!(
            overview.chunk_segments[0].role,
            TRANSLATION_CHUNK_SEGMENT_ROLE_CORE
        );
        assert_eq!(
            overview.chunk_segments[1].role,
            TRANSLATION_CHUNK_SEGMENT_ROLE_CORE
        );
        assert_eq!(
            overview.chunk_segments[2].role,
            TRANSLATION_CHUNK_SEGMENT_ROLE_CORE
        );
    }

    #[test]
    fn resegmenting_document_invalidates_previous_chunks() {
        let (_temp_dir, runtime, encryption_key) = create_runtime();
        seed_segmented_document(
            &runtime,
            &encryption_key,
            &[],
            &[
                NewSegment {
                    id: "doc_chunk_001_seg_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    source_text: "One.".to_owned(),
                    source_word_count: 1,
                    source_character_count: 4,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_200,
                    updated_at: 1_743_517_200,
                },
                NewSegment {
                    id: "doc_chunk_001_seg_0002".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 2,
                    source_text: "Two.".to_owned(),
                    source_word_count: 1,
                    source_character_count: 4,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_200,
                    updated_at: 1_743_517_200,
                },
            ],
        );

        build_document_translation_chunks_with_runtime(
            BuildDocumentTranslationChunksInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_chunk_001".to_owned(),
            },
            &runtime,
        )
        .expect("chunk building should succeed");

        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        SegmentRepository::new(&mut connection)
            .replace_for_document(
                "prj_active_001",
                "doc_chunk_001",
                &[NewSegment {
                    id: "doc_chunk_001_seg_0001".to_owned(),
                    document_id: "doc_chunk_001".to_owned(),
                    sequence: 1,
                    source_text: "Replacement.".to_owned(),
                    source_word_count: 1,
                    source_character_count: 12,
                    status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                    created_at: 1_743_517_201,
                    updated_at: 1_743_517_201,
                }],
                1_743_517_201,
            )
            .expect("resegmentation should succeed");

        let mut chunk_repository = TranslationChunkRepository::new(&mut connection);
        let chunks = chunk_repository
            .list_chunks_by_document("doc_chunk_001")
            .expect("chunks should load");
        let chunk_segments = chunk_repository
            .list_chunk_segments_by_document("doc_chunk_001")
            .expect("chunk links should load");

        assert!(chunks.is_empty());
        assert!(chunk_segments.is_empty());
    }
}
