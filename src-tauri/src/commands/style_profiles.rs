use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::error::DesktopCommandError;
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::style_profiles::StyleProfileRepository;
use crate::style_profiles::{
    CreateStyleProfileInput, NewStyleProfile, StyleProfileChanges, StyleProfileSummary,
    StyleProfilesOverview, UpdateStyleProfileInput, STYLE_PROFILE_FORMALITY_FORMAL,
    STYLE_PROFILE_FORMALITY_INFORMAL, STYLE_PROFILE_FORMALITY_NEUTRAL,
    STYLE_PROFILE_FORMALITY_SEMI_FORMAL, STYLE_PROFILE_STATUS_ACTIVE,
    STYLE_PROFILE_STATUS_ARCHIVED, STYLE_PROFILE_TONE_DIRECT, STYLE_PROFILE_TONE_NEUTRAL,
    STYLE_PROFILE_TONE_TECHNICAL, STYLE_PROFILE_TONE_WARM, STYLE_PROFILE_TREATMENT_IMPERSONAL,
    STYLE_PROFILE_TREATMENT_MIXED, STYLE_PROFILE_TREATMENT_TUTEO, STYLE_PROFILE_TREATMENT_USTED,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenStyleProfileInput {
    pub style_profile_id: String,
}

#[tauri::command]
pub fn list_style_profiles(
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<StyleProfilesOverview, DesktopCommandError> {
    list_style_profiles_with_runtime(database_runtime.inner())
}

fn list_style_profiles_with_runtime(
    database_runtime: &DatabaseRuntime,
) -> Result<StyleProfilesOverview, DesktopCommandError> {
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for style-profile listing.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = StyleProfileRepository::new(&mut connection);

    repository.load_overview().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not load the persisted style profiles.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn create_style_profile(
    input: CreateStyleProfileInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<StyleProfileSummary, DesktopCommandError> {
    create_style_profile_with_runtime(input, database_runtime.inner())
}

fn create_style_profile_with_runtime(
    input: CreateStyleProfileInput,
    database_runtime: &DatabaseRuntime,
) -> Result<StyleProfileSummary, DesktopCommandError> {
    let timestamp = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for style-profile creation.",
            Some(error.to_string()),
        )
    })?;
    let new_style_profile = validate_new_style_profile(input, timestamp)?;
    let mut repository = StyleProfileRepository::new(&mut connection);

    repository.create(&new_style_profile).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not create the requested style profile.",
            Some(error.to_string()),
        )
    })
}

#[tauri::command]
pub fn open_style_profile(
    input: OpenStyleProfileInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<StyleProfileSummary, DesktopCommandError> {
    open_style_profile_with_runtime(input, database_runtime.inner())
}

fn open_style_profile_with_runtime(
    input: OpenStyleProfileInput,
    database_runtime: &DatabaseRuntime,
) -> Result<StyleProfileSummary, DesktopCommandError> {
    let style_profile_id = validate_style_profile_id(&input.style_profile_id)?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for style-profile selection.",
            Some(error.to_string()),
        )
    })?;
    let mut repository = StyleProfileRepository::new(&mut connection);

    repository
        .open_style_profile(&style_profile_id, current_timestamp()?)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not open the requested style profile.",
                Some(error.to_string()),
            )
        })
}

#[tauri::command]
pub fn update_style_profile(
    input: UpdateStyleProfileInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<StyleProfileSummary, DesktopCommandError> {
    update_style_profile_with_runtime(input, database_runtime.inner())
}

fn update_style_profile_with_runtime(
    input: UpdateStyleProfileInput,
    database_runtime: &DatabaseRuntime,
) -> Result<StyleProfileSummary, DesktopCommandError> {
    let updated_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for style-profile updates.",
            Some(error.to_string()),
        )
    })?;
    let changes = validate_style_profile_changes(input, updated_at)?;
    let mut repository = StyleProfileRepository::new(&mut connection);

    repository.update(&changes).map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not update the requested style profile.",
            Some(error.to_string()),
        )
    })
}

fn validate_new_style_profile(
    input: CreateStyleProfileInput,
    timestamp: i64,
) -> Result<NewStyleProfile, DesktopCommandError> {
    Ok(NewStyleProfile {
        id: generate_style_profile_id(timestamp),
        name: validate_style_profile_name(&input.name)?,
        description: normalize_optional_text(
            input.description,
            1000,
            "The style-profile description must stay within 1000 characters.",
        )?,
        tone: validate_tone(&input.tone)?,
        formality: validate_formality(&input.formality)?,
        treatment_preference: validate_treatment_preference(&input.treatment_preference)?,
        consistency_instructions: normalize_optional_text(
            input.consistency_instructions,
            2000,
            "The style-profile consistency instructions must stay within 2000 characters.",
        )?,
        editorial_notes: normalize_optional_text(
            input.editorial_notes,
            2000,
            "The style-profile editorial notes must stay within 2000 characters.",
        )?,
        status: STYLE_PROFILE_STATUS_ACTIVE.to_owned(),
        created_at: timestamp,
        updated_at: timestamp,
        last_opened_at: timestamp,
    })
}

fn validate_style_profile_changes(
    input: UpdateStyleProfileInput,
    updated_at: i64,
) -> Result<StyleProfileChanges, DesktopCommandError> {
    Ok(StyleProfileChanges {
        style_profile_id: validate_style_profile_id(&input.style_profile_id)?,
        name: validate_style_profile_name(&input.name)?,
        description: normalize_optional_text(
            input.description,
            1000,
            "The style-profile description must stay within 1000 characters.",
        )?,
        tone: validate_tone(&input.tone)?,
        formality: validate_formality(&input.formality)?,
        treatment_preference: validate_treatment_preference(&input.treatment_preference)?,
        consistency_instructions: normalize_optional_text(
            input.consistency_instructions,
            2000,
            "The style-profile consistency instructions must stay within 2000 characters.",
        )?,
        editorial_notes: normalize_optional_text(
            input.editorial_notes,
            2000,
            "The style-profile editorial notes must stay within 2000 characters.",
        )?,
        status: validate_status(&input.status)?,
        updated_at,
    })
}

fn validate_style_profile_name(name: &str) -> Result<String, DesktopCommandError> {
    let trimmed_name = name.trim();

    if trimmed_name.is_empty() {
        return Err(DesktopCommandError::validation(
            "The style-profile name is required.",
            None,
        ));
    }

    if trimmed_name.chars().count() > 120 {
        return Err(DesktopCommandError::validation(
            "The style-profile name must stay within 120 characters.",
            None,
        ));
    }

    Ok(trimmed_name.to_owned())
}

fn validate_style_profile_id(style_profile_id: &str) -> Result<String, DesktopCommandError> {
    let trimmed_style_profile_id = style_profile_id.trim();

    if trimmed_style_profile_id.is_empty() {
        return Err(DesktopCommandError::validation(
            "The style-profile selection is missing a valid style-profile id.",
            None,
        ));
    }

    validate_safe_identifier(
        trimmed_style_profile_id,
        "The style-profile selection requires a safe persisted style-profile id.",
    )?;

    Ok(trimmed_style_profile_id.to_owned())
}

fn normalize_optional_text(
    value: Option<String>,
    max_length: usize,
    message: &str,
) -> Result<Option<String>, DesktopCommandError> {
    let normalized_value = value
        .as_deref()
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(ToOwned::to_owned);

    if let Some(text) = &normalized_value {
        if text.chars().count() > max_length {
            return Err(DesktopCommandError::validation(message, None));
        }
    }

    Ok(normalized_value)
}

fn validate_tone(tone: &str) -> Result<String, DesktopCommandError> {
    let normalized_tone = tone.trim().to_ascii_lowercase();

    match normalized_tone.as_str() {
        STYLE_PROFILE_TONE_NEUTRAL
        | STYLE_PROFILE_TONE_DIRECT
        | STYLE_PROFILE_TONE_WARM
        | STYLE_PROFILE_TONE_TECHNICAL => Ok(normalized_tone),
        _ => Err(DesktopCommandError::validation(
            "The style-profile tone must be neutral, direct, warm, or technical.",
            None,
        )),
    }
}

fn validate_formality(formality: &str) -> Result<String, DesktopCommandError> {
    let normalized_formality = formality.trim().to_ascii_lowercase();

    match normalized_formality.as_str() {
        STYLE_PROFILE_FORMALITY_FORMAL
        | STYLE_PROFILE_FORMALITY_NEUTRAL
        | STYLE_PROFILE_FORMALITY_SEMI_FORMAL
        | STYLE_PROFILE_FORMALITY_INFORMAL => Ok(normalized_formality),
        _ => Err(DesktopCommandError::validation(
            "The style-profile formality must be formal, neutral, semi_formal, or informal.",
            None,
        )),
    }
}

fn validate_treatment_preference(
    treatment_preference: &str,
) -> Result<String, DesktopCommandError> {
    let normalized_treatment_preference = treatment_preference.trim().to_ascii_lowercase();

    match normalized_treatment_preference.as_str() {
        STYLE_PROFILE_TREATMENT_USTED
        | STYLE_PROFILE_TREATMENT_TUTEO
        | STYLE_PROFILE_TREATMENT_IMPERSONAL
        | STYLE_PROFILE_TREATMENT_MIXED => Ok(normalized_treatment_preference),
        _ => Err(DesktopCommandError::validation(
            "The style-profile treatment preference must be usted, tuteo, impersonal, or mixed.",
            None,
        )),
    }
}

fn validate_status(status: &str) -> Result<String, DesktopCommandError> {
    let normalized_status = status.trim().to_ascii_lowercase();

    match normalized_status.as_str() {
        STYLE_PROFILE_STATUS_ACTIVE | STYLE_PROFILE_STATUS_ARCHIVED => Ok(normalized_status),
        _ => Err(DesktopCommandError::validation(
            "The style-profile status must be active or archived.",
            None,
        )),
    }
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

fn generate_style_profile_id(timestamp: i64) -> String {
    let random_part = rand::random::<u64>();

    format!(
        "stp_{}_{}",
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
                    "The desktop shell could not compute the current style-profile timestamp.",
                    Some(error.to_string()),
                )
            })?
            .as_secs(),
    )
    .map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell produced an invalid style-profile timestamp size.",
            Some(error.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::style_profiles::{
        CreateStyleProfileInput, UpdateStyleProfileInput, STYLE_PROFILE_FORMALITY_FORMAL,
        STYLE_PROFILE_FORMALITY_NEUTRAL, STYLE_PROFILE_STATUS_ARCHIVED, STYLE_PROFILE_TONE_DIRECT,
        STYLE_PROFILE_TONE_TECHNICAL, STYLE_PROFILE_TREATMENT_TUTEO, STYLE_PROFILE_TREATMENT_USTED,
    };

    use super::{
        create_style_profile_with_runtime, list_style_profiles_with_runtime,
        open_style_profile_with_runtime, update_style_profile_with_runtime, OpenStyleProfileInput,
    };

    #[test]
    fn create_open_and_update_style_profile_end_to_end() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
        let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
        let encryption_key = load_or_create_encryption_key(&encryption_key_path)
            .expect("encryption key should be created");

        bootstrap_database(&database_path, &encryption_key)
            .expect("database bootstrap should succeed");

        let created_style_profile = create_style_profile_with_runtime(
            CreateStyleProfileInput {
                name: "Medical ES baseline".to_owned(),
                description: Some("Reusable editorial base for medical Spanish.".to_owned()),
                tone: STYLE_PROFILE_TONE_TECHNICAL.to_owned(),
                formality: STYLE_PROFILE_FORMALITY_FORMAL.to_owned(),
                treatment_preference: STYLE_PROFILE_TREATMENT_USTED.to_owned(),
                consistency_instructions: Some(
                    "Keep recurring labels and safety terminology stable.".to_owned(),
                ),
                editorial_notes: Some("Avoid promotional phrasing.".to_owned()),
            },
            &runtime,
        )
        .expect("style profile should be created");

        let opened_style_profile = open_style_profile_with_runtime(
            OpenStyleProfileInput {
                style_profile_id: created_style_profile.id.clone(),
            },
            &runtime,
        )
        .expect("style profile should open");

        let updated_style_profile = update_style_profile_with_runtime(
            UpdateStyleProfileInput {
                style_profile_id: created_style_profile.id.clone(),
                name: "Medical ES modernized".to_owned(),
                description: Some("Updated for warmer patient-facing copy.".to_owned()),
                tone: STYLE_PROFILE_TONE_DIRECT.to_owned(),
                formality: STYLE_PROFILE_FORMALITY_NEUTRAL.to_owned(),
                treatment_preference: STYLE_PROFILE_TREATMENT_TUTEO.to_owned(),
                consistency_instructions: Some(
                    "Keep recurring labels stable across warnings and forms.".to_owned(),
                ),
                editorial_notes: Some("Use plain language when possible.".to_owned()),
                status: STYLE_PROFILE_STATUS_ARCHIVED.to_owned(),
            },
            &runtime,
        )
        .expect("style profile should update");

        let overview =
            list_style_profiles_with_runtime(&runtime).expect("style profile overview should load");

        assert_eq!(opened_style_profile.id, created_style_profile.id);
        assert_eq!(updated_style_profile.id, created_style_profile.id);
        assert_eq!(updated_style_profile.name, "Medical ES modernized");
        assert_eq!(updated_style_profile.status, STYLE_PROFILE_STATUS_ARCHIVED);
        assert_eq!(
            overview.active_style_profile_id,
            Some(created_style_profile.id.clone())
        );
        assert_eq!(overview.style_profiles.len(), 1);
        assert_eq!(
            overview.style_profiles[0].treatment_preference,
            STYLE_PROFILE_TREATMENT_TUTEO
        );
    }
}
