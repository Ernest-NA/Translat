use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::persistence::error::PersistenceError;
use crate::style_profiles::{
    NewStyleProfile, StyleProfileChanges, StyleProfileSummary, StyleProfilesOverview,
    ACTIVE_STYLE_PROFILE_METADATA_KEY,
};

pub struct StyleProfileRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> StyleProfileRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn create(
        &mut self,
        new_style_profile: &NewStyleProfile,
    ) -> Result<StyleProfileSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The style-profile repository could not start the style-profile creation transaction.",
                error,
            )
        })?;

        transaction
            .execute(
                r#"
                INSERT INTO style_profiles (
                  id,
                  name,
                  description,
                  tone,
                  formality,
                  treatment_preference,
                  consistency_instructions,
                  editorial_notes,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
                params![
                    new_style_profile.id,
                    new_style_profile.name,
                    new_style_profile.description,
                    new_style_profile.tone,
                    new_style_profile.formality,
                    new_style_profile.treatment_preference,
                    new_style_profile.consistency_instructions,
                    new_style_profile.editorial_notes,
                    new_style_profile.status,
                    new_style_profile.created_at,
                    new_style_profile.updated_at,
                    new_style_profile.last_opened_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The style-profile repository could not persist the new style profile.",
                    error,
                )
            })?;

        upsert_active_style_profile(
            &transaction,
            &new_style_profile.id,
            new_style_profile.updated_at,
        )?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The style-profile repository could not commit the style-profile creation transaction.",
                error,
            )
        })?;

        Ok(StyleProfileSummary {
            id: new_style_profile.id.clone(),
            name: new_style_profile.name.clone(),
            description: new_style_profile.description.clone(),
            tone: new_style_profile.tone.clone(),
            formality: new_style_profile.formality.clone(),
            treatment_preference: new_style_profile.treatment_preference.clone(),
            consistency_instructions: new_style_profile.consistency_instructions.clone(),
            editorial_notes: new_style_profile.editorial_notes.clone(),
            status: new_style_profile.status.clone(),
            created_at: new_style_profile.created_at,
            updated_at: new_style_profile.updated_at,
            last_opened_at: new_style_profile.last_opened_at,
        })
    }

    pub fn list(&mut self) -> Result<Vec<StyleProfileSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  tone,
                  formality,
                  treatment_preference,
                  consistency_instructions,
                  editorial_notes,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM style_profiles
                ORDER BY
                  CASE status WHEN 'active' THEN 0 ELSE 1 END ASC,
                  last_opened_at DESC,
                  created_at DESC,
                  name COLLATE NOCASE ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The style-profile repository could not prepare the style-profile listing query.",
                    error,
                )
            })?;

        let rows = statement
            .query_map([], map_style_profile_summary)
            .map_err(|error| {
                PersistenceError::with_details(
                    "The style-profile repository could not read the style-profile list.",
                    error,
                )
            })?;

        let mut style_profiles = Vec::new();

        for row in rows {
            style_profiles.push(row.map_err(|error| {
                PersistenceError::with_details(
                    "The style-profile repository could not decode a style-profile row.",
                    error,
                )
            })?);
        }

        Ok(style_profiles)
    }

    pub fn open_style_profile(
        &mut self,
        style_profile_id: &str,
        opened_at: i64,
    ) -> Result<StyleProfileSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The style-profile repository could not start the style-profile opening transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                "UPDATE style_profiles SET last_opened_at = ?2 WHERE id = ?1",
                params![style_profile_id, opened_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The style-profile repository could not mark style profile {style_profile_id} as opened."
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested style profile {style_profile_id} does not exist."
            )));
        }

        upsert_active_style_profile(&transaction, style_profile_id, opened_at)?;

        let style_profile = transaction
            .query_row(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  tone,
                  formality,
                  treatment_preference,
                  consistency_instructions,
                  editorial_notes,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM style_profiles
                WHERE id = ?1
                "#,
                [style_profile_id],
                map_style_profile_summary,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The style-profile repository could not reload style profile {style_profile_id}."
                    ),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The style-profile repository could not commit the style-profile opening transaction.",
                error,
            )
        })?;

        Ok(style_profile)
    }

    pub fn update(
        &mut self,
        changes: &StyleProfileChanges,
    ) -> Result<StyleProfileSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The style-profile repository could not start the style-profile update transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                r#"
                UPDATE style_profiles
                SET
                  name = ?2,
                  description = ?3,
                  tone = ?4,
                  formality = ?5,
                  treatment_preference = ?6,
                  consistency_instructions = ?7,
                  editorial_notes = ?8,
                  status = ?9,
                  updated_at = ?10
                WHERE id = ?1
                "#,
                params![
                    changes.style_profile_id,
                    changes.name,
                    changes.description,
                    changes.tone,
                    changes.formality,
                    changes.treatment_preference,
                    changes.consistency_instructions,
                    changes.editorial_notes,
                    changes.status,
                    changes.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The style-profile repository could not update style profile {}.",
                        changes.style_profile_id
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested style profile {} does not exist.",
                changes.style_profile_id
            )));
        }

        upsert_active_style_profile(&transaction, &changes.style_profile_id, changes.updated_at)?;

        let style_profile = transaction
            .query_row(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  tone,
                  formality,
                  treatment_preference,
                  consistency_instructions,
                  editorial_notes,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM style_profiles
                WHERE id = ?1
                "#,
                [&changes.style_profile_id],
                map_style_profile_summary,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The style-profile repository could not reload style profile {}.",
                        changes.style_profile_id
                    ),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The style-profile repository could not commit the style-profile update transaction.",
                error,
            )
        })?;

        Ok(style_profile)
    }

    pub fn load_overview(&mut self) -> Result<StyleProfilesOverview, PersistenceError> {
        Ok(StyleProfilesOverview {
            active_style_profile_id: self.active_style_profile_id()?,
            style_profiles: self.list()?,
        })
    }

    pub fn exists(&mut self, style_profile_id: &str) -> Result<bool, PersistenceError> {
        let row = self
            .connection
            .query_row(
                "SELECT 1 FROM style_profiles WHERE id = ?1 LIMIT 1",
                [style_profile_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The style-profile repository could not inspect style profile {style_profile_id}."
                    ),
                    error,
                )
            })?;

        Ok(row.is_some())
    }

    fn active_style_profile_id(&mut self) -> Result<Option<String>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                [ACTIVE_STYLE_PROFILE_METADATA_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    "The style-profile repository could not load the active style-profile id.",
                    error,
                )
            })
    }
}

fn upsert_active_style_profile(
    connection: &Connection,
    style_profile_id: &str,
    timestamp: i64,
) -> Result<(), PersistenceError> {
    connection
        .execute(
            r#"
            INSERT INTO app_metadata (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
              value = excluded.value,
              updated_at = excluded.updated_at
            "#,
            params![ACTIVE_STYLE_PROFILE_METADATA_KEY, style_profile_id, timestamp],
        )
        .map_err(|error| {
            PersistenceError::with_details(
                "The style-profile repository could not persist the active style-profile selection.",
                error,
            )
        })?;

    Ok(())
}

fn map_style_profile_summary(row: &Row<'_>) -> rusqlite::Result<StyleProfileSummary> {
    Ok(StyleProfileSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        tone: row.get(3)?,
        formality: row.get(4)?,
        treatment_preference: row.get(5)?,
        consistency_instructions: row.get(6)?,
        editorial_notes: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        last_opened_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::StyleProfileRepository;
    use tempfile::tempdir;

    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::style_profiles::{
        NewStyleProfile, StyleProfileChanges, STYLE_PROFILE_FORMALITY_FORMAL,
        STYLE_PROFILE_FORMALITY_SEMI_FORMAL, STYLE_PROFILE_STATUS_ACTIVE,
        STYLE_PROFILE_STATUS_ARCHIVED, STYLE_PROFILE_TONE_NEUTRAL, STYLE_PROFILE_TONE_WARM,
        STYLE_PROFILE_TREATMENT_TUTEO, STYLE_PROFILE_TREATMENT_USTED,
    };

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-d3";

    fn sample_style_profile(now: i64) -> NewStyleProfile {
        NewStyleProfile {
            id: "stp_test_001".to_owned(),
            name: "Clinical neutral".to_owned(),
            description: Some("Reusable style profile for clinical materials.".to_owned()),
            tone: STYLE_PROFILE_TONE_NEUTRAL.to_owned(),
            formality: STYLE_PROFILE_FORMALITY_FORMAL.to_owned(),
            treatment_preference: STYLE_PROFILE_TREATMENT_USTED.to_owned(),
            consistency_instructions: Some("Keep recurring warnings and labels stable.".to_owned()),
            editorial_notes: Some("Avoid marketing language.".to_owned()),
            status: STYLE_PROFILE_STATUS_ACTIVE.to_owned(),
            created_at: now,
            updated_at: now,
            last_opened_at: now,
        }
    }

    #[test]
    fn create_and_list_style_profiles_round_trip() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_775_488_800_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        let mut repository = StyleProfileRepository::new(&mut connection);
        let created_style_profile = repository
            .create(&sample_style_profile(now))
            .expect("style profile should be created");
        let overview = repository
            .load_overview()
            .expect("style profile overview should load");

        assert_eq!(created_style_profile.id, "stp_test_001");
        assert_eq!(
            overview.active_style_profile_id,
            Some("stp_test_001".to_owned())
        );
        assert_eq!(overview.style_profiles, vec![created_style_profile]);
    }

    #[test]
    fn style_profile_updates_and_survives_reopen() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let created_at = 1_775_488_800_i64;
        let reopened_at = created_at + 300;
        let updated_at = reopened_at + 60;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        {
            let mut first_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
                .expect("database connection should open");
            let mut repository = StyleProfileRepository::new(&mut first_connection);
            repository
                .create(&sample_style_profile(created_at))
                .expect("style profile should be created");
            repository
                .open_style_profile("stp_test_001", reopened_at)
                .expect("style profile should reopen");
            repository
                .update(&StyleProfileChanges {
                    style_profile_id: "stp_test_001".to_owned(),
                    name: "Clinical warm".to_owned(),
                    description: Some("Updated for patient-facing content.".to_owned()),
                    tone: STYLE_PROFILE_TONE_WARM.to_owned(),
                    formality: STYLE_PROFILE_FORMALITY_SEMI_FORMAL.to_owned(),
                    treatment_preference: STYLE_PROFILE_TREATMENT_TUTEO.to_owned(),
                    consistency_instructions: Some(
                        "Keep terminology stable across discharge instructions.".to_owned(),
                    ),
                    editorial_notes: Some("Prefer plain-language phrasing.".to_owned()),
                    status: STYLE_PROFILE_STATUS_ARCHIVED.to_owned(),
                    updated_at,
                })
                .expect("style profile should update");
        }

        let mut second_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should reopen");
        let mut repository = StyleProfileRepository::new(&mut second_connection);
        let overview = repository
            .load_overview()
            .expect("style profile overview should reload");

        assert_eq!(
            overview.active_style_profile_id,
            Some("stp_test_001".to_owned())
        );
        assert_eq!(overview.style_profiles.len(), 1);
        assert_eq!(overview.style_profiles[0].name, "Clinical warm");
        assert_eq!(overview.style_profiles[0].tone, STYLE_PROFILE_TONE_WARM);
        assert_eq!(
            overview.style_profiles[0].formality,
            STYLE_PROFILE_FORMALITY_SEMI_FORMAL
        );
        assert_eq!(
            overview.style_profiles[0].treatment_preference,
            STYLE_PROFILE_TREATMENT_TUTEO
        );
        assert_eq!(
            overview.style_profiles[0].status,
            STYLE_PROFILE_STATUS_ARCHIVED
        );
        assert_eq!(overview.style_profiles[0].last_opened_at, reopened_at);
        assert_eq!(overview.style_profiles[0].updated_at, updated_at);
    }
}
