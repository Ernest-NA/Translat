use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::glossaries::{
    GlossariesOverview, GlossaryChanges, GlossarySummary, NewGlossary, ACTIVE_GLOSSARY_METADATA_KEY,
};
use crate::persistence::error::PersistenceError;

pub struct GlossaryRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> GlossaryRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn create(
        &mut self,
        new_glossary: &NewGlossary,
    ) -> Result<GlossarySummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The glossary repository could not start the glossary creation transaction.",
                error,
            )
        })?;

        transaction
            .execute(
                r#"
                INSERT INTO glossaries (
                  id,
                  name,
                  description,
                  project_id,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    new_glossary.id,
                    new_glossary.name,
                    new_glossary.description,
                    new_glossary.project_id,
                    new_glossary.status,
                    new_glossary.created_at,
                    new_glossary.updated_at,
                    new_glossary.last_opened_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The glossary repository could not persist the new glossary.",
                    error,
                )
            })?;

        upsert_active_glossary(&transaction, &new_glossary.id, new_glossary.updated_at)?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The glossary repository could not commit the glossary creation transaction.",
                error,
            )
        })?;

        Ok(GlossarySummary {
            id: new_glossary.id.clone(),
            name: new_glossary.name.clone(),
            description: new_glossary.description.clone(),
            project_id: new_glossary.project_id.clone(),
            status: new_glossary.status.clone(),
            created_at: new_glossary.created_at,
            updated_at: new_glossary.updated_at,
            last_opened_at: new_glossary.last_opened_at,
        })
    }

    pub fn list(&mut self) -> Result<Vec<GlossarySummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  project_id,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM glossaries
                ORDER BY
                  CASE status WHEN 'active' THEN 0 ELSE 1 END ASC,
                  last_opened_at DESC,
                  created_at DESC,
                  name COLLATE NOCASE ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The glossary repository could not prepare the glossary listing query.",
                    error,
                )
            })?;

        let rows = statement
            .query_map([], map_glossary_summary)
            .map_err(|error| {
                PersistenceError::with_details(
                    "The glossary repository could not read the glossary list.",
                    error,
                )
            })?;

        let mut glossaries = Vec::new();

        for row in rows {
            glossaries.push(row.map_err(|error| {
                PersistenceError::with_details(
                    "The glossary repository could not decode a glossary row.",
                    error,
                )
            })?);
        }

        Ok(glossaries)
    }

    pub fn open_glossary(
        &mut self,
        glossary_id: &str,
        opened_at: i64,
    ) -> Result<GlossarySummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The glossary repository could not start the glossary opening transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                "UPDATE glossaries SET last_opened_at = ?2 WHERE id = ?1",
                params![glossary_id, opened_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The glossary repository could not mark glossary {glossary_id} as opened."
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested glossary {glossary_id} does not exist."
            )));
        }

        upsert_active_glossary(&transaction, glossary_id, opened_at)?;

        let glossary = transaction
            .query_row(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  project_id,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM glossaries
                WHERE id = ?1
                "#,
                [glossary_id],
                map_glossary_summary,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The glossary repository could not reload glossary {glossary_id}."),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The glossary repository could not commit the glossary opening transaction.",
                error,
            )
        })?;

        Ok(glossary)
    }

    pub fn update(
        &mut self,
        changes: &GlossaryChanges,
    ) -> Result<GlossarySummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The glossary repository could not start the glossary update transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                r#"
                UPDATE glossaries
                SET
                  name = ?2,
                  description = ?3,
                  project_id = ?4,
                  status = ?5,
                  updated_at = ?6
                WHERE id = ?1
                "#,
                params![
                    changes.glossary_id,
                    changes.name,
                    changes.description,
                    changes.project_id,
                    changes.status,
                    changes.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The glossary repository could not update glossary {}.",
                        changes.glossary_id
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested glossary {} does not exist.",
                changes.glossary_id
            )));
        }

        upsert_active_glossary(&transaction, &changes.glossary_id, changes.updated_at)?;

        let glossary = transaction
            .query_row(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  project_id,
                  status,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM glossaries
                WHERE id = ?1
                "#,
                [&changes.glossary_id],
                map_glossary_summary,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The glossary repository could not reload glossary {}.",
                        changes.glossary_id
                    ),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The glossary repository could not commit the glossary update transaction.",
                error,
            )
        })?;

        Ok(glossary)
    }

    pub fn load_overview(&mut self) -> Result<GlossariesOverview, PersistenceError> {
        Ok(GlossariesOverview {
            active_glossary_id: self.active_glossary_id()?,
            glossaries: self.list()?,
        })
    }

    pub fn active_glossary_id(&mut self) -> Result<Option<String>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                [ACTIVE_GLOSSARY_METADATA_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    "The glossary repository could not load the active glossary id.",
                    error,
                )
            })
    }

    pub fn exists(&mut self, glossary_id: &str) -> Result<bool, PersistenceError> {
        let glossary_count = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM glossaries WHERE id = ?1",
                [glossary_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The glossary repository could not inspect glossary {glossary_id}."),
                    error,
                )
            })?;

        Ok(glossary_count == 1)
    }
}

fn upsert_active_glossary(
    connection: &Connection,
    glossary_id: &str,
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
            params![ACTIVE_GLOSSARY_METADATA_KEY, glossary_id, timestamp],
        )
        .map_err(|error| {
            PersistenceError::with_details(
                "The glossary repository could not persist the active glossary selection.",
                error,
            )
        })?;

    Ok(())
}

fn map_glossary_summary(row: &Row<'_>) -> rusqlite::Result<GlossarySummary> {
    Ok(GlossarySummary {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        project_id: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        last_opened_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::GlossaryRepository;
    use tempfile::tempdir;

    use crate::glossaries::{
        GlossaryChanges, NewGlossary, GLOSSARY_STATUS_ACTIVE, GLOSSARY_STATUS_ARCHIVED,
    };
    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::persistence::projects::ProjectRepository;
    use crate::projects::NewProject;

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-d1";

    fn sample_project(now: i64) -> NewProject {
        NewProject {
            id: "prj_test_001".to_owned(),
            name: "Medical oncology".to_owned(),
            description: Some("Project used to validate glossary persistence.".to_owned()),
            created_at: now,
            updated_at: now,
            last_opened_at: now,
        }
    }

    fn sample_glossary(now: i64) -> NewGlossary {
        NewGlossary {
            id: "gls_test_001".to_owned(),
            name: "Core oncology".to_owned(),
            description: Some("Reusable terminology baseline.".to_owned()),
            project_id: Some("prj_test_001".to_owned()),
            status: GLOSSARY_STATUS_ACTIVE.to_owned(),
            created_at: now,
            updated_at: now,
            last_opened_at: now,
        }
    }

    #[test]
    fn create_and_list_glossaries_round_trip() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_775_315_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        ProjectRepository::new(&mut connection)
            .create(&sample_project(now))
            .expect("project should be created");

        let mut repository = GlossaryRepository::new(&mut connection);
        let created_glossary = repository
            .create(&sample_glossary(now))
            .expect("glossary should be created");
        let overview = repository
            .load_overview()
            .expect("glossary overview should load");

        assert_eq!(created_glossary.id, "gls_test_001");
        assert_eq!(overview.active_glossary_id, Some("gls_test_001".to_owned()));
        assert_eq!(overview.glossaries, vec![created_glossary]);
    }

    #[test]
    fn glossary_updates_and_survives_reopen() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let created_at = 1_775_315_200_i64;
        let reopened_at = created_at + 300;
        let updated_at = reopened_at + 60;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        {
            let mut first_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
                .expect("database connection should open");
            ProjectRepository::new(&mut first_connection)
                .create(&sample_project(created_at))
                .expect("project should be created");

            let mut repository = GlossaryRepository::new(&mut first_connection);
            repository
                .create(&sample_glossary(created_at))
                .expect("glossary should be created");
            repository
                .open_glossary("gls_test_001", reopened_at)
                .expect("glossary should reopen");
            repository
                .update(&GlossaryChanges {
                    glossary_id: "gls_test_001".to_owned(),
                    name: "Archived oncology baseline".to_owned(),
                    description: Some("Glossary archived for D2 readiness.".to_owned()),
                    project_id: None,
                    status: GLOSSARY_STATUS_ARCHIVED.to_owned(),
                    updated_at,
                })
                .expect("glossary should update");
        }

        let mut second_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should reopen");
        let mut repository = GlossaryRepository::new(&mut second_connection);
        let overview = repository
            .load_overview()
            .expect("glossary overview should reload");

        assert_eq!(overview.active_glossary_id, Some("gls_test_001".to_owned()));
        assert_eq!(overview.glossaries.len(), 1);
        assert_eq!(overview.glossaries[0].name, "Archived oncology baseline");
        assert_eq!(overview.glossaries[0].project_id, None);
        assert_eq!(overview.glossaries[0].status, GLOSSARY_STATUS_ARCHIVED);
        assert_eq!(overview.glossaries[0].last_opened_at, reopened_at);
        assert_eq!(overview.glossaries[0].updated_at, updated_at);
    }
}
