use std::collections::HashMap;

use rusqlite::{params, Connection, Row};

use crate::glossary_entries::{
    GlossaryEntriesOverview, GlossaryEntryChanges, GlossaryEntrySummary, NewGlossaryEntry,
    GLOSSARY_ENTRY_VARIANT_TYPE_FORBIDDEN, GLOSSARY_ENTRY_VARIANT_TYPE_SOURCE,
    GLOSSARY_ENTRY_VARIANT_TYPE_TARGET,
};
use crate::persistence::error::PersistenceError;

#[derive(Debug, Clone, Default)]
struct EntryVariants {
    forbidden_terms: Vec<String>,
    source_variants: Vec<String>,
    target_variants: Vec<String>,
}

#[derive(Debug, Clone)]
struct GlossaryEntryRow {
    created_at: i64,
    glossary_id: String,
    id: String,
    source_term: String,
    status: String,
    target_term: String,
    context_note: Option<String>,
    updated_at: i64,
}

pub struct GlossaryEntryRepository<'connection> {
    connection: &'connection mut Connection,
}

impl<'connection> GlossaryEntryRepository<'connection> {
    pub fn new(connection: &'connection mut Connection) -> Self {
        Self { connection }
    }

    pub fn create(
        &mut self,
        new_entry: &NewGlossaryEntry,
    ) -> Result<GlossaryEntrySummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The glossary entry repository could not start the entry creation transaction.",
                error,
            )
        })?;

        transaction
            .execute(
                r#"
                INSERT INTO glossary_entries (
                  id,
                  glossary_id,
                  source_term,
                  target_term,
                  context_note,
                  status,
                  created_at,
                  updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
                params![
                    new_entry.id,
                    new_entry.glossary_id,
                    new_entry.source_term,
                    new_entry.target_term,
                    new_entry.context_note,
                    new_entry.status,
                    new_entry.created_at,
                    new_entry.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    "The glossary entry repository could not persist the new glossary entry.",
                    error,
                )
            })?;

        persist_variants(
            &transaction,
            &new_entry.id,
            new_entry.created_at,
            &new_entry.source_variants,
            &new_entry.target_variants,
            &new_entry.forbidden_terms,
        )?;
        touch_glossary(&transaction, &new_entry.glossary_id, new_entry.updated_at)?;

        let created_entry = load_entry(&transaction, &new_entry.glossary_id, &new_entry.id)?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The glossary entry repository could not commit the entry creation transaction.",
                error,
            )
        })?;

        Ok(created_entry)
    }

    pub fn list_by_glossary(
        &mut self,
        glossary_id: &str,
    ) -> Result<Vec<GlossaryEntrySummary>, PersistenceError> {
        let entry_rows = load_entry_rows(self.connection, glossary_id)?;
        let entry_variants = load_variants(self.connection, glossary_id)?;

        Ok(entry_rows
            .into_iter()
            .map(|entry_row| map_entry_summary(entry_row, &entry_variants))
            .collect())
    }

    pub fn update(
        &mut self,
        changes: &GlossaryEntryChanges,
    ) -> Result<GlossaryEntrySummary, PersistenceError> {
        let transaction = self.connection.transaction().map_err(|error| {
            PersistenceError::with_details(
                "The glossary entry repository could not start the entry update transaction.",
                error,
            )
        })?;

        let updated_rows = transaction
            .execute(
                r#"
                UPDATE glossary_entries
                SET
                  source_term = ?3,
                  target_term = ?4,
                  context_note = ?5,
                  status = ?6,
                  updated_at = ?7
                WHERE id = ?1 AND glossary_id = ?2
                "#,
                params![
                    changes.glossary_entry_id,
                    changes.glossary_id,
                    changes.source_term,
                    changes.target_term,
                    changes.context_note,
                    changes.status,
                    changes.updated_at
                ],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The glossary entry repository could not update entry {}.",
                        changes.glossary_entry_id
                    ),
                    error,
                )
            })?;

        if updated_rows == 0 {
            return Err(PersistenceError::new(format!(
                "The requested glossary entry {} does not exist in glossary {}.",
                changes.glossary_entry_id, changes.glossary_id
            )));
        }

        transaction
            .execute(
                "DELETE FROM glossary_entry_variants WHERE glossary_entry_id = ?1",
                [&changes.glossary_entry_id],
            )
            .map_err(|error| {
                PersistenceError::with_details(
                    format!(
                        "The glossary entry repository could not clear variants for entry {}.",
                        changes.glossary_entry_id
                    ),
                    error,
                )
            })?;

        persist_variants(
            &transaction,
            &changes.glossary_entry_id,
            changes.updated_at,
            &changes.source_variants,
            &changes.target_variants,
            &changes.forbidden_terms,
        )?;
        touch_glossary(&transaction, &changes.glossary_id, changes.updated_at)?;

        let updated_entry = load_entry(
            &transaction,
            &changes.glossary_id,
            &changes.glossary_entry_id,
        )?;

        transaction.commit().map_err(|error| {
            PersistenceError::with_details(
                "The glossary entry repository could not commit the entry update transaction.",
                error,
            )
        })?;

        Ok(updated_entry)
    }

    pub fn load_overview(
        &mut self,
        glossary_id: &str,
    ) -> Result<GlossaryEntriesOverview, PersistenceError> {
        Ok(GlossaryEntriesOverview {
            glossary_id: glossary_id.to_owned(),
            entries: self.list_by_glossary(glossary_id)?,
        })
    }
}

fn load_entry(
    connection: &Connection,
    glossary_id: &str,
    glossary_entry_id: &str,
) -> Result<GlossaryEntrySummary, PersistenceError> {
    let entry_row = connection
        .query_row(
            r#"
            SELECT
              id,
              glossary_id,
              source_term,
              target_term,
              context_note,
              status,
              created_at,
              updated_at
            FROM glossary_entries
            WHERE glossary_id = ?1 AND id = ?2
            "#,
            params![glossary_id, glossary_entry_id],
            map_entry_row,
        )
        .map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The glossary entry repository could not reload entry {glossary_entry_id}."
                ),
                error,
            )
        })?;
    let entry_variants = load_variants(connection, glossary_id)?;

    Ok(map_entry_summary(entry_row, &entry_variants))
}

fn load_entry_rows(
    connection: &Connection,
    glossary_id: &str,
) -> Result<Vec<GlossaryEntryRow>, PersistenceError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
              id,
              glossary_id,
              source_term,
              target_term,
              context_note,
              status,
              created_at,
              updated_at
            FROM glossary_entries
            WHERE glossary_id = ?1
            ORDER BY
              CASE status WHEN 'active' THEN 0 ELSE 1 END ASC,
              source_term COLLATE NOCASE ASC,
              target_term COLLATE NOCASE ASC,
              updated_at DESC
            "#,
        )
        .map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The glossary entry repository could not prepare the listing query for glossary {glossary_id}."
                ),
                error,
            )
        })?;

    let rows = statement
        .query_map([glossary_id], map_entry_row)
        .map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The glossary entry repository could not read entries for glossary {glossary_id}."
                ),
                error,
            )
        })?;

    let mut entry_rows = Vec::new();

    for row in rows {
        entry_rows.push(row.map_err(|error| {
            PersistenceError::with_details(
                format!(
                    "The glossary entry repository could not decode an entry row for glossary {glossary_id}."
                ),
                error,
            )
        })?);
    }

    Ok(entry_rows)
}

fn load_variants(
    connection: &Connection,
    glossary_id: &str,
) -> Result<HashMap<String, EntryVariants>, PersistenceError> {
    let mut statement = connection
        .prepare(
            r#"
        SELECT
          glossary_entry_variants.glossary_entry_id,
          glossary_entry_variants.variant_type,
          glossary_entry_variants.variant_text
        FROM glossary_entry_variants
        INNER JOIN glossary_entries
          ON glossary_entries.id = glossary_entry_variants.glossary_entry_id
        WHERE glossary_entries.glossary_id = ?1
        ORDER BY
          glossary_entry_variants.glossary_entry_id ASC,
          CASE glossary_entry_variants.variant_type
            WHEN 'source' THEN 0
            WHEN 'target' THEN 1
            ELSE 2
          END ASC,
          glossary_entry_variants.variant_text COLLATE NOCASE ASC
        "#,
        )
        .map_err(|error| {
            PersistenceError::with_details(
                "The glossary entry repository could not prepare the variants listing query.",
                error,
            )
        })?;
    let rows = statement
        .query_map([glossary_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| {
            PersistenceError::with_details(
                "The glossary entry repository could not read glossary entry variants.",
                error,
            )
        })?;

    let mut entry_variants = HashMap::new();

    for row in rows {
        let (entry_id, variant_type, variant_text) = row.map_err(|error| {
            PersistenceError::with_details(
                "The glossary entry repository could not decode a glossary entry variant row.",
                error,
            )
        })?;
        let variants = entry_variants
            .entry(entry_id)
            .or_insert_with(EntryVariants::default);

        match variant_type.as_str() {
            GLOSSARY_ENTRY_VARIANT_TYPE_SOURCE => variants.source_variants.push(variant_text),
            GLOSSARY_ENTRY_VARIANT_TYPE_TARGET => variants.target_variants.push(variant_text),
            GLOSSARY_ENTRY_VARIANT_TYPE_FORBIDDEN => variants.forbidden_terms.push(variant_text),
            _ => {}
        }
    }

    Ok(entry_variants)
}

fn map_entry_summary(
    entry_row: GlossaryEntryRow,
    entry_variants: &HashMap<String, EntryVariants>,
) -> GlossaryEntrySummary {
    let variants = entry_variants
        .get(&entry_row.id)
        .cloned()
        .unwrap_or_default();

    GlossaryEntrySummary {
        id: entry_row.id,
        glossary_id: entry_row.glossary_id,
        source_term: entry_row.source_term,
        target_term: entry_row.target_term,
        context_note: entry_row.context_note,
        status: entry_row.status,
        created_at: entry_row.created_at,
        updated_at: entry_row.updated_at,
        source_variants: variants.source_variants,
        target_variants: variants.target_variants,
        forbidden_terms: variants.forbidden_terms,
    }
}

fn persist_variants(
    connection: &Connection,
    glossary_entry_id: &str,
    created_at: i64,
    source_variants: &[String],
    target_variants: &[String],
    forbidden_terms: &[String],
) -> Result<(), PersistenceError> {
    for (variant_type, variants) in [
        (GLOSSARY_ENTRY_VARIANT_TYPE_SOURCE, source_variants),
        (GLOSSARY_ENTRY_VARIANT_TYPE_TARGET, target_variants),
        (GLOSSARY_ENTRY_VARIANT_TYPE_FORBIDDEN, forbidden_terms),
    ] {
        for (index, variant_text) in variants.iter().enumerate() {
            connection
                .execute(
                    r#"
                    INSERT INTO glossary_entry_variants (
                      id,
                      glossary_entry_id,
                      variant_text,
                      variant_type,
                      created_at
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    "#,
                    params![
                        format!("{glossary_entry_id}_{variant_type}_{index}"),
                        glossary_entry_id,
                        variant_text,
                        variant_type,
                        created_at
                    ],
                )
                .map_err(|error| {
                    PersistenceError::with_details(
                        format!(
                            "The glossary entry repository could not persist the {variant_type} variant {variant_text}."
                        ),
                        error,
                    )
                })?;
        }
    }

    Ok(())
}

fn touch_glossary(
    connection: &Connection,
    glossary_id: &str,
    updated_at: i64,
) -> Result<(), PersistenceError> {
    connection
        .execute(
            "UPDATE glossaries SET updated_at = ?2 WHERE id = ?1",
            params![glossary_id, updated_at],
        )
        .map_err(|error| {
            PersistenceError::with_details(
                format!("The glossary entry repository could not touch glossary {glossary_id}."),
                error,
            )
        })?;

    Ok(())
}

fn map_entry_row(row: &Row<'_>) -> rusqlite::Result<GlossaryEntryRow> {
    Ok(GlossaryEntryRow {
        id: row.get(0)?,
        glossary_id: row.get(1)?,
        source_term: row.get(2)?,
        target_term: row.get(3)?,
        context_note: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::GlossaryEntryRepository;
    use rusqlite::params;
    use tempfile::tempdir;

    use crate::glossaries::{NewGlossary, GLOSSARY_STATUS_ACTIVE};
    use crate::glossary_entries::{
        GlossaryEntryChanges, NewGlossaryEntry, GLOSSARY_ENTRY_STATUS_ACTIVE,
        GLOSSARY_ENTRY_STATUS_ARCHIVED,
    };
    use crate::persistence::bootstrap::{bootstrap_database, open_database_with_key};
    use crate::persistence::glossaries::GlossaryRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::projects::NewProject;

    const TEST_DATABASE_KEY: &str = "translat-test-key-for-d2";

    fn sample_project(now: i64) -> NewProject {
        NewProject {
            id: "prj_test_001".to_owned(),
            name: "Regulatory".to_owned(),
            description: Some("Project used to validate glossary entry persistence.".to_owned()),
            created_at: now,
            updated_at: now,
            last_opened_at: now,
        }
    }

    fn sample_glossary(now: i64) -> NewGlossary {
        NewGlossary {
            id: "gls_test_001".to_owned(),
            name: "Regulatory core".to_owned(),
            description: Some("Glossary container for D2 tests.".to_owned()),
            project_id: Some("prj_test_001".to_owned()),
            status: GLOSSARY_STATUS_ACTIVE.to_owned(),
            created_at: now,
            updated_at: now,
            last_opened_at: now,
        }
    }

    fn sample_entry(now: i64) -> NewGlossaryEntry {
        NewGlossaryEntry {
            id: "gle_test_001".to_owned(),
            glossary_id: "gls_test_001".to_owned(),
            source_term: "adverse event".to_owned(),
            target_term: "acontecimiento adverso".to_owned(),
            context_note: Some("Use in clinical reporting only.".to_owned()),
            status: GLOSSARY_ENTRY_STATUS_ACTIVE.to_owned(),
            created_at: now,
            updated_at: now,
            source_variants: vec!["adverse events".to_owned()],
            target_variants: vec!["evento adverso".to_owned()],
            forbidden_terms: vec!["incidente adverso".to_owned()],
        }
    }

    #[test]
    fn create_and_list_entries_round_trip() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_775_401_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        ProjectRepository::new(&mut connection)
            .create(&sample_project(now))
            .expect("project should be created");
        GlossaryRepository::new(&mut connection)
            .create(&sample_glossary(now))
            .expect("glossary should be created");

        let mut repository = GlossaryEntryRepository::new(&mut connection);
        let created_entry = repository
            .create(&sample_entry(now))
            .expect("entry should be created");
        let overview = repository
            .load_overview("gls_test_001")
            .expect("entry overview should load");

        assert_eq!(created_entry.id, "gle_test_001");
        assert_eq!(
            created_entry.source_variants,
            vec!["adverse events".to_owned()]
        );
        assert_eq!(
            created_entry.target_variants,
            vec!["evento adverso".to_owned()]
        );
        assert_eq!(
            created_entry.forbidden_terms,
            vec!["incidente adverso".to_owned()]
        );
        assert_eq!(overview.glossary_id, "gls_test_001");
        assert_eq!(overview.entries, vec![created_entry]);
    }

    #[test]
    fn entry_updates_and_survives_reopen() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let created_at = 1_775_401_200_i64;
        let updated_at = created_at + 120;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        {
            let mut first_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
                .expect("database connection should open");
            ProjectRepository::new(&mut first_connection)
                .create(&sample_project(created_at))
                .expect("project should be created");
            GlossaryRepository::new(&mut first_connection)
                .create(&sample_glossary(created_at))
                .expect("glossary should be created");

            let mut repository = GlossaryEntryRepository::new(&mut first_connection);
            repository
                .create(&sample_entry(created_at))
                .expect("entry should be created");
            repository
                .update(&GlossaryEntryChanges {
                    glossary_entry_id: "gle_test_001".to_owned(),
                    glossary_id: "gls_test_001".to_owned(),
                    source_term: "serious adverse event".to_owned(),
                    target_term: "acontecimiento adverso grave".to_owned(),
                    context_note: Some("Updated for the final termbase.".to_owned()),
                    status: GLOSSARY_ENTRY_STATUS_ARCHIVED.to_owned(),
                    updated_at,
                    source_variants: vec!["serious adverse events".to_owned(), "SAE".to_owned()],
                    target_variants: vec!["evento adverso grave".to_owned()],
                    forbidden_terms: vec!["evento serio".to_owned()],
                })
                .expect("entry should update");
        }

        let mut second_connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should reopen");
        let mut repository = GlossaryEntryRepository::new(&mut second_connection);
        let overview = repository
            .load_overview("gls_test_001")
            .expect("entry overview should reload");

        assert_eq!(overview.entries.len(), 1);
        assert_eq!(overview.entries[0].source_term, "serious adverse event");
        assert_eq!(
            overview.entries[0].target_term,
            "acontecimiento adverso grave"
        );
        assert_eq!(overview.entries[0].status, GLOSSARY_ENTRY_STATUS_ARCHIVED);
        assert_eq!(
            overview.entries[0].source_variants,
            vec!["SAE".to_owned(), "serious adverse events".to_owned()]
        );
        assert_eq!(
            overview.entries[0].target_variants,
            vec!["evento adverso grave".to_owned()]
        );
        assert_eq!(
            overview.entries[0].forbidden_terms,
            vec!["evento serio".to_owned()]
        );
        assert_eq!(overview.entries[0].updated_at, updated_at);
    }

    #[test]
    fn list_entries_supports_large_glossaries() {
        let temporary_directory = tempdir().expect("temp dir should be created");
        let database_path = temporary_directory.path().join("translat.sqlite3");
        let now = 1_775_401_200_i64;

        bootstrap_database(&database_path, TEST_DATABASE_KEY)
            .expect("database bootstrap should succeed");

        let mut connection = open_database_with_key(&database_path, TEST_DATABASE_KEY)
            .expect("database connection should open");
        ProjectRepository::new(&mut connection)
            .create(&sample_project(now))
            .expect("project should be created");
        GlossaryRepository::new(&mut connection)
            .create(&sample_glossary(now))
            .expect("glossary should be created");

        let transaction = connection
            .transaction()
            .expect("bulk entry transaction should start");

        for index in 0..1_050 {
            transaction
                .execute(
                    r#"
                    INSERT INTO glossary_entries (
                      id,
                      glossary_id,
                      source_term,
                      target_term,
                      context_note,
                      status,
                      created_at,
                      updated_at
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    "#,
                    params![
                        format!("gle_bulk_{index:04}"),
                        "gls_test_001",
                        format!("source term {index:04}"),
                        format!("target term {index:04}"),
                        Option::<String>::None,
                        GLOSSARY_ENTRY_STATUS_ACTIVE,
                        now,
                        now
                    ],
                )
                .expect("bulk glossary entry should be inserted");
        }

        transaction
            .commit()
            .expect("bulk entry transaction should commit");

        let mut repository = GlossaryEntryRepository::new(&mut connection);
        let overview = repository
            .load_overview("gls_test_001")
            .expect("large glossary entry overview should load");

        assert_eq!(overview.entries.len(), 1_050);
        assert_eq!(overview.entries[0].source_term, "source term 0000");
        assert_eq!(overview.entries[1_049].source_term, "source term 1049");
    }
}
