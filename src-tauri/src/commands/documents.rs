use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::documents::{
    DocumentSummary, ImportDocumentInput, ListProjectDocumentsInput, NewDocument,
    ProjectDocumentsOverview, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_IMPORTED,
    MAX_IMPORTED_DOCUMENT_BYTES,
};
use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::documents::{DocumentRepository, StoredDocumentRecord};
use crate::persistence::projects::ProjectRepository;
use crate::persistence::secret_store;

const PENDING_DOCUMENT_PREFIX: &str = "__pending__";
const ORPHAN_PENDING_GRACE_PERIOD_SECS: i64 = 300;
const MAX_STORAGE_FILE_NAME_CHARS: usize = 240;

#[derive(Debug, Clone, Deserialize)]
struct ValidatedDocumentImport {
    project_id: String,
    file_name: String,
    format: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct StoredDocumentPaths {
    final_path: PathBuf,
    pending_path: PathBuf,
}

#[tauri::command]
pub fn list_project_documents(
    input: ListProjectDocumentsInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<ProjectDocumentsOverview, DesktopCommandError> {
    list_project_documents_with_runtime(input, database_runtime.inner())
}

fn list_project_documents_with_runtime(
    input: ListProjectDocumentsInput,
    database_runtime: &DatabaseRuntime,
) -> Result<ProjectDocumentsOverview, DesktopCommandError> {
    let project_id = validate_project_id(&input.project_id)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for document listing.",
            Some(error.to_string()),
        )
    })?;

    ensure_project_exists(&mut connection, &project_id)?;
    ensure_project_is_active(&mut connection, &project_id)?;
    reconcile_project_document_storage(database_runtime, &mut connection, &project_id)?;

    let mut repository = DocumentRepository::new(&mut connection);

    repository.load_overview(&project_id).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not load the persisted documents for the selected project.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn import_project_document(
    input: ImportDocumentInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<DocumentSummary, DesktopCommandError> {
    import_project_document_with_runtime(input, database_runtime.inner())
}

pub(crate) fn import_project_document_with_runtime(
    input: ImportDocumentInput,
    database_runtime: &DatabaseRuntime,
) -> Result<DocumentSummary, DesktopCommandError> {
    let validated_import = validate_import_document(input)?;
    let imported_at = current_timestamp()?;
    let document_id = generate_document_id(imported_at);

    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for document import.",
            Some(error.to_string()),
        )
    })?;

    ensure_project_exists(&mut connection, &validated_import.project_id)?;
    ensure_project_is_active(&mut connection, &validated_import.project_id)?;
    reconcile_project_document_storage(
        database_runtime,
        &mut connection,
        &validated_import.project_id,
    )?;

    let stored_document_paths = persist_document_bytes(
        database_runtime,
        &validated_import.project_id,
        &document_id,
        &validated_import.file_name,
        &validated_import.bytes,
    )?;

    let new_document = NewDocument {
        id: document_id,
        project_id: validated_import.project_id.clone(),
        name: validated_import.file_name.clone(),
        source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
        format: validated_import.format,
        mime_type: validated_import.mime_type,
        stored_path: stored_document_paths.pending_path.display().to_string(),
        file_size_bytes: i64::try_from(validated_import.bytes.len()).map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell produced an invalid imported document size.",
                Some(error.to_string()),
            )
        })?,
        status: DOCUMENT_STATUS_IMPORTED.to_owned(),
        created_at: imported_at,
        updated_at: imported_at,
    };

    let mut repository = DocumentRepository::new(&mut connection);
    let created_document = repository.create(&new_document).map_err(|error| {
        best_effort_remove_file(&stored_document_paths.pending_path);

        DesktopCommandError::internal(
            "The desktop shell could not register the imported document.",
            Some(error.to_string()),
        )
    })?;

    if let Err(error) = finalize_stored_document(&stored_document_paths) {
        let rollback_error = repository.delete_by_id(&new_document.id).err();
        best_effort_remove_file(&stored_document_paths.pending_path);
        return Err(match rollback_error {
            Some(rollback_error) => DesktopCommandError::internal(
                "The desktop shell could not roll back a failed imported document finalization.",
                Some(format!(
                    "finalize error: {}; rollback error: {}",
                    error.message, rollback_error
                )),
            ),
            None => error,
        });
    }

    if let Err(error) = repository.update_stored_path(
        &new_document.id,
        &new_document.project_id,
        &stored_document_paths.final_path.display().to_string(),
        imported_at,
    ) {
        restore_pending_document_payload(&stored_document_paths);
        return Err(DesktopCommandError::internal(
            "The desktop shell could not finalize the imported document registration.",
            Some(error.to_string()),
        ));
    }

    Ok(created_document)
}

fn validate_project_id(project_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_project_id = project_id.trim();

    if trimmed_project_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The document flow requires a valid project id.",
            None,
        ));
    }

    if !trimmed_project_id
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(
            "The document flow requires a safe persisted project id.",
            None,
        ));
    }

    Ok(trimmed_project_id.to_owned())
}

fn validate_import_document(
    input: ImportDocumentInput,
) -> Result<ValidatedDocumentImport, DesktopCommandError> {
    let project_id = validate_project_id(&input.project_id)?;
    let normalized_file_name = normalize_file_name(&input.file_name)?;
    let mime_type = normalize_mime_type(input.mime_type)?;
    let normalized_base64_content = input.base64_content.trim();

    validate_base64_payload_size(normalized_base64_content)?;

    let bytes = STANDARD
        .decode(normalized_base64_content)
        .map_err(|error| {
            DesktopCommandError::validation(
                "The selected document payload could not be decoded.",
                Some(error.to_string()),
            )
        })?;

    if bytes.is_empty() {
        return Err(DesktopCommandError::validation(
            "The selected document is empty.",
            None,
        ));
    }

    if bytes.len() > MAX_IMPORTED_DOCUMENT_BYTES {
        return Err(DesktopCommandError::validation(
            "The selected document exceeds the current 20 MiB import limit for C2.",
            None,
        ));
    }

    Ok(ValidatedDocumentImport {
        project_id,
        format: validate_document_format(derive_document_format(&normalized_file_name))?,
        file_name: normalized_file_name,
        mime_type,
        bytes,
    })
}

fn normalize_file_name(file_name: &str) -> Result<String, DesktopCommandError> {
    let trimmed_file_name = file_name.trim();

    if trimmed_file_name.is_empty() {
        return Err(DesktopCommandError::validation(
            "The selected document is missing a file name.",
            None,
        ));
    }

    let normalized_file_name = Path::new(trimmed_file_name)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            DesktopCommandError::validation(
                "The selected document produced an invalid file name.",
                None,
            )
        })?
        .to_owned();

    if normalized_file_name.chars().count() > 255 {
        return Err(DesktopCommandError::validation(
            "The selected document name must stay within 255 characters.",
            None,
        ));
    }

    Ok(normalized_file_name)
}

fn normalize_mime_type(mime_type: Option<String>) -> Result<Option<String>, DesktopCommandError> {
    let normalized_mime_type = mime_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(mime_type) = &normalized_mime_type {
        if mime_type.chars().count() > 255 {
            return Err(DesktopCommandError::validation(
                "The selected document mime type must stay within 255 characters.",
                None,
            ));
        }
    }

    Ok(normalized_mime_type)
}

fn validate_base64_payload_size(base64_content: &str) -> Result<(), DesktopCommandError> {
    let estimated_decoded_length = estimate_base64_decoded_length(base64_content)?;

    if estimated_decoded_length > MAX_IMPORTED_DOCUMENT_BYTES {
        return Err(DesktopCommandError::validation(
            "The selected document exceeds the current 20 MiB import limit for C2.",
            None,
        ));
    }

    Ok(())
}

fn estimate_base64_decoded_length(base64_content: &str) -> Result<usize, DesktopCommandError> {
    if base64_content.is_empty() {
        return Ok(0);
    }

    let input_length = base64_content.len();
    let full_chunks = input_length / 4;
    let remainder = input_length % 4;
    let trailing_padding = base64_content
        .as_bytes()
        .iter()
        .rev()
        .take_while(|&&value| value == b'=')
        .take(2)
        .count();
    let base_length = full_chunks.checked_mul(3).ok_or_else(|| {
        DesktopCommandError::validation("The selected document payload could not be decoded.", None)
    })?;

    let estimated_length = match remainder {
        0 => base_length.checked_sub(trailing_padding).ok_or_else(|| {
            DesktopCommandError::validation(
                "The selected document payload could not be decoded.",
                None,
            )
        })?,
        2 => base_length + 1,
        3 => base_length + 2,
        _ => {
            return Err(DesktopCommandError::validation(
                "The selected document payload could not be decoded.",
                None,
            ));
        }
    };

    Ok(estimated_length)
}

fn derive_document_format(file_name: &str) -> String {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn validate_document_format(format: String) -> Result<String, DesktopCommandError> {
    let normalized_format = format.trim().to_owned();

    if normalized_format.is_empty() || normalized_format.chars().count() > 40 {
        return Err(DesktopCommandError::validation(
            "The selected document format must stay within 40 characters.",
            None,
        ));
    }

    Ok(normalized_format)
}

fn persist_document_bytes(
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    file_name: &str,
    bytes: &[u8],
) -> Result<StoredDocumentPaths, DesktopCommandError> {
    let documents_directory = database_runtime.documents_directory().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not resolve the document storage directory.",
            Some(error.to_string()),
        )
    })?;
    let project_directory = documents_directory.join(project_id);

    fs::create_dir_all(&project_directory).map_err(|error| {
        DesktopCommandError::internal(
            format!(
                "The desktop shell could not prepare the project document directory at {}.",
                project_directory.display()
            ),
            Some(error.to_string()),
        )
    })?;

    let stored_file_name = format!(
        "{document_id}__{}",
        sanitize_storage_file_name_for_storage(document_id, file_name)
    );
    let final_path = project_directory.join(stored_file_name);
    let pending_file_name = format!(
        "{}{}",
        PENDING_DOCUMENT_PREFIX,
        final_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("document")
    );
    let pending_path = project_directory.join(pending_file_name);

    let protected_bytes =
        secret_store::protect_local_payload(bytes, "Translat imported document payload").map_err(
            |error| {
                DesktopCommandError::internal(
            "The desktop shell could not protect the imported document payload for local storage.",
            Some(error.to_string()),
        )
            },
        )?;

    fs::write(&pending_path, protected_bytes).map_err(|error| {
        DesktopCommandError::internal(
            format!(
                "The desktop shell could not persist the imported document at {}.",
                pending_path.display()
            ),
            Some(error.to_string()),
        )
    })?;

    Ok(StoredDocumentPaths {
        final_path,
        pending_path,
    })
}

fn finalize_stored_document(
    stored_document_paths: &StoredDocumentPaths,
) -> Result<(), DesktopCommandError> {
    fs::rename(
        &stored_document_paths.pending_path,
        &stored_document_paths.final_path,
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            format!(
                "The desktop shell could not finalize the imported document payload at {}.",
                stored_document_paths.final_path.display()
            ),
            Some(error.to_string()),
        )
    })
}

pub(crate) fn reconcile_project_document_storage(
    database_runtime: &DatabaseRuntime,
    connection: &mut rusqlite::Connection,
    project_id: &str,
) -> Result<(), DesktopCommandError> {
    let documents_directory = database_runtime.documents_directory().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not resolve the document storage directory.",
            Some(error.to_string()),
        )
    })?;
    let project_directory = documents_directory.join(project_id);

    if !project_directory.exists() {
        return Ok(());
    }

    let now = current_timestamp()?;

    let mut repository = DocumentRepository::new(connection);
    let storage_records = repository
        .list_storage_records_by_project(project_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not inspect stored document records for cleanup.",
                Some(error.to_string()),
            )
        })?;
    let referenced_paths = repair_project_document_storage(&mut repository, storage_records, now)?
        .into_iter()
        .collect::<HashSet<_>>();

    let directory_entries = fs::read_dir(&project_directory).map_err(|error| {
        DesktopCommandError::internal(
            format!(
                "The desktop shell could not inspect the project document directory at {}.",
                project_directory.display()
            ),
            Some(error.to_string()),
        )
    })?;

    for entry in directory_entries {
        let entry = entry.map_err(|error| {
            DesktopCommandError::internal(
                format!(
                    "The desktop shell could not inspect a stored document entry under {}.",
                    project_directory.display()
                ),
                Some(error.to_string()),
            )
        })?;
        let entry_path = entry.path();

        if !entry_path.is_file() {
            continue;
        }

        if is_pending_document_payload(&entry_path) {
            if is_stale_pending_document_payload(&entry_path, now) {
                best_effort_remove_file(&entry_path);
            }
            continue;
        }

        if !referenced_paths.contains(&entry_path)
            && is_stale_unreferenced_document_payload(&entry_path, now)
        {
            best_effort_remove_file(&entry_path);
        }
    }

    Ok(())
}

fn repair_project_document_storage(
    repository: &mut DocumentRepository<'_>,
    storage_records: Vec<StoredDocumentRecord>,
    now: i64,
) -> Result<Vec<PathBuf>, DesktopCommandError> {
    let mut referenced_paths = Vec::new();

    for storage_record in storage_records {
        let stored_path = PathBuf::from(&storage_record.stored_path);

        if !is_pending_document_payload(&stored_path) {
            referenced_paths.push(stored_path);
            continue;
        }

        if !is_stale_pending_document_payload(&stored_path, now) {
            referenced_paths.push(stored_path);
            continue;
        }

        let final_path = final_path_from_pending(&stored_path)?;

        if final_path.exists() {
            repository
                .update_stored_path(
                    &storage_record.document_id,
                    project_id_from_stored_path(&final_path)?,
                    &final_path.display().to_string(),
                    now,
                )
                .map_err(|error| {
                    DesktopCommandError::internal(
                        "The desktop shell could not repair a pending imported document payload.",
                        Some(error.to_string()),
                    )
                })?;
            referenced_paths.push(final_path);
            continue;
        }

        if stored_path.exists() {
            let stored_document_paths = StoredDocumentPaths {
                final_path: final_path.clone(),
                pending_path: stored_path.clone(),
            };

            finalize_stored_document(&stored_document_paths)?;
        }

        if final_path.exists() {
            repository
                .update_stored_path(
                    &storage_record.document_id,
                    project_id_from_stored_path(&final_path)?,
                    &final_path.display().to_string(),
                    now,
                )
                .map_err(|error| {
                    DesktopCommandError::internal(
                        "The desktop shell could not repair a pending imported document payload.",
                        Some(error.to_string()),
                    )
                })?;
            referenced_paths.push(final_path);
        } else {
            referenced_paths.push(stored_path);
        }
    }

    Ok(referenced_paths)
}

fn is_pending_document_payload(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.starts_with(PENDING_DOCUMENT_PREFIX))
}

fn best_effort_remove_file(path: &Path) {
    let _ = fs::remove_file(path);
}

fn restore_pending_document_payload(stored_document_paths: &StoredDocumentPaths) {
    let _ = fs::rename(
        &stored_document_paths.final_path,
        &stored_document_paths.pending_path,
    );
}

fn final_path_from_pending(pending_path: &Path) -> Result<PathBuf, DesktopCommandError> {
    let file_name = pending_path
        .file_name()
        .and_then(|value| value.to_str())
        .and_then(|value| value.strip_prefix(PENDING_DOCUMENT_PREFIX))
        .ok_or_else(|| {
            DesktopCommandError::internal(
                "The desktop shell found a pending document payload with an invalid file name.",
                None,
            )
        })?;

    Ok(pending_path.with_file_name(file_name))
}

fn project_id_from_stored_path(path: &Path) -> Result<&str, DesktopCommandError> {
    path.parent()
        .and_then(|parent| parent.file_name())
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            DesktopCommandError::internal(
                "The desktop shell could not resolve the persisted project path for document storage.",
                None,
            )
        })
}

fn is_stale_unreferenced_document_payload(path: &Path, now: i64) -> bool {
    storage_payload_timestamp(path)
        .is_some_and(|timestamp| now.saturating_sub(timestamp) > ORPHAN_PENDING_GRACE_PERIOD_SECS)
}

fn is_stale_pending_document_payload(path: &Path, now: i64) -> bool {
    storage_payload_timestamp(path)
        .is_some_and(|timestamp| now.saturating_sub(timestamp) > ORPHAN_PENDING_GRACE_PERIOD_SECS)
}

fn storage_payload_timestamp(path: &Path) -> Option<i64> {
    let file_name = path.file_name().and_then(|value| value.to_str())?;
    let without_prefix = file_name
        .strip_prefix(PENDING_DOCUMENT_PREFIX)
        .unwrap_or(file_name);
    let without_doc_prefix = without_prefix.strip_prefix("doc_")?;
    let timestamp = without_doc_prefix.split('_').next()?;

    timestamp.parse::<i64>().ok()
}

fn sanitize_storage_file_name(file_name: &str) -> String {
    let candidate = Path::new(file_name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let sanitized: String = candidate
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => character,
            ' ' => '_',
            _ => '_',
        })
        .collect();
    let trimmed = sanitized.trim_matches('_').trim_matches('.');

    if trimmed.is_empty() {
        "document".to_owned()
    } else {
        let shortened: String = trimmed.chars().take(120).collect();

        if is_windows_reserved_file_name(&shortened) {
            format!("document_{shortened}")
        } else {
            shortened
        }
    }
}

fn sanitize_storage_file_name_for_storage(document_id: &str, file_name: &str) -> String {
    let available_name_chars = MAX_STORAGE_FILE_NAME_CHARS
        .saturating_sub(document_id.chars().count())
        .saturating_sub(2);
    let sanitized = sanitize_storage_file_name(file_name);
    let mut shortened: String = sanitized.chars().take(available_name_chars).collect();

    if shortened.is_empty() {
        shortened = "document".to_owned();
    }

    if is_windows_reserved_file_name(&shortened) {
        let prefixed = format!("document_{shortened}");
        prefixed.chars().take(available_name_chars).collect()
    } else {
        shortened
    }
}

fn is_windows_reserved_file_name(file_name: &str) -> bool {
    let stem = file_name
        .split('.')
        .next()
        .unwrap_or(file_name)
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();

    matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

fn ensure_project_exists(
    connection: &mut rusqlite::Connection,
    project_id: &str,
) -> Result<(), DesktopCommandError> {
    let mut repository = ProjectRepository::new(connection);
    let project_exists = repository.exists(project_id).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not validate the selected project for the document workflow.",
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
            "The desktop shell could not load the active project selection for the document workflow.",
            Some(error.to_string()),
        )
    })?;

    if active_project_id.as_deref() != Some(project_id) {
        return Err(DesktopCommandError::validation(
            "Documents can only be listed or imported for the currently open project.",
            None,
        ));
    }

    Ok(())
}

fn generate_document_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "doc_{}_{}",
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
                    "The desktop shell could not compute the current document timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid document timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        current_timestamp, derive_document_format, ensure_project_is_active,
        import_project_document_with_runtime, list_project_documents_with_runtime,
        reconcile_project_document_storage, sanitize_storage_file_name,
        validate_base64_payload_size, validate_document_format, validate_project_id,
        ORPHAN_PENDING_GRACE_PERIOD_SECS, PENDING_DOCUMENT_PREFIX,
    };
    use crate::documents::{
        ImportDocumentInput, ListProjectDocumentsInput, NewDocument, DOCUMENT_SOURCE_LOCAL_FILE,
        DOCUMENT_STATUS_IMPORTED, MAX_IMPORTED_DOCUMENT_BYTES,
    };
    use crate::persistence::bootstrap::{
        bootstrap_database, open_database_with_key, DatabaseRuntime,
    };
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::secret_store::{
        load_or_create_encryption_key, protect_local_payload, unprotect_local_payload,
    };
    use crate::projects::NewProject;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
    use base64::Engine;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn derive_document_format_uses_extension_when_available() {
        assert_eq!(derive_document_format("notes.final.DOCX"), "docx");
        assert_eq!(derive_document_format("README"), "unknown");
    }

    #[test]
    fn validate_document_format_rejects_extensions_longer_than_database_limit() {
        let long_format = "a".repeat(41);

        assert!(validate_document_format(long_format).is_err());
        assert_eq!(
            validate_document_format("docx".to_owned()).expect("format should be valid"),
            "docx"
        );
    }

    #[test]
    fn validate_base64_payload_size_rejects_oversized_inputs_before_decode() {
        let oversized_bytes = vec![0_u8; MAX_IMPORTED_DOCUMENT_BYTES + 1];
        let oversized_payload = BASE64_STANDARD.encode(oversized_bytes);
        let max_sized_payload = BASE64_STANDARD.encode(vec![0_u8; MAX_IMPORTED_DOCUMENT_BYTES]);

        assert!(validate_base64_payload_size(&oversized_payload).is_err());
        assert!(validate_base64_payload_size(&max_sized_payload).is_ok());
    }

    #[test]
    fn validate_project_id_rejects_unsafe_storage_segments() {
        assert_eq!(
            validate_project_id("prj_valid_001").expect("project id should be accepted"),
            "prj_valid_001"
        );
        assert!(validate_project_id("../escape").is_err());
        assert!(validate_project_id(r"..\escape").is_err());
        assert!(validate_project_id("C:/absolute").is_err());
    }

    #[test]
    fn ensure_project_is_active_rejects_non_active_project_ids() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, "translat-test-key-for-c2")
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, "translat-test-key-for-c2")
            .expect("database connection should open");
        {
            let mut repository = ProjectRepository::new(&mut connection);
            repository
                .create(&NewProject {
                    id: "prj_active_001".to_owned(),
                    name: "Active project".to_owned(),
                    description: None,
                    created_at: now,
                    updated_at: now,
                    last_opened_at: now,
                })
                .expect("active project should be created");
            repository
                .create(&NewProject {
                    id: "prj_other_001".to_owned(),
                    name: "Other project".to_owned(),
                    description: None,
                    created_at: now + 1,
                    updated_at: now + 1,
                    last_opened_at: now + 1,
                })
                .expect("other project should be created");
        }

        assert!(ensure_project_is_active(&mut connection, "prj_other_001").is_ok());
        {
            let mut repository = ProjectRepository::new(&mut connection);
            repository
                .open_project("prj_active_001", now + 2)
                .expect("active project should be reopened");
        }
        assert!(ensure_project_is_active(&mut connection, "prj_active_001").is_ok());
        assert!(ensure_project_is_active(&mut connection, "prj_other_001").is_err());
    }

    #[test]
    fn reconcile_project_document_storage_removes_orphaned_payloads() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path);
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, "translat-test-key-for-c2")
            .expect("database bootstrap should succeed");

        let documents_directory = runtime
            .documents_directory()
            .expect("documents directory should resolve");
        let project_directory = documents_directory.join("prj_active_001");
        fs::create_dir_all(&project_directory).expect("project directory should exist");

        let stale_timestamp = now - (ORPHAN_PENDING_GRACE_PERIOD_SECS + 60);
        let referenced_payload_path =
            project_directory.join(format!("doc_{stale_timestamp}_ref__source.txt"));
        let orphaned_payload_path =
            project_directory.join(format!("doc_{stale_timestamp}_orphan__source.txt"));
        fs::write(&referenced_payload_path, b"referenced")
            .expect("referenced payload should write");
        fs::write(&orphaned_payload_path, b"orphaned").expect("orphaned payload should write");

        let mut connection = open_database_with_key(&database_path, "translat-test-key-for-c2")
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
                .expect("active project should be created");
        }
        {
            let mut document_repository = DocumentRepository::new(&mut connection);
            document_repository
                .create(&NewDocument {
                    id: format!("doc_{stale_timestamp}_ref"),
                    project_id: "prj_active_001".to_owned(),
                    name: "source.txt".to_owned(),
                    source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                    format: "txt".to_owned(),
                    mime_type: Some("text/plain".to_owned()),
                    stored_path: referenced_payload_path.display().to_string(),
                    file_size_bytes: 10,
                    status: DOCUMENT_STATUS_IMPORTED.to_owned(),
                    created_at: now,
                    updated_at: now,
                })
                .expect("referenced document should be created");
        }

        reconcile_project_document_storage(&runtime, &mut connection, "prj_active_001")
            .expect("cleanup should succeed");

        assert!(referenced_payload_path.exists());
        assert!(!orphaned_payload_path.exists());
    }

    #[test]
    fn reconcile_project_document_storage_keeps_recent_pending_payloads() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path);
        let now = current_timestamp().expect("timestamp should be available");

        bootstrap_database(&database_path, "translat-test-key-for-c2")
            .expect("database bootstrap should succeed");

        let documents_directory = runtime
            .documents_directory()
            .expect("documents directory should resolve");
        let project_directory = documents_directory.join("prj_active_001");
        fs::create_dir_all(&project_directory).expect("project directory should exist");

        let pending_payload_path = project_directory.join(format!(
            "{}doc_{}_test__source.txt",
            PENDING_DOCUMENT_PREFIX, now
        ));
        fs::write(&pending_payload_path, b"pending").expect("pending payload should write");

        let mut connection = open_database_with_key(&database_path, "translat-test-key-for-c2")
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
                .expect("active project should be created");
        }

        reconcile_project_document_storage(&runtime, &mut connection, "prj_active_001")
            .expect("cleanup should succeed");

        assert!(pending_payload_path.exists());
    }

    #[test]
    fn reconcile_project_document_storage_keeps_recent_unreferenced_payloads() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path);
        let now = current_timestamp().expect("timestamp should be available");

        bootstrap_database(&database_path, "translat-test-key-for-c2")
            .expect("database bootstrap should succeed");

        let documents_directory = runtime
            .documents_directory()
            .expect("documents directory should resolve");
        let project_directory = documents_directory.join("prj_active_001");
        fs::create_dir_all(&project_directory).expect("project directory should exist");

        let recent_payload_path = project_directory.join(format!("doc_{now}_recent__source.txt"));
        fs::write(&recent_payload_path, b"recent").expect("recent payload should write");

        let mut connection = open_database_with_key(&database_path, "translat-test-key-for-c2")
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
                .expect("active project should be created");
        }

        reconcile_project_document_storage(&runtime, &mut connection, "prj_active_001")
            .expect("cleanup should succeed");

        assert!(recent_payload_path.exists());
    }

    #[test]
    fn reconcile_project_document_storage_keeps_real_documents_named_pending() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path);
        let now = current_timestamp().expect("timestamp should be available");

        bootstrap_database(&database_path, "translat-test-key-for-c2")
            .expect("database bootstrap should succeed");

        let documents_directory = runtime
            .documents_directory()
            .expect("documents directory should resolve");
        let project_directory = documents_directory.join("prj_active_001");
        fs::create_dir_all(&project_directory).expect("project directory should exist");

        let referenced_payload_path =
            project_directory.join(format!("doc_{now}_report__report.pending"));
        fs::write(&referenced_payload_path, b"report").expect("report payload should write");

        let mut connection = open_database_with_key(&database_path, "translat-test-key-for-c2")
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
                .expect("active project should be created");
        }
        {
            let mut document_repository = DocumentRepository::new(&mut connection);
            document_repository
                .create(&NewDocument {
                    id: format!("doc_{now}_report"),
                    project_id: "prj_active_001".to_owned(),
                    name: "report.pending".to_owned(),
                    source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                    format: "pending".to_owned(),
                    mime_type: Some("text/plain".to_owned()),
                    stored_path: referenced_payload_path.display().to_string(),
                    file_size_bytes: 6,
                    status: DOCUMENT_STATUS_IMPORTED.to_owned(),
                    created_at: now,
                    updated_at: now,
                })
                .expect("document should be created");
        }

        reconcile_project_document_storage(&runtime, &mut connection, "prj_active_001")
            .expect("cleanup should succeed");

        assert!(referenced_payload_path.exists());
    }

    #[test]
    fn reconcile_project_document_storage_ignores_unrelated_files() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path);
        let now = current_timestamp().expect("timestamp should be available");

        bootstrap_database(&database_path, "translat-test-key-for-c2")
            .expect("database bootstrap should succeed");

        let documents_directory = runtime
            .documents_directory()
            .expect("documents directory should resolve");
        let project_directory = documents_directory.join("prj_active_001");
        fs::create_dir_all(&project_directory).expect("project directory should exist");

        let unrelated_path = project_directory.join("desktop.ini");
        fs::write(&unrelated_path, b"metadata").expect("unrelated payload should write");

        let mut connection = open_database_with_key(&database_path, "translat-test-key-for-c2")
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
                .expect("active project should be created");
        }

        reconcile_project_document_storage(&runtime, &mut connection, "prj_active_001")
            .expect("cleanup should succeed");

        assert!(unrelated_path.exists());
    }

    #[test]
    fn import_project_document_registers_payload_and_metadata_end_to_end() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let now = 1_743_517_200_i64;
        let plaintext = b"Hello from C2 import";
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

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
                .expect("active project should be created");
            project_repository
                .open_project("prj_active_001", now + 1)
                .expect("project should become active");
        }
        drop(connection);

        let imported_document = import_project_document_with_runtime(
            ImportDocumentInput {
                project_id: "prj_active_001".to_owned(),
                file_name: "source.txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                base64_content: BASE64_STANDARD.encode(plaintext),
            },
            &runtime,
        )
        .expect("document import should succeed");

        assert_eq!(imported_document.project_id, "prj_active_001");
        assert_eq!(imported_document.name, "source.txt");
        assert_eq!(imported_document.format, "txt");
        assert_eq!(imported_document.file_size_bytes, plaintext.len() as i64);
        assert_eq!(imported_document.status, DOCUMENT_STATUS_IMPORTED);

        let mut reopened_connection = open_database_with_key(&database_path, &encryption_key)
            .expect("database connection should reopen");
        let mut document_repository = DocumentRepository::new(&mut reopened_connection);
        let overview = document_repository
            .load_overview("prj_active_001")
            .expect("overview should load");
        let imported_payload_paths = runtime
            .documents_directory()
            .expect("documents directory should resolve")
            .join("prj_active_001");
        let stored_entries = fs::read_dir(imported_payload_paths)
            .expect("stored directory should be readable")
            .collect::<Result<Vec<_>, _>>()
            .expect("stored entries should be readable");

        assert_eq!(overview.documents.len(), 1);
        assert_eq!(overview.documents[0], imported_document);
        assert_eq!(stored_entries.len(), 1);
        assert!(stored_entries[0].path().is_file());
    }

    #[test]
    fn list_project_documents_repairs_pending_payload_references() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let now = current_timestamp().expect("timestamp should be available");
        let stale_timestamp = now - (ORPHAN_PENDING_GRACE_PERIOD_SECS + 60);
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let documents_directory = runtime
            .documents_directory()
            .expect("documents directory should resolve");
        let project_directory = documents_directory.join("prj_active_001");
        fs::create_dir_all(&project_directory).expect("project directory should exist");
        let pending_payload_path = project_directory.join(format!(
            "{}doc_{stale_timestamp}_repair__source.txt",
            PENDING_DOCUMENT_PREFIX
        ));
        let final_payload_path =
            project_directory.join(format!("doc_{stale_timestamp}_repair__source.txt"));
        fs::write(&pending_payload_path, b"pending").expect("pending payload should write");

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
                .expect("active project should be created");
            project_repository
                .open_project("prj_active_001", now + 1)
                .expect("project should become active");
        }
        {
            let mut document_repository = DocumentRepository::new(&mut connection);
            document_repository
                .create(&NewDocument {
                    id: format!("doc_{stale_timestamp}_repair"),
                    project_id: "prj_active_001".to_owned(),
                    name: "source.txt".to_owned(),
                    source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                    format: "txt".to_owned(),
                    mime_type: Some("text/plain".to_owned()),
                    stored_path: pending_payload_path.display().to_string(),
                    file_size_bytes: 7,
                    status: DOCUMENT_STATUS_IMPORTED.to_owned(),
                    created_at: stale_timestamp,
                    updated_at: stale_timestamp,
                })
                .expect("document should be created");
        }
        drop(connection);

        let overview = list_project_documents_with_runtime(
            ListProjectDocumentsInput {
                project_id: "prj_active_001".to_owned(),
            },
            &runtime,
        )
        .expect("listing should repair pending storage");

        let mut reopened_connection = open_database_with_key(&database_path, &encryption_key)
            .expect("database connection should reopen");
        let repaired_paths = DocumentRepository::new(&mut reopened_connection)
            .list_stored_paths_by_project("prj_active_001")
            .expect("stored paths should load");

        assert_eq!(overview.documents.len(), 1);
        assert!(!pending_payload_path.exists());
        assert!(final_payload_path.exists());
        assert_eq!(
            repaired_paths,
            vec![final_payload_path.display().to_string()]
        );
    }

    #[test]
    fn list_project_documents_repairs_pending_rows_when_final_payload_already_exists() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let now = current_timestamp().expect("timestamp should be available");
        let stale_timestamp = now - (ORPHAN_PENDING_GRACE_PERIOD_SECS + 60);
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let documents_directory = runtime
            .documents_directory()
            .expect("documents directory should resolve");
        let project_directory = documents_directory.join("prj_active_001");
        fs::create_dir_all(&project_directory).expect("project directory should exist");
        let pending_payload_path = project_directory.join(format!(
            "{}doc_{stale_timestamp}_repair__source.txt",
            PENDING_DOCUMENT_PREFIX
        ));
        let final_payload_path =
            project_directory.join(format!("doc_{stale_timestamp}_repair__source.txt"));
        fs::write(&final_payload_path, b"finalized").expect("final payload should write");

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
                .expect("active project should be created");
            project_repository
                .open_project("prj_active_001", now + 1)
                .expect("project should become active");
        }
        {
            let mut document_repository = DocumentRepository::new(&mut connection);
            document_repository
                .create(&NewDocument {
                    id: format!("doc_{stale_timestamp}_repair"),
                    project_id: "prj_active_001".to_owned(),
                    name: "source.txt".to_owned(),
                    source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                    format: "txt".to_owned(),
                    mime_type: Some("text/plain".to_owned()),
                    stored_path: pending_payload_path.display().to_string(),
                    file_size_bytes: 9,
                    status: DOCUMENT_STATUS_IMPORTED.to_owned(),
                    created_at: stale_timestamp,
                    updated_at: stale_timestamp,
                })
                .expect("document should be created");
        }
        drop(connection);

        let overview = list_project_documents_with_runtime(
            ListProjectDocumentsInput {
                project_id: "prj_active_001".to_owned(),
            },
            &runtime,
        )
        .expect("listing should repair pending storage");

        let mut reopened_connection = open_database_with_key(&database_path, &encryption_key)
            .expect("database connection should reopen");
        let repaired_paths = DocumentRepository::new(&mut reopened_connection)
            .list_stored_paths_by_project("prj_active_001")
            .expect("stored paths should load");

        assert_eq!(overview.documents.len(), 1);
        assert!(!pending_payload_path.exists());
        assert!(final_payload_path.exists());
        assert_eq!(
            repaired_paths,
            vec![final_payload_path.display().to_string()]
        );
    }

    #[test]
    fn sanitize_storage_file_name_removes_unsafe_characters() {
        assert_eq!(
            sanitize_storage_file_name("chapter 01 (draft).txt"),
            "chapter_01__draft_.txt"
        );
        assert_eq!(sanitize_storage_file_name(""), "document");
        assert_eq!(sanitize_storage_file_name("CON.txt"), "document_CON.txt");
        assert_eq!(sanitize_storage_file_name("nul"), "document_nul");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn protected_document_payload_round_trips() {
        let plaintext = b"Highly sensitive source content.";
        let protected_payload =
            protect_local_payload(plaintext, "Translat imported document payload")
                .expect("payload should be protected");
        let unprotected_payload =
            unprotect_local_payload(&protected_payload).expect("payload should be readable");

        assert_ne!(protected_payload, plaintext);
        assert_eq!(unprotected_payload, plaintext);
    }
}
