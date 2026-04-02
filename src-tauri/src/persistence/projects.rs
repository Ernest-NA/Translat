use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::persistence::error::PersistenceError;
use crate::projects::{NewProject, ProjectSummary, ProjectsOverview, ACTIVE_PROJECT_METADATA_KEY};

pub struct ProjectRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> ProjectRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn create(&mut self, new_project: &NewProject) -> Result<ProjectSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The project repository could not start the project creation transaction.",
                error,
            )
        })?;

        transaction
            .execute(
                r#"
                INSERT INTO projects (
                  id,
                  name,
                  description,
                  created_at,
                  updated_at,
                  last_opened_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    new_project.id,
                    new_project.name,
                    new_project.description,
                    new_project.created_at,
                    new_project.updated_at,
                    new_project.last_opened_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The project repository could not persist the new project.",
                    error,
                )
            })?;

        upsert_active_project(&transaction, &new_project.id, new_project.last_opened_at)?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The project repository could not commit the project creation transaction.",
                error,
            )
        })?;

        Ok(ProjectSummary {
            id: new_project.id.clone(),
            name: new_project.name.clone(),
            description: new_project.description.clone(),
            created_at: new_project.created_at,
            updated_at: new_project.updated_at,
            last_opened_at: new_project.last_opened_at,
        })
    }

    pub fn list(&mut self) -> Result<Vec<ProjectSummary>, PersistenceError> {
        let mut statement = self
            .connection
            .prepare(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM projects
                ORDER BY last_opened_at DESC, created_at DESC, name COLLATE NOCASE ASC
                "#,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The project repository could not prepare the project listing query.",
                    error,
                )
            })?;

        let rows = statement
            .query_map([], map_project_summary)
            .map_err(|error| {
                PersistenceError::with_details(
                    "The project repository could not read the project list.",
                    error,
                )
            })?;

        let mut projects = Vec::new();

        for row in rows {
            projects.push(row.map_err(|error| {
                PersistenceError::with_details(
                    "The project repository could not decode a project row.",
                    error,
                )
            })?);
        }

        Ok(projects)
    }

    pub fn open_project(
        &mut self,
        project_id: &str,
        opened_at: i64,
    ) -> Result<ProjectSummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The project repository could not start the project opening transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                "UPDATE projects SET last_opened_at = ?2 WHERE id = ?1",
                params![project_id, opened_at],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The project repository could not mark project {project_id} as opened."
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested project {project_id} does not exist."
            )));
        }

        upsert_active_project(&transaction, project_id, opened_at)?;

        let project = transaction
            .query_row(
                r#"
                SELECT
                  id,
                  name,
                  description,
                  created_at,
                  updated_at,
                  last_opened_at
                FROM projects
                WHERE id = ?1
                "#,
                [project_id],
                map_project_summary,
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!("The project repository could not reload project {project_id}."),
                    error,
                )
            })?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The project repository could not commit the project opening transaction.",
                error,
            )
        })?;

        Ok(project)
    }

    pub fn load_overview(&mut self) -> Result<ProjectsOverview, PersistenceError> {
        Ok(ProjectsOverview {
            active_project_id: self.active_project_id()?,
            projects: self.list()?,
        })
    }

    fn active_project_id(&mut self) -> Result<Option<String>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                [ACTIVE_PROJECT_METADATA_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    "The project repository could not load the active project id.",
                    error,
                )
            })
    }
}

fn upsert_active_project(
    connection: &Connection,
    project_id: &str,
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
            params![ACTIVE_PROJECT_METADATA_KEY, project_id, timestamp],
        )
        .map_err(|error| {
            PersistenceError::with_details(
                "The project repository could not persist the active project selection.",
                error,
            )
        })?;

    Ok(())
}

fn map_project_summary(row: &Row<'_>) -> rusqlite::Result<ProjectSummary> {
    Ok(ProjectSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        last_opened_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::ProjectRepository;
    use tempfile::tempdir;

    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::projects::NewProject;

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-c1";

    fn sample_project(now: i64) -> NewProject {
        NewProject {
            id: "prj_test_001".to_owned(),
            name: "Glossary pilot".to_owned(),
            description: Some("Initial workspace for project persistence tests.".to_owned()),
            created_at: now,
            updated_at: now,
            last_opened_at: now,
        }
    }

    #[test]
    fn create_and_list_projects_round_trip() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_743_517_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        let mut repository = ProjectRepository::new(&mut connection);
        let created_project = repository
            .create(&sample_project(now))
            .expect("project should be created");
        let overview = repository
            .load_overview()
            .expect("project overview should load");

        assert_eq!(created_project.id, "prj_test_001");
        assert_eq!(overview.active_project_id, Some("prj_test_001".to_owned()));
        assert_eq!(overview.projects, vec![created_project]);
    }

    #[test]
    fn active_project_and_projects_survive_reopen() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let created_at = 1_743_517_200_i64;
        let reopened_at = created_at + 300;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        {
            let mut first_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
                .expect("database connection should open");
            let mut repository = ProjectRepository::new(&mut first_connection);

            repository
                .create(&sample_project(created_at))
                .expect("project should be created");
            repository
                .open_project("prj_test_001", reopened_at)
                .expect("project should reopen");
        }

        let mut second_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should reopen");
        let mut repository = ProjectRepository::new(&mut second_connection);
        let overview = repository
            .load_overview()
            .expect("project overview should reload");

        assert_eq!(overview.active_project_id, Some("prj_test_001".to_owned()));
        assert_eq!(overview.projects.len(), 1);
        assert_eq!(overview.projects[0].last_opened_at, reopened_at);
        assert_eq!(overview.projects[0].updated_at, created_at);
    }
}
