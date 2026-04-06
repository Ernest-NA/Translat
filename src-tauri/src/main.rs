#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod chapter_contexts;
mod commands;
mod context_builder;
mod documents;
mod error;
mod glossaries;
mod glossary_entries;
mod persistence;
mod projects;
mod qa_findings;
mod rule_sets;
mod sections;
mod segments;
mod style_profiles;
mod task_runs;
mod translate_chunk;
mod translation_chunks;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let (database_runtime, bootstrap_report) =
                persistence::bootstrap::bootstrap_app_database(app.handle())?;

            println!(
                "[persistence] {} ready at {} with migrations: {}",
                bootstrap_report.encryption,
                bootstrap_report.database_path.display(),
                bootstrap_report.applied_migrations.join(", ")
            );

            app.manage(database_runtime);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::glossaries::create_glossary,
            commands::glossary_entries::create_glossary_entry,
            commands::rule_sets::create_rule,
            commands::rule_sets::create_rule_set,
            commands::style_profiles::create_style_profile,
            commands::glossaries::list_glossaries,
            commands::glossary_entries::list_glossary_entries,
            commands::rule_sets::list_rule_set_rules,
            commands::rule_sets::list_rule_sets,
            commands::style_profiles::list_style_profiles,
            commands::glossaries::open_glossary,
            commands::rule_sets::open_rule_set,
            commands::style_profiles::open_style_profile,
            commands::glossary_entries::update_glossary_entry,
            commands::glossaries::update_glossary,
            commands::rule_sets::update_rule,
            commands::rule_sets::update_rule_set,
            commands::style_profiles::update_style_profile,
            commands::documents::import_project_document,
            commands::documents::list_project_documents,
            commands::healthcheck::healthcheck,
            commands::projects::create_project,
            commands::projects::list_projects,
            commands::projects::open_project,
            commands::projects::update_project_editorial_defaults,
            commands::segments::list_document_segments,
            commands::segments::process_project_document,
            commands::translation_chunks::build_document_translation_chunks,
            commands::translation_chunks::list_document_translation_chunks,
            commands::context_builder::build_translation_context,
            commands::translate_chunk::translate_chunk
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Translat desktop shell");
}

#[cfg(test)]
mod tests {
    use super::commands::healthcheck::build_healthcheck_response;
    use super::persistence::bootstrap::DatabaseStatus;

    #[test]
    fn healthcheck_returns_expected_shell_status() {
        let response = build_healthcheck_response(DatabaseStatus {
            applied_migrations: vec!["0001_initial_schema".to_owned()],
            encryption: "sqlcipher".to_owned(),
            key_storage: "windows-dpapi".to_owned(),
            migration_count: 1,
            path: "C:\\Translat\\translat.sqlite3".to_owned(),
            schema_ready: true,
        })
        .expect("healthcheck should succeed");

        assert_eq!(response.app_name, "Translat");
        assert_eq!(response.database.encryption, "sqlcipher");
        assert_eq!(response.database.migration_count, 1);
        assert_eq!(
            response.database.applied_migrations,
            vec!["0001_initial_schema".to_owned()]
        );
        assert!(response.database.schema_ready);
        assert_eq!(response.environment, "development");
        assert_eq!(response.status, "ok");
        assert!(!response.message.is_empty());
        assert!(!response.version.is_empty());
        assert!(response.checked_at > 0);
    }
}
