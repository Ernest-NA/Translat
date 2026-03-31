use rusqlite::{params, Connection, OptionalExtension};

use crate::persistence::error::PersistenceError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    pub version: &'static str,
    pub name: &'static str,
    pub sql: &'static str,
}

impl Migration {
    pub fn label(self) -> &'static str {
        match (self.version, self.name) {
            ("0001", "initial_schema") => "0001_initial_schema",
            _ => self.version,
        }
    }
}

const MIGRATIONS: [Migration; 1] = [Migration {
    version: "0001",
    name: "initial_schema",
    sql: include_str!("../../migrations/0001_initial_schema.sql"),
}];

pub fn ensure_schema_migrations_table(connection: &Connection) -> Result<(), PersistenceError> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
              version TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              applied_at INTEGER NOT NULL
            );
            "#,
        )
        .map_err(|error| {
            PersistenceError::with_details(
                "The database bootstrap could not ensure the schema_migrations table.",
                error,
            )
        })
}

pub fn run_pending_migrations(
    connection: &mut Connection,
) -> Result<Vec<String>, PersistenceError> {
    ensure_schema_migrations_table(connection)?;

    let transaction = connection.transaction().map_err(|error| {
        PersistenceError::with_details(
            "The database bootstrap could not start the migration transaction.",
            error,
        )
    })?;

    let mut newly_applied_migrations = Vec::new();

    for migration in MIGRATIONS {
        let existing_version = transaction
            .query_row(
                "SELECT version FROM schema_migrations WHERE version = ?1",
                [migration.version],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The database bootstrap could not inspect migration {}.",
                        migration.label()
                    ),
                    error,
                )
            })?;

        if existing_version.is_some() {
            continue;
        }

        transaction.execute_batch(migration.sql).map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The database bootstrap could not apply migration {}.",
                    migration.label()
                ),
                error,
            )
        })?;

        transaction
            .execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, unixepoch())",
                params![migration.version, migration.name],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The database bootstrap could not register migration {}.",
                        migration.label()
                    ),
                    error,
                )
            })?;

        newly_applied_migrations.push(migration.label().to_owned());
    }

    transaction.commit().map_err(|error| {
        PersistenceError::with_details(
            "The database bootstrap could not commit the migration transaction.",
            error,
        )
    })?;

    Ok(newly_applied_migrations)
}

pub fn list_applied_migrations(connection: &Connection) -> Result<Vec<String>, PersistenceError> {
    let mut statement = connection
        .prepare("SELECT version, name FROM schema_migrations ORDER BY version ASC")
        .map_err(|error| {
            PersistenceError::with_details(
                "The database bootstrap could not prepare the migration inspection query.",
                error,
            )
        })?;

    let rows = statement
        .query_map([], |row| {
            let version = row.get::<_, String>(0)?;
            let name = row.get::<_, String>(1)?;

            Ok(format!("{version}_{name}"))
        })
        .map_err(|error| {
            PersistenceError::with_details(
                "The database bootstrap could not read the applied migrations.",
                error,
            )
        })?;

    let mut applied_migrations = Vec::new();

    for row in rows {
        applied_migrations.push(row.map_err(|error| {
            PersistenceError::with_details(
                "The database bootstrap could not decode an applied migration row.",
                error,
            )
        })?);
    }

    Ok(applied_migrations)
}

pub fn has_table(connection: &Connection, table_name: &str) -> Result<bool, PersistenceError> {
    let table_count = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| {
            PersistenceError::with_details(
                format!("The database bootstrap could not inspect table {table_name}."),
                error,
            )
        })?;

    Ok(table_count == 1)
}
