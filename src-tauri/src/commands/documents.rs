use std::fs;
use std::path::Path;
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
use crate::persistence::documents::DocumentRepository;
use crate::persistence::projects::ProjectRepository;

#[derive(Debug, Clone, Deserialize)]
struct ValidatedDocumentImport {
    project_id: String,
    file_name: String,
    format: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
}

#[tauri::command]
pub fn list_project_documents(
    input: ListProjectDocumentsInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<ProjectDocumentsOverview, DesktopCommandError> {
    let project_id = validate_project_id(&input.project_id)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for document listing.",
            Some(error.to_string()),
        )
    })?;

    ensure_project_exists(&mut connection, &project_id)?;

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
    let validated_import = validate_import_document(input)?;
    let imported_at = current_timestamp()?;
    let document_id = generate_document_id(imported_at);
    let stored_document_path = persist_document_bytes(
        &database_runtime,
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
        stored_path: stored_document_path.display().to_string(),
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

    let mut connection = database_runtime.open_connection().map_err(|error| {
        let _ = fs::remove_file(&stored_document_path);

        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for document import.",
            Some(error.to_string()),
        )
    })?;

    if let Err(error) = ensure_project_exists(&mut connection, &new_document.project_id) {
        let _ = fs::remove_file(&stored_document_path);
        return Err(error);
    }

    let mut repository = DocumentRepository::new(&mut connection);

    match repository.create(&new_document) {
        Ok(document) => Ok(document),
        Err(error) => {
            let _ = fs::remove_file(&stored_document_path);

            Err(DesktopCommandError::internal(
                "The desktop shell could not register the imported document.",
                Some(error.to_string()),
            ))
        }
    }
}

fn validate_project_id(project_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_project_id = project_id.trim();

    if trimmed_project_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The document flow requires a valid project id.",
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
    let bytes = STANDARD.decode(input.base64_content.trim()).map_err(|error| {
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
        format: derive_document_format(&normalized_file_name),
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

fn normalize_mime_type(
    mime_type: Option<String>,
) -> Result<Option<String>, DesktopCommandError> {
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

fn derive_document_format(file_name: &str) -> String {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn persist_document_bytes(
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    file_name: &str,
    bytes: &[u8],
) -> Result<std::path::PathBuf, DesktopCommandError> {
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

    let stored_file_name = format!("{document_id}__{}", sanitize_storage_file_name(file_name));
    let stored_document_path = project_directory.join(stored_file_name);

    fs::write(&stored_document_path, bytes).map_err(|error| {
        DesktopCommandError::internal(
            format!(
                "The desktop shell could not persist the imported document at {}.",
                stored_document_path.display()
            ),
            Some(error.to_string()),
        )
    })?;

    Ok(stored_document_path)
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
        trimmed.chars().take(120).collect()
    }
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
    use super::{derive_document_format, sanitize_storage_file_name};

    #[test]
    fn derive_document_format_uses_extension_when_available() {
        assert_eq!(derive_document_format("notes.final.DOCX"), "docx");
        assert_eq!(derive_document_format("README"), "unknown");
    }

    #[test]
    fn sanitize_storage_file_name_removes_unsafe_characters() {
        assert_eq!(
            sanitize_storage_file_name("chapter 01 (draft).txt"),
            "chapter_01__draft_.txt"
        );
        assert_eq!(sanitize_storage_file_name(""), "document");
    }
}
