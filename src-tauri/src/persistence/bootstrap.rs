use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::persistence::error::PersistenceError;
use crate::persistence::migrations;
use crate::persistence::secret_store;

pub const DATABASE_FILE_NAME: &str = "translat.sqlite3";
pub const DATABASE_KEY_FILE_NAME: &str = "translat.sqlite3.key";
pub const DATABASE_ENCRYPTION_LABEL: &str = "sqlcipher";
pub const DATABASE_KEY_STORAGE_LABEL: &str = "windows-dpapi";

#[derive(Debug, Clone)]
pub struct DatabaseRuntime {
    database_path: PathBuf,
    encryption_key_path: PathBuf,
}

impl DatabaseRuntime {
    pub fn new(database_path: PathBuf, encryption_key_path: PathBuf) -> Self {
        Self {
            database_path,
            encryption_key_path,
        }
    }

    pub fn open_connection(&self) -> Result<Connection, PersistenceError> {
        let encryption_key = secret_store::load_existing_encryption_key(&self.encryption_key_path)?;

        open_database_with_key(&self.database_path, &encryption_key)
    }

    pub fn inspect(&self) -> Result<DatabaseStatus, PersistenceError> {
        let connection = self.open_connection()?;

        inspect_connection(&self.database_path, &connection)
    }

    pub fn documents_directory(&self) -> Result<PathBuf, PersistenceError> {
        self.database_path
            .parent()
            .map(|directory| directory.join("documents"))
            .ok_or_else(|| {
                PersistenceError::new(
                    "The persistence runtime could not derive the document storage directory from the database path.",
                )
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseBootstrapReport {
    pub applied_migrations: Vec<String>,
    pub database_path: PathBuf,
    pub encryption: &'static str,
    pub key_storage: &'static str,
    pub newly_applied_migrations: Vec<String>,
    pub schema_ready: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatus {
    pub applied_migrations: Vec<String>,
    pub encryption: String,
    pub key_storage: String,
    pub migration_count: u64,
    pub path: String,
    pub schema_ready: bool,
}

pub fn bootstrap_app_database(
    app_handle: &AppHandle,
) -> Result<(DatabaseRuntime, DatabaseBootstrapReport), PersistenceError> {
    let app_data_directory = app_handle.path().app_data_dir().map_err(|error| {
        PersistenceError::with_details(
            "The persistence bootstrap could not resolve the Translat app data directory.",
            error,
        )
    })?;

    fs::create_dir_all(&app_data_directory).map_err(|error| {
        PersistenceError::with_details(
            format!(
                "The persistence bootstrap could not create the app data directory at {}.",
                app_data_directory.display()
            ),
            error,
        )
    })?;

    let database_path = app_data_directory.join(DATABASE_FILE_NAME);
    let encryption_key_path = app_data_directory.join(DATABASE_KEY_FILE_NAME);

    if database_path.exists() && !encryption_key_path.exists() {
        return Err(PersistenceError::new(
            "The encrypted SQLite database already exists, but the protected key file is missing.",
        ));
    }

    let encryption_key = secret_store::load_or_create_encryption_key(&encryption_key_path)?;
    let bootstrap_report = bootstrap_database(&database_path, &encryption_key)?;
    let runtime = DatabaseRuntime::new(database_path, encryption_key_path);

    Ok((runtime, bootstrap_report))
}

pub fn bootstrap_database(
    database_path: &Path,
    encryption_key: &str,
) -> Result<DatabaseBootstrapReport, PersistenceError> {
    if let Some(parent_directory) = database_path.parent() {
        fs::create_dir_all(parent_directory).map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The persistence bootstrap could not create the database directory at {}.",
                    parent_directory.display()
                ),
                error,
            )
        })?;
    }

    let mut connection = open_database_with_key(database_path, encryption_key)?;
    let newly_applied_migrations = migrations::run_pending_migrations(&mut connection)?;
    let database_status = inspect_connection(database_path, &connection)?;

    Ok(DatabaseBootstrapReport {
        applied_migrations: database_status.applied_migrations,
        database_path: database_path.to_path_buf(),
        encryption: DATABASE_ENCRYPTION_LABEL,
        key_storage: DATABASE_KEY_STORAGE_LABEL,
        newly_applied_migrations,
        schema_ready: database_status.schema_ready,
    })
}

#[cfg(test)]
pub fn inspect_database(
    database_path: &Path,
    encryption_key: &str,
) -> Result<DatabaseStatus, PersistenceError> {
    let connection = open_database_with_key(database_path, encryption_key)?;

    inspect_connection(database_path, &connection)
}

pub fn open_database_with_key(
    database_path: &Path,
    encryption_key: &str,
) -> Result<Connection, PersistenceError> {
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|error| {
        PersistenceError::with_details(
            format!(
                "The persistence bootstrap could not open the encrypted database at {}.",
                database_path.display()
            ),
            error,
        )
    })?;

    configure_connection(&connection, encryption_key)?;

    Ok(connection)
}

fn configure_connection(
    connection: &Connection,
    encryption_key: &str,
) -> Result<(), PersistenceError> {
    connection.pragma_update(None, "key", encryption_key).map_err(|error| {
        PersistenceError::with_details(
            "The persistence bootstrap could not apply the SQLCipher key to the database connection.",
            error,
        )
    })?;

    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| {
            PersistenceError::with_details(
                "The persistence bootstrap could not configure the SQLite busy timeout.",
                error,
            )
        })?;

    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(|error| {
            PersistenceError::with_details(
                "The persistence bootstrap could not enable foreign key enforcement.",
                error,
            )
        })?;

    Ok(())
}

fn inspect_connection(
    database_path: &Path,
    connection: &Connection,
) -> Result<DatabaseStatus, PersistenceError> {
    let applied_migrations = migrations::list_applied_migrations(connection)?;
    let migration_count = connection
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| {
            PersistenceError::with_details(
                "The persistence bootstrap could not count applied schema migrations.",
                error,
            )
        })?;

    let schema_ready = migrations::has_table(connection, "app_metadata")?
        && migrations::has_table(connection, "projects")?
        && migrations::has_table(connection, "documents")?;

    Ok(DatabaseStatus {
        applied_migrations,
        encryption: DATABASE_ENCRYPTION_LABEL.to_owned(),
        key_storage: DATABASE_KEY_STORAGE_LABEL.to_owned(),
        migration_count: u64::try_from(migration_count).map_err(|error| {
            PersistenceError::with_details(
                "The persistence bootstrap produced an invalid migration count.",
                error,
            )
        })?,
        path: database_path.display().to_string(),
        schema_ready,
    })
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{bootstrap_database, inspect_database, open_database_with_key};

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-b4";

    #[test]
    fn bootstrap_creates_schema_and_records_initial_migration() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");

        let bootstrap_report = bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        assert!(database_path.exists());
        assert_eq!(
            bootstrap_report.newly_applied_migrations,
            vec![
                "0001_initial_schema".to_owned(),
                "0002_projects".to_owned(),
                "0003_documents".to_owned()
            ]
        );
        assert_eq!(
            bootstrap_report.applied_migrations,
            vec![
                "0001_initial_schema".to_owned(),
                "0002_projects".to_owned(),
                "0003_documents".to_owned()
            ]
        );
        assert!(bootstrap_report.schema_ready);
    }

    #[test]
    fn bootstrap_is_idempotent_on_second_open() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("first bootstrap should succeed");

        let second_report = bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("second bootstrap should succeed");

        assert!(second_report.newly_applied_migrations.is_empty());
        assert_eq!(
            second_report.applied_migrations,
            vec![
                "0001_initial_schema".to_owned(),
                "0002_projects".to_owned(),
                "0003_documents".to_owned()
            ]
        );
        assert!(second_report.schema_ready);
    }

    #[test]
    fn bootstrap_keeps_schema_migrations_and_initial_schema_queryable() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should reopen");
        let migration_count = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("schema_migrations count should be queryable");
        let app_metadata_table_count = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'app_metadata'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("initial schema table should be queryable");
        let projects_table_count = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'projects'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("projects table should be queryable");
        let documents_table_count = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'documents'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("documents table should be queryable");

        let database_status = inspect_database(&database_path, TEST_DATABASE_KEY)
            .expect("database inspection should succeed");

        assert_eq!(migration_count, 3);
        assert_eq!(app_metadata_table_count, 1);
        assert_eq!(projects_table_count, 1);
        assert_eq!(documents_table_count, 1);
        assert_eq!(database_status.migration_count, 3);
        assert_eq!(
            database_status.applied_migrations,
            vec![
                "0001_initial_schema".to_owned(),
                "0002_projects".to_owned(),
                "0003_documents".to_owned()
            ]
        );
        assert!(database_status.schema_ready);
    }
}
