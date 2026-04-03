use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::State;

use crate::commands::documents::reconcile_project_document_storage;
use crate::documents::{DocumentSummary, DOCUMENT_STATUS_IMPORTED, DOCUMENT_STATUS_SEGMENTED};
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::documents::{DocumentProcessingRecord, DocumentRepository};
use crate::persistence::projects::ProjectRepository;
use crate::persistence::secret_store;
use crate::persistence::segments::SegmentRepository;
use crate::segments::{
    DocumentSegmentsOverview, ListDocumentSegmentsInput, NewSegment, ProcessDocumentInput,
    SEGMENT_STATUS_PENDING_TRANSLATION,
};

#[tauri::command]
pub fn list_document_segments(
    input: ListDocumentSegmentsInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<DocumentSegmentsOverview, DesktopCommandError> {
    list_document_segments_with_runtime(input, database_runtime.inner())
}

#[tauri::command]
pub fn process_project_document(
    input: ProcessDocumentInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<DocumentSummary, DesktopCommandError> {
    process_project_document_with_runtime(input, database_runtime.inner())
}

fn process_project_document_with_runtime(
    input: ProcessDocumentInput,
    database_runtime: &DatabaseRuntime,
) -> Result<DocumentSummary, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let processed_at = current_timestamp()?;

    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for document processing.",
            Some(error.to_string()),
        )
    })?;

    ensure_project_exists(&mut connection, &project_id)?;
    ensure_project_is_active(&mut connection, &project_id)?;
    reconcile_project_document_storage(database_runtime, &mut connection, &project_id)?;

    let processing_record = {
        let mut document_repository = DocumentRepository::new(&mut connection);
        document_repository
            .load_processing_record(&project_id, &document_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not inspect the selected document for segmentation.",
                    Some(error.to_string()),
                )
            })?
            .ok_or_else(|| {
                DesktopCommandError::validation(
                    "The selected document does not exist in the active project.",
                    None,
                )
            })?
    };

    ensure_document_can_be_processed(&processing_record)?;

    let document_text = load_document_text(&processing_record)?;
    let normalized_text = normalize_document_text(&document_text);
    let segments = build_segments(&document_id, &normalized_text, processed_at)?;

    {
        let mut segment_repository = SegmentRepository::new(&mut connection);
        segment_repository
            .replace_for_document(&project_id, &document_id, &segments, processed_at)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not persist the segmented document.",
                    Some(error.to_string()),
                )
            })?;
    }

    Ok(DocumentSummary {
        id: processing_record.id,
        project_id: processing_record.project_id,
        name: processing_record.name,
        source_kind: processing_record.source_kind,
        format: processing_record.format,
        mime_type: processing_record.mime_type,
        file_size_bytes: processing_record.file_size_bytes,
        status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
        segment_count: i64::try_from(segments.len()).map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell produced an invalid persisted segment count.",
                Some(error.to_string()),
            )
        })?,
        created_at: processing_record.created_at,
        updated_at: processed_at,
    })
}

fn list_document_segments_with_runtime(
    input: ListDocumentSegmentsInput,
    database_runtime: &DatabaseRuntime,
) -> Result<DocumentSegmentsOverview, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for segment listing.",
            Some(error.to_string()),
        )
    })?;

    ensure_project_exists(&mut connection, &project_id)?;
    ensure_project_is_active(&mut connection, &project_id)?;
    reconcile_project_document_storage(database_runtime, &mut connection, &project_id)?;

    let processing_record = {
        let mut document_repository = DocumentRepository::new(&mut connection);
        document_repository
            .load_processing_record(&project_id, &document_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not inspect the selected document for segment navigation.",
                    Some(error.to_string()),
                )
            })?
            .ok_or_else(|| {
                DesktopCommandError::validation(
                    "The selected document does not exist in the active project.",
                    None,
                )
            })?
    };

    if processing_record.status != DOCUMENT_STATUS_SEGMENTED {
        return Err(DesktopCommandError::validation(
            "The selected document must be segmented before its persisted segments can be opened.",
            None,
        ));
    }

    let mut segment_repository = SegmentRepository::new(&mut connection);
    let segments = segment_repository
        .list_by_document(&document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load the persisted segments for the selected document.",
                Some(error.to_string()),
            )
        })?;

    Ok(DocumentSegmentsOverview {
        project_id,
        document_id,
        segments,
    })
}

fn validate_identifier(value: &str, label: &str) -> Result<String, DesktopCommandError> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Err(DesktopCommandError::validation(
            format!("The document processing flow requires a valid {label}."),
            None,
        ));
    }

    if !trimmed
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(
            format!("The document processing flow requires a safe persisted {label}."),
            None,
        ));
    }

    Ok(trimmed.to_owned())
}

fn ensure_project_exists(
    connection: &mut rusqlite::Connection,
    project_id: &str,
) -> Result<(), DesktopCommandError> {
    let mut repository = ProjectRepository::new(connection);
    let project_exists = repository.exists(project_id).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not validate the selected project for document processing.",
            Some(error.to_string()),
        )
    })?;

    if !project_exists {
        return Err(DesktopCommandError::validation(
            "The selected project does not exist anymore.",
            None,
        ));
    }

    Ok(())
}

fn ensure_project_is_active(
    connection: &mut rusqlite::Connection,
    project_id: &str,
) -> Result<(), DesktopCommandError> {
    let mut repository = ProjectRepository::new(connection);
    let active_project_id = repository.active_project_id().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not load the active project selection for document processing.",
            Some(error.to_string()),
        )
    })?;

    if active_project_id.as_deref() != Some(project_id) {
        return Err(DesktopCommandError::validation(
            "Documents can only be processed for the currently open project.",
            None,
        ));
    }

    Ok(())
}

fn ensure_document_can_be_processed(
    processing_record: &DocumentProcessingRecord,
) -> Result<(), DesktopCommandError> {
    if matches!(
        processing_record.status.as_str(),
        DOCUMENT_STATUS_IMPORTED | DOCUMENT_STATUS_SEGMENTED
    ) {
        return Ok(());
    }

    Err(DesktopCommandError::validation(
        "The selected document is not in a state that can be segmented yet.",
        Some(format!("current status: {}", processing_record.status)),
    ))
}

fn load_document_text(
    processing_record: &DocumentProcessingRecord,
) -> Result<String, DesktopCommandError> {
    let protected_payload = fs::read(&processing_record.stored_path).map_err(|error| {
        DesktopCommandError::internal(
            format!(
                "The desktop shell could not read the stored payload for document {}.",
                processing_record.id
            ),
            Some(error.to_string()),
        )
    })?;

    let payload = secret_store::unprotect_local_payload(&protected_payload).map_err(|error| {
        DesktopCommandError::internal(
            format!(
                "The desktop shell could not decrypt the stored payload for document {}.",
                processing_record.id
            ),
            Some(error.to_string()),
        )
    })?;

    String::from_utf8(payload).map_err(|error| {
        DesktopCommandError::validation(
            "C3 currently processes UTF-8 text documents only.",
            Some(error.to_string()),
        )
    })
}

fn normalize_document_text(document_text: &str) -> String {
    let canonical_newlines = document_text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{00A0}', " ");
    let mut paragraphs = Vec::new();
    let mut current_lines = Vec::new();

    for line in canonical_newlines.lines() {
        let normalized_line = line.split_whitespace().collect::<Vec<_>>().join(" ");

        if normalized_line.is_empty() {
            if !current_lines.is_empty() {
                paragraphs.push(current_lines.join(" "));
                current_lines.clear();
            }
            continue;
        }

        current_lines.push(normalized_line);
    }

    if !current_lines.is_empty() {
        paragraphs.push(current_lines.join(" "));
    }

    paragraphs.join("\n\n")
}

fn build_segments(
    document_id: &str,
    normalized_text: &str,
    processed_at: i64,
) -> Result<Vec<NewSegment>, DesktopCommandError> {
    let mut sequence = 0_i64;
    let mut segments = Vec::new();

    for paragraph in normalized_text
        .split("\n\n")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        for source_text in split_paragraph_into_segments(paragraph) {
            let source_word_count = i64::try_from(source_text.split_whitespace().count()).map_err(
                |error| {
                    DesktopCommandError::internal(
                        "The desktop shell produced an invalid segment word count.",
                        Some(error.to_string()),
                    )
                },
            )?;
            let source_character_count =
                i64::try_from(source_text.chars().count()).map_err(|error| {
                    DesktopCommandError::internal(
                        "The desktop shell produced an invalid segment character count.",
                        Some(error.to_string()),
                    )
                })?;

            sequence += 1;
            segments.push(NewSegment {
                id: build_segment_id(document_id, sequence),
                document_id: document_id.to_owned(),
                sequence,
                source_text,
                source_word_count,
                source_character_count,
                status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                created_at: processed_at,
                updated_at: processed_at,
            });
        }
    }

    if segments.is_empty() {
        return Err(DesktopCommandError::validation(
            "The selected document did not produce any usable segments after normalization.",
            None,
        ));
    }

    Ok(segments)
}

fn split_paragraph_into_segments(paragraph: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let characters = paragraph.char_indices().collect::<Vec<_>>();
    let mut start = 0_usize;
    let mut index = 0_usize;

    while index < characters.len() {
        let (byte_index, character) = characters[index];

        if matches!(character, '.' | '!' | '?') {
            let mut end = byte_index + character.len_utf8();
            let mut lookahead = index + 1;

            while let Some((lookahead_index, lookahead_character)) = characters.get(lookahead) {
                if matches!(
                    lookahead_character,
                    '.' | '!' | '?' | '"' | '\'' | ')' | ']' | '}' | '\u{201D}' | '\u{2019}'
                ) {
                    end = *lookahead_index + lookahead_character.len_utf8();
                    lookahead += 1;
                    continue;
                }

                break;
            }

            let next_character = characters.get(lookahead).map(|(_, next)| *next);

            if (next_character.is_none() || next_character.is_some_and(char::is_whitespace))
                && should_split_at_boundary(paragraph, &characters, index, lookahead)
            {
                let candidate = paragraph[start..end].trim();

                if !candidate.is_empty() {
                    segments.push(candidate.to_owned());
                }

                while let Some((next_index, next_character)) = characters.get(lookahead) {
                    if next_character.is_whitespace() {
                        lookahead += 1;
                        start = *next_index + next_character.len_utf8();
                        continue;
                    }

                    start = *next_index;
                    break;
                }

                if lookahead >= characters.len() {
                    start = paragraph.len();
                }

                index = lookahead;
                continue;
            }
        }

        index += 1;
    }

    let trailing_segment = paragraph[start..].trim();

    if !trailing_segment.is_empty() {
        segments.push(trailing_segment.to_owned());
    }

    if segments.is_empty() {
        vec![paragraph.trim().to_owned()]
    } else {
        segments
    }
}

fn should_split_at_boundary(
    paragraph: &str,
    characters: &[(usize, char)],
    punctuation_index: usize,
    lookahead_index: usize,
) -> bool {
    let (_, punctuation) = characters[punctuation_index];

    if punctuation != '.' {
        return true;
    }

    let byte_index = characters[punctuation_index].0;
    let current_meaningful_token = previous_meaningful_token_before(paragraph, byte_index);
    let next_meaningful_character = next_meaningful_character_after(characters, lookahead_index);

    if current_meaningful_token.is_some_and(|token| {
        token.chars().any(|character| character.is_ascii_digit())
    }) && next_meaningful_character.is_some_and(|character| character.is_ascii_digit())
    {
        return false;
    }
    let current_token = match alphabetic_token_before(paragraph, byte_index) {
        Some(token) => token,
        None => return true,
    };
    let current_token_lower = current_token.to_lowercase();

    if matches!(
        current_token_lower.as_str(),
        "dr" | "dra" | "mr" | "mrs" | "ms" | "prof" | "sr" | "sra" | "srta" | "jr"
    ) {
        return false;
    }

    let previous_token_lower = previous_alphabetic_token_before(paragraph, byte_index)
        .map(|token| token.to_lowercase());
    let next_token = next_alphabetic_token_after(paragraph, characters, lookahead_index);
    let next_token_lower = next_token
        .as_ref()
        .map(|(token, _, _): &(&str, usize, bool)| token.to_lowercase());

    if current_token.chars().count() == 1
        && (previous_token_lower
            .as_ref()
            .is_some_and(|token| token.chars().count() == 1)
            || next_token
                .as_ref()
                .is_some_and(|(token, _, followed_by_period): &(&str, usize, bool)| {
                    token.chars().count() == 1 && *followed_by_period
                }))
    {
        return false;
    }

    if matches!(
        (current_token_lower.as_str(), next_token_lower.as_deref()),
        ("p", Some("ej")) | ("e", Some("g")) | ("i", Some("e"))
    ) {
        return false;
    }

    if matches!(
        (previous_token_lower.as_deref(), current_token_lower.as_str()),
        (Some("p"), "ej") | (Some("e"), "g") | (Some("i"), "e")
    ) {
        return false;
    }

    let previous_meaningful_token_lower = current_meaningful_token
        .and_then(|token| {
            let token_start = paragraph[..byte_index].rfind(token)?;
            previous_meaningful_token_before(paragraph, token_start)
        })
        .map(|token| token.to_lowercase());

    if current_meaningful_token.is_some_and(|token| {
        token.chars().all(|character| character.is_ascii_digit())
    })
        && previous_meaningful_token_lower.as_ref().is_some_and(|token| {
            matches!(
                token.as_str(),
                "chapter"
                    | "section"
                    | "sec"
                    | "part"
                    | "annex"
                    | "appendix"
                    | "capitulo"
                    | "cap\u{ed}tulo"
                    | "apartado"
                    | "seccion"
                    | "secci\u{f3}n"
            )
        })
        && next_token
            .as_ref()
            .is_some_and(|(token, _, _): &(&str, usize, bool)| {
                token.chars().next().is_some_and(char::is_uppercase)
            })
    {
        return false;
    }

    if matches!(
        current_token_lower.as_str(),
        "etc" | "no" | "art" | "cap" | "vol" | "fig" | "aprox" | "pp" | "dept"
    ) && next_meaningful_character
        .is_some_and(|character| character.is_lowercase() || character.is_ascii_digit())
    {
        return false;
    }

    true
}

fn alphabetic_token_before(paragraph: &str, byte_index: usize) -> Option<&str> {
    let prefix = &paragraph[..byte_index];
    let mut token_end = None;
    let mut token_start = 0_usize;

    for (index, character) in prefix.char_indices().rev() {
        if token_end.is_none() {
            if character.is_alphabetic() {
                token_end = Some(index + character.len_utf8());
                token_start = index;
            }
            continue;
        }

        if character.is_alphabetic() {
            token_start = index;
            continue;
        }

        break;
    }

    token_end.map(|end| &prefix[token_start..end])
}

fn previous_alphabetic_token_before(paragraph: &str, byte_index: usize) -> Option<&str> {
    let current_token = alphabetic_token_before(paragraph, byte_index)?;
    let current_start = paragraph[..byte_index].rfind(current_token)?;

    alphabetic_token_before(paragraph, current_start)
}

fn previous_meaningful_token_before(paragraph: &str, byte_index: usize) -> Option<&str> {
    let prefix = &paragraph[..byte_index];
    let mut token_end = None;
    let mut token_start = 0_usize;

    for (index, character) in prefix.char_indices().rev() {
        if token_end.is_none() {
            if character.is_alphanumeric() {
                token_end = Some(index + character.len_utf8());
                token_start = index;
            }
            continue;
        }

        if character.is_alphanumeric() {
            token_start = index;
            continue;
        }

        break;
    }

    token_end.map(|end| &prefix[token_start..end])
}

fn next_alphabetic_token_after<'a>(
    paragraph: &'a str,
    characters: &[(usize, char)],
    lookahead_index: usize,
) -> Option<(&'a str, usize, bool)> {
    let mut token_start = None;
    let mut token_end = 0_usize;
    let mut next_index = lookahead_index;

    while let Some((byte_index, character)) = characters.get(next_index) {
        if token_start.is_none() {
            if character.is_whitespace() || matches!(character, '"' | '\'' | '(' | '[' | '{') {
                next_index += 1;
                continue;
            }

            if character.is_alphabetic() {
                token_start = Some(*byte_index);
                token_end = *byte_index + character.len_utf8();
                next_index += 1;
                continue;
            }

            return None;
        }

        if character.is_alphabetic() {
            token_end = *byte_index + character.len_utf8();
            next_index += 1;
            continue;
        }

        break;
    }

    let start = token_start?;
    let followed_by_period = characters
        .get(next_index)
        .is_some_and(|(_, character)| *character == '.');

    Some((&paragraph[start..token_end], token_end, followed_by_period))
}

fn next_meaningful_character_after(
    characters: &[(usize, char)],
    lookahead_index: usize,
) -> Option<char> {
    characters
        .iter()
        .skip(lookahead_index)
        .map(|(_, character)| *character)
        .find(|character| {
            !character.is_whitespace() && !matches!(character, '"' | '\'' | '(' | '[' | '{')
        })
}

fn build_segment_id(document_id: &str, sequence: i64) -> String {
    format!("{document_id}_seg_{sequence:05}")
}

fn current_timestamp() -> Result<i64, DesktopCommandError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not compute the current segmentation timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid segmentation timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        build_segments, list_document_segments_with_runtime, normalize_document_text,
        process_project_document_with_runtime,
        split_paragraph_into_segments,
    };
    use crate::documents::{
        NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_IMPORTED, DOCUMENT_STATUS_SEGMENTED,
    };
    use crate::persistence::bootstrap::{
        bootstrap_database, open_database_with_key, DatabaseRuntime,
    };
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::secret_store::{load_or_create_encryption_key, protect_local_payload};
    use crate::persistence::segments::SegmentRepository;
    use crate::projects::NewProject;
    use crate::segments::{
        ListDocumentSegmentsInput, NewSegment, ProcessDocumentInput,
        SEGMENT_STATUS_PENDING_TRANSLATION,
    };

    #[test]
    fn normalize_document_text_canonicalizes_whitespace_and_paragraphs() {
        let normalized = normalize_document_text("Alpha\tbeta\r\nGamma  \r\n\r\n\r\nDelta");

        assert_eq!(normalized, "Alpha beta Gamma\n\nDelta");
    }

    #[test]
    fn split_paragraph_into_segments_breaks_sentences_deterministically() {
        let segments =
            split_paragraph_into_segments("First sentence. Second sentence? Third sentence!");

        assert_eq!(
            segments,
            vec![
                "First sentence.".to_owned(),
                "Second sentence?".to_owned(),
                "Third sentence!".to_owned()
            ]
        );
    }

    #[test]
    fn split_paragraph_into_segments_keeps_common_abbreviations_together() {
        let segments = split_paragraph_into_segments(
            "Dr. Smith reviewed the U.S. draft. Then added p. ej. a brief note.",
        );

        assert_eq!(
            segments,
            vec![
                "Dr. Smith reviewed the U.S. draft.".to_owned(),
                "Then added p. ej. a brief note.".to_owned(),
            ]
        );
    }

    #[test]
    fn split_paragraph_into_segments_keeps_numbered_references_together() {
        let segments = split_paragraph_into_segments(
            "Fig. 2 shows the flow. No. 5 remains pending. Art. 12 applies here.",
        );

        assert_eq!(
            segments,
            vec![
                "Fig. 2 shows the flow.".to_owned(),
                "No. 5 remains pending.".to_owned(),
                "Art. 12 applies here.".to_owned(),
            ]
        );
    }

    #[test]
    fn split_paragraph_into_segments_keeps_numbered_headings_together() {
        let segments = split_paragraph_into_segments(
            "Chapter 1. Introduction. Dept. 4 remains active. pp. 12-13 cover the scope.",
        );

        assert_eq!(
            segments,
            vec![
                "Chapter 1. Introduction.".to_owned(),
                "Dept. 4 remains active.".to_owned(),
                "pp. 12-13 cover the scope.".to_owned(),
            ]
        );
    }

    #[test]
    fn split_paragraph_into_segments_keeps_spanish_numbered_headings_together() {
        let segments = split_paragraph_into_segments(
            "Cap\u{ed}tulo 1. Introducci\u{f3}n. Secci\u{f3}n 2. Alcance.",
        );

        assert_eq!(
            segments,
            vec![
                "Cap\u{ed}tulo 1. Introducci\u{f3}n.".to_owned(),
                "Secci\u{f3}n 2. Alcance.".to_owned(),
            ]
        );
    }

    #[test]
    fn split_paragraph_into_segments_keeps_numeric_tokens_together() {
        let segments = split_paragraph_into_segments(
            "Version v1.2.3 stays stable. Pi is 3.14 in this note. Date 2026.04.03 remains grouped.",
        );

        assert_eq!(
            segments,
            vec![
                "Version v1.2.3 stays stable.".to_owned(),
                "Pi is 3.14 in this note.".to_owned(),
                "Date 2026.04.03 remains grouped.".to_owned(),
            ]
        );
    }

    #[test]
    fn build_segments_returns_ordered_segments_with_counts() {
        let segments = build_segments(
            "doc_1743517200_test",
            "Alpha beta. Gamma!\n\nDelta epsilon",
            1_743_517_200,
        )
        .expect("segments should build");

        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].id, "doc_1743517200_test_seg_00001");
        assert_eq!(segments[0].source_text, "Alpha beta.");
        assert_eq!(segments[0].source_word_count, 2);
        assert_eq!(segments[1].source_text, "Gamma!");
        assert_eq!(segments[2].source_text, "Delta epsilon");
        assert_eq!(segments[2].status, SEGMENT_STATUS_PENDING_TRANSLATION);
    }

    #[test]
    fn process_project_document_segments_payload_and_updates_document_status() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let now = 1_743_517_200_i64;
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let stored_document_path = {
            let documents_directory = runtime
                .documents_directory()
                .expect("documents directory should resolve");
            let project_directory = documents_directory.join("prj_active_001");
            fs::create_dir_all(&project_directory).expect("project directory should exist");
            let payload_path = project_directory.join("doc_1743517200_test__source.txt");
            let protected_payload = protect_local_payload(
                b"First sentence. Second sentence!\r\n\r\nThird block",
                "Translat imported document payload",
            )
            .expect("payload should be protected");
            fs::write(&payload_path, protected_payload).expect("payload should write");
            payload_path
        };

        let mut connection = open_database_with_key(&database_path, &encryption_key)
            .expect("database connection should open");
        {
            let mut project_repository = ProjectRepository::new(&mut connection);
            project_repository
                .create(&NewProject {
                    id: "prj_active_001".to_owned(),
                    name: "Active project".to_owned(),
                    description: None,
                    created_at: now,
                    updated_at: now,
                    last_opened_at: now,
                })
                .expect("project should be created");
            project_repository
                .open_project("prj_active_001", now + 1)
                .expect("project should become active");
        }
        {
            let mut document_repository = DocumentRepository::new(&mut connection);
            document_repository
                .create(&NewDocument {
                    id: "doc_1743517200_test".to_owned(),
                    project_id: "prj_active_001".to_owned(),
                    name: "source.txt".to_owned(),
                    source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                    format: "txt".to_owned(),
                    mime_type: Some("text/plain".to_owned()),
                    stored_path: stored_document_path.display().to_string(),
                    file_size_bytes: 44,
                    status: DOCUMENT_STATUS_IMPORTED.to_owned(),
                    created_at: now,
                    updated_at: now,
                })
                .expect("document should be created");
        }
        drop(connection);

        let processed_document = process_project_document_with_runtime(
            ProcessDocumentInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_1743517200_test".to_owned(),
            },
            &runtime,
        )
        .expect("document processing should succeed");

        assert_eq!(processed_document.status, DOCUMENT_STATUS_SEGMENTED);
        assert_eq!(processed_document.segment_count, 3);

        let mut reopened_connection = open_database_with_key(&database_path, &encryption_key)
            .expect("database connection should reopen");
        let segments = SegmentRepository::new(&mut reopened_connection)
            .list_by_document("doc_1743517200_test")
            .expect("segments should load");
        let reloaded_document = DocumentRepository::new(&mut reopened_connection)
            .load_summary("prj_active_001", "doc_1743517200_test")
            .expect("document summary should load")
            .expect("document should exist");

        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.source_text.clone())
                .collect::<Vec<_>>(),
            vec![
                "First sentence.".to_owned(),
                "Second sentence!".to_owned(),
                "Third block".to_owned()
            ]
        );
        assert_eq!(reloaded_document, processed_document);
    }

    #[test]
    fn process_project_document_replaces_existing_segments_when_reprocessed() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let now = 1_743_517_200_i64;
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let stored_document_path = {
            let documents_directory = runtime
                .documents_directory()
                .expect("documents directory should resolve");
            let project_directory = documents_directory.join("prj_active_001");
            fs::create_dir_all(&project_directory).expect("project directory should exist");
            let payload_path = project_directory.join("doc_1743517200_test__source.txt");
            let protected_payload = protect_local_payload(
                b"Alpha beta. Gamma delta.",
                "Translat imported document payload",
            )
            .expect("payload should be protected");
            fs::write(&payload_path, protected_payload).expect("payload should write");
            payload_path
        };

        let mut connection = open_database_with_key(&database_path, &encryption_key)
            .expect("database connection should open");
        {
            let mut project_repository = ProjectRepository::new(&mut connection);
            project_repository
                .create(&NewProject {
                    id: "prj_active_001".to_owned(),
                    name: "Active project".to_owned(),
                    description: None,
                    created_at: now,
                    updated_at: now,
                    last_opened_at: now,
                })
                .expect("project should be created");
            project_repository
                .open_project("prj_active_001", now + 1)
                .expect("project should become active");
        }
        {
            let mut document_repository = DocumentRepository::new(&mut connection);
            document_repository
                .create(&NewDocument {
                    id: "doc_1743517200_test".to_owned(),
                    project_id: "prj_active_001".to_owned(),
                    name: "source.txt".to_owned(),
                    source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                    format: "txt".to_owned(),
                    mime_type: Some("text/plain".to_owned()),
                    stored_path: stored_document_path.display().to_string(),
                    file_size_bytes: 24,
                    status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                    created_at: now,
                    updated_at: now,
                })
                .expect("document should be created");
        }
        {
            let mut segment_repository = SegmentRepository::new(&mut connection);
            segment_repository
                .replace_for_document(
                    "prj_active_001",
                    "doc_1743517200_test",
                    &[NewSegment {
                        id: "doc_1743517200_test_seg_00001".to_owned(),
                        document_id: "doc_1743517200_test".to_owned(),
                        sequence: 1,
                        source_text: "Legacy segment".to_owned(),
                        source_word_count: 2,
                        source_character_count: 14,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: now,
                        updated_at: now,
                    }],
                    now,
                )
                .expect("seed segment should be created");
        }
        drop(connection);

        let processed_document = process_project_document_with_runtime(
            ProcessDocumentInput {
                project_id: "prj_active_001".to_owned(),
                document_id: "doc_1743517200_test".to_owned(),
            },
            &runtime,
        )
        .expect("document processing should succeed");

        let mut reopened_connection = open_database_with_key(&database_path, &encryption_key)
            .expect("database connection should reopen");
        let segments = SegmentRepository::new(&mut reopened_connection)
            .list_by_document("doc_1743517200_test")
            .expect("segments should load");

        assert_eq!(processed_document.segment_count, 2);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source_text, "Alpha beta.");
        assert_eq!(segments[1].source_text, "Gamma delta.");
    }

    #[test]
    fn list_document_segments_returns_ordered_segments_for_segmented_document() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, &encryption_key)
            .expect("database connection should open");

        let project = NewProject {
            id: "prj_active_001".to_owned(),
            name: "Segment project".to_owned(),
            description: None,
            created_at: 1_743_517_200,
            updated_at: 1_743_517_200,
            last_opened_at: 1_743_517_200,
        };

        ProjectRepository::new(&mut connection)
            .create(&project)
            .expect("project should persist");

        let document = NewDocument {
            id: "doc_1743517200_test".to_owned(),
            project_id: project.id.clone(),
            name: "Segmented.txt".to_owned(),
            source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
            format: "txt".to_owned(),
            mime_type: Some("text/plain".to_owned()),
            stored_path: "ignored".to_owned(),
            file_size_bytes: 10,
            status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
            created_at: 1_743_517_200,
            updated_at: 1_743_517_200,
        };

        DocumentRepository::new(&mut connection)
            .create(&document)
            .expect("document should persist");

        SegmentRepository::new(&mut connection)
            .replace_for_document(
                &project.id,
                &document.id,
                &[
                    NewSegment {
                        id: "doc_1743517200_test_seg_00002".to_owned(),
                        document_id: document.id.clone(),
                        sequence: 2,
                        source_text: "Second".to_owned(),
                        source_word_count: 1,
                        source_character_count: 6,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: 1_743_517_201,
                        updated_at: 1_743_517_201,
                    },
                    NewSegment {
                        id: "doc_1743517200_test_seg_00001".to_owned(),
                        document_id: document.id.clone(),
                        sequence: 1,
                        source_text: "First".to_owned(),
                        source_word_count: 1,
                        source_character_count: 5,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: 1_743_517_201,
                        updated_at: 1_743_517_201,
                    },
                ],
                1_743_517_201,
            )
            .expect("segments should persist");

        drop(connection);

        let overview = list_document_segments_with_runtime(
            ListDocumentSegmentsInput {
                project_id: project.id,
                document_id: document.id,
            },
            &runtime,
        )
        .expect("segments should load");

        assert_eq!(overview.segments.len(), 2);
        assert_eq!(overview.segments[0].sequence, 1);
        assert_eq!(overview.segments[0].source_text, "First");
        assert_eq!(overview.segments[0].target_text, None);
        assert_eq!(overview.segments[1].sequence, 2);
    }
}
