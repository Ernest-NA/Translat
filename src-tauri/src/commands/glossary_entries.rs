use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use tauri::State;

use crate::error::DesktopCommandError;
use crate::glossary_entries::{
    CreateGlossaryEntryInput, GlossaryEntriesOverview, GlossaryEntryChanges, GlossaryEntrySummary,
    ListGlossaryEntriesInput, NewGlossaryEntry, UpdateGlossaryEntryInput,
    GLOSSARY_ENTRY_STATUS_ACTIVE, GLOSSARY_ENTRY_STATUS_ARCHIVED,
};
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::glossaries::GlossaryRepository;
use crate::persistence::glossary_entries::GlossaryEntryRepository;

const MAX_TERM_LENGTH: usize = 240;
const MAX_CONTEXT_NOTE_LENGTH: usize = 2000;
const MAX_VARIANT_COUNT_PER_BUCKET: usize = 24;

#[tauri::command]
pub fn list_glossary_entries(
    input: ListGlossaryEntriesInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<GlossaryEntriesOverview, DesktopCommandError> {
    list_glossary_entries_with_runtime(input, database_runtime.inner())
}

fn list_glossary_entries_with_runtime(
    input: ListGlossaryEntriesInput,
    database_runtime: &DatabaseRuntime,
) -> Result<GlossaryEntriesOverview, DesktopCommandError> {
    let glossary_id = validate_glossary_id(&input.glossary_id)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for glossary entry listing.",
            Some(error.to_string()),
        )
    })?;
    validate_glossary_exists(&glossary_id, &mut connection)?;
    let mut repository = GlossaryEntryRepository::new(&mut connection);

    repository.load_overview(&glossary_id).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not load the persisted glossary entries.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn create_glossary_entry(
    input: CreateGlossaryEntryInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<GlossaryEntrySummary, DesktopCommandError> {
    create_glossary_entry_with_runtime(input, database_runtime.inner())
}

fn create_glossary_entry_with_runtime(
    input: CreateGlossaryEntryInput,
    database_runtime: &DatabaseRuntime,
) -> Result<GlossaryEntrySummary, DesktopCommandError> {
    let timestamp = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for glossary entry creation.",
            Some(error.to_string()),
        )
    })?;
    let new_entry = validate_new_entry(input, &mut connection, timestamp)?;
    let mut repository = GlossaryEntryRepository::new(&mut connection);

    repository.create(&new_entry).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not create the requested glossary entry.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn update_glossary_entry(
    input: UpdateGlossaryEntryInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<GlossaryEntrySummary, DesktopCommandError> {
    update_glossary_entry_with_runtime(input, database_runtime.inner())
}

fn update_glossary_entry_with_runtime(
    input: UpdateGlossaryEntryInput,
    database_runtime: &DatabaseRuntime,
) -> Result<GlossaryEntrySummary, DesktopCommandError> {
    let updated_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for glossary entry updates.",
            Some(error.to_string()),
        )
    })?;
    let changes = validate_entry_changes(input, &mut connection, updated_at)?;
    let mut repository = GlossaryEntryRepository::new(&mut connection);

    repository.update(&changes).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not update the requested glossary entry.",
            Some(error.to_string()),
        )
    })
}

fn validate_new_entry(
    input: CreateGlossaryEntryInput,
    connection: &mut rusqlite::Connection,
    timestamp: i64,
) -> Result<NewGlossaryEntry, DesktopCommandError> {
    let glossary_id = validate_glossary_id(&input.glossary_id)?;
    validate_glossary_exists(&glossary_id, connection)?;

    Ok(NewGlossaryEntry {
        id: generate_glossary_entry_id(timestamp),
        glossary_id,
        source_term: validate_term(&input.source_term, "The source term is required.")?,
        target_term: validate_term(&input.target_term, "The target term is required.")?,
        context_note: normalize_context_note(input.context_note)?,
        status: GLOSSARY_ENTRY_STATUS_ACTIVE.to_owned(),
        created_at: timestamp,
        updated_at: timestamp,
        source_variants: normalize_variant_bucket(input.source_variants, "source")?,
        target_variants: normalize_variant_bucket(input.target_variants, "target")?,
        forbidden_terms: normalize_variant_bucket(input.forbidden_terms, "forbidden")?,
    })
}

fn validate_entry_changes(
    input: UpdateGlossaryEntryInput,
    connection: &mut rusqlite::Connection,
    updated_at: i64,
) -> Result<GlossaryEntryChanges, DesktopCommandError> {
    let glossary_id = validate_glossary_id(&input.glossary_id)?;
    validate_glossary_exists(&glossary_id, connection)?;

    Ok(GlossaryEntryChanges {
        glossary_entry_id: validate_glossary_entry_id(&input.glossary_entry_id)?,
        glossary_id,
        source_term: validate_term(&input.source_term, "The source term is required.")?,
        target_term: validate_term(&input.target_term, "The target term is required.")?,
        context_note: normalize_context_note(input.context_note)?,
        status: validate_status(&input.status)?,
        updated_at,
        source_variants: normalize_variant_bucket(input.source_variants, "source")?,
        target_variants: normalize_variant_bucket(input.target_variants, "target")?,
        forbidden_terms: normalize_variant_bucket(input.forbidden_terms, "forbidden")?,
    })
}

fn validate_glossary_exists(
    glossary_id: &str,
    connection: &mut rusqlite::Connection,
) -> Result<(), DesktopCommandError> {
    let glossary_exists = GlossaryRepository::new(connection)
        .exists(glossary_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not validate the glossary selected for terminology entries.",
                Some(error.to_string()),
            )
        })?;

    if !glossary_exists {
        return Err(DesktopCommandError::validation(
            "The selected glossary does not exist.",
            None,
        ));
    }

    Ok(())
}

fn validate_term(term: &str, empty_message: &str) -> Result<String, DesktopCommandError> {
    let normalized_term = term.trim();

    if normalized_term.is_empty() {
        return Err(DesktopCommandError::validation(empty_message, None));
    }

    if normalized_term.chars().count() > MAX_TERM_LENGTH {
        return Err(DesktopCommandError::validation(
            "Glossary terms must stay within 240 characters.",
            None,
        ));
    }

    Ok(normalized_term.to_owned())
}

fn normalize_context_note(
    context_note: Option<String>,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_context_note = context_note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    if let Some(context_note) = &normalized_context_note {
        if context_note.chars().count() > MAX_CONTEXT_NOTE_LENGTH {
            return Err(DesktopCommandError::validation(
                "The context note must stay within 2000 characters.",
                None,
            ));
        }
    }

    Ok(normalized_context_note)
}

fn normalize_variant_bucket(
    variants: Vec<String>,
    label: &str,
) -> Result<Vec<String>, DesktopCommandError> {
    if variants.len() > MAX_VARIANT_COUNT_PER_BUCKET * 2 {
        return Err(DesktopCommandError::validation(
            format!("Too many {label} terms were submitted at once."),
            None,
        ));
    }

    let mut seen = HashSet::new();
    let mut normalized_variants = Vec::new();

    for variant in variants {
        let normalized_variant = variant.trim();

        if normalized_variant.is_empty() {
            continue;
        }

        if normalized_variant.chars().count() > MAX_TERM_LENGTH {
            return Err(DesktopCommandError::validation(
                format!("Each {label} term must stay within 240 characters."),
                None,
            ));
        }

        let dedupe_key = normalized_variant.to_ascii_lowercase();

        if seen.insert(dedupe_key) {
            normalized_variants.push(normalized_variant.to_owned());
        }
    }

    if normalized_variants.len() > MAX_VARIANT_COUNT_PER_BUCKET {
        return Err(DesktopCommandError::validation(
            format!("Only {MAX_VARIANT_COUNT_PER_BUCKET} {label} terms are allowed per entry."),
            None,
        ));
    }

    Ok(normalized_variants)
}

fn validate_status(status: &str) -> Result<String, DesktopCommandError> {
    let normalized_status = status.trim().to_ascii_lowercase();

    match normalized_status.as_str() {
        GLOSSARY_ENTRY_STATUS_ACTIVE | GLOSSARY_ENTRY_STATUS_ARCHIVED => Ok(normalized_status),
        _ => Err(DesktopCommandError::validation(
            "The glossary entry status must be active or archived.",
            None,
        )),
    }
}

fn validate_glossary_id(glossary_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_glossary_id = glossary_id.trim();

    if trimmed_glossary_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The glossary entry request is missing a glossary id.",
            None,
        ));
    }

    validate_safe_identifier(
        trimmed_glossary_id,
        "The glossary entry request requires a safe persisted glossary id.",
    )?;

    Ok(trimmed_glossary_id.to_owned())
}

fn validate_glossary_entry_id(glossary_entry_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_glossary_entry_id = glossary_entry_id.trim();

    if trimmed_glossary_entry_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The glossary entry request is missing an entry id.",
            None,
        ));
    }

    validate_safe_identifier(
        trimmed_glossary_entry_id,
        "The glossary entry request requires a safe persisted glossary entry id.",
    )?;

    Ok(trimmed_glossary_entry_id.to_owned())
}

fn validate_safe_identifier(value: &str, message: &str) -> Result<(), DesktopCommandError> {
    if !value
        .chars()
        .all(|character| matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-'))
    {
        return Err(DesktopCommandError::validation(message, None));
    }

    Ok(())
}

fn generate_glossary_entry_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "gle_{}_{}",
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
                    "The desktop shell could not compute the current glossary entry timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid glossary entry timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::glossaries::{NewGlossary, GLOSSARY_STATUS_ACTIVE};
    use crate::glossary_entries::{
        CreateGlossaryEntryInput, ListGlossaryEntriesInput, UpdateGlossaryEntryInput,
        GLOSSARY_ENTRY_STATUS_ARCHIVED,
    };
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::glossaries::GlossaryRepository;
    use crate::persistence::secret_store::load_or_create_encryption_key;

    use super::{
        create_glossary_entry_with_runtime, list_glossary_entries_with_runtime,
        update_glossary_entry_with_runtime,
    };

    #[test]
    fn create_list_and_update_glossary_entries_end_to_end() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");
        let now = 1_775_401_200_i64;

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");
        let glossary = GlossaryRepository::new(&mut connection)
            .create(&NewGlossary {
                id: "gls_test_001".to_owned(),
                name: "Clinical core".to_owned(),
                description: Some("Base glossary for terminology tests.".to_owned()),
                project_id: None,
                status: GLOSSARY_STATUS_ACTIVE.to_owned(),
                created_at: now,
                updated_at: now,
                last_opened_at: now,
            })
            .expect("glossary should be created");
        drop(connection);

        let created_entry = create_glossary_entry_with_runtime(
            CreateGlossaryEntryInput {
                glossary_id: glossary.id.clone(),
                source_term: "black box warning".to_owned(),
                target_term: "advertencia de recuadro negro".to_owned(),
                context_note: Some("Use in regulatory documentation.".to_owned()),
                source_variants: vec!["boxed warning".to_owned()],
                target_variants: vec!["advertencia destacada".to_owned()],
                forbidden_terms: vec!["caja negra".to_owned()],
            },
            &runtime,
        )
        .expect("entry should be created");

        let updated_entry = update_glossary_entry_with_runtime(
            UpdateGlossaryEntryInput {
                glossary_entry_id: created_entry.id.clone(),
                glossary_id: glossary.id.clone(),
                source_term: "boxed warning".to_owned(),
                target_term: "advertencia destacada".to_owned(),
                context_note: Some("Updated preferred term.".to_owned()),
                source_variants: vec!["black box warning".to_owned(), "boxed warnings".to_owned()],
                target_variants: vec!["advertencia de recuadro".to_owned()],
                forbidden_terms: vec!["caja negra".to_owned(), "aviso negro".to_owned()],
                status: GLOSSARY_ENTRY_STATUS_ARCHIVED.to_owned(),
            },
            &runtime,
        )
        .expect("entry should update");

        let overview = list_glossary_entries_with_runtime(
            ListGlossaryEntriesInput {
                glossary_id: glossary.id,
            },
            &runtime,
        )
        .expect("entry overview should load");

        assert_eq!(overview.entries.len(), 1);
        assert_eq!(overview.entries[0].id, created_entry.id);
        assert_eq!(updated_entry.status, GLOSSARY_ENTRY_STATUS_ARCHIVED);
        assert_eq!(updated_entry.source_term, "boxed warning");
        assert_eq!(
            updated_entry.source_variants,
            vec!["black box warning".to_owned(), "boxed warnings".to_owned()]
        );
        assert_eq!(
            updated_entry.target_variants,
            vec!["advertencia de recuadro".to_owned()]
        );
        assert_eq!(
            updated_entry.forbidden_terms,
            vec!["aviso negro".to_owned(), "caja negra".to_owned()]
        );
    }

    #[test]
    fn creating_entries_requires_an_existing_glossary() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let error = create_glossary_entry_with_runtime(
            CreateGlossaryEntryInput {
                glossary_id: "gls_missing".to_owned(),
                source_term: "warning".to_owned(),
                target_term: "advertencia".to_owned(),
                context_note: None,
                source_variants: vec![],
                target_variants: vec![],
                forbidden_terms: vec![],
            },
            &runtime,
        )
        .expect_err("entry creation should fail without a glossary");

        assert_eq!(error.code, "INVALID_INPUT");
    }
}
