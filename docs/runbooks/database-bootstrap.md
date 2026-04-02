# Database bootstrap

## Objective

Describe how Translat initializes its local encrypted SQLite database during the current foundation stage.

## Current strategy

- The desktop backend uses `rusqlite` with SQLCipher enabled through a bundled build.
- The SQLite key is generated once and stored in the user profile with Windows DPAPI protection.
- The database file and the encrypted key file live under the Translat Tauri `app_data_dir`.
- Every connection enables `PRAGMA foreign_keys = ON` before running migrations or queries.
- SQLCipher currently requires a full Perl distribution to compile the bundled OpenSSL dependency on Windows, so local and CI builds use Strawberry Perl.

## Current files

- `src-tauri/src/persistence/bootstrap.rs`
- `src-tauri/src/persistence/documents.rs`
- `src-tauri/src/persistence/migrations.rs`
- `src-tauri/src/persistence/projects.rs`
- `src-tauri/src/persistence/secret_store.rs`
- `src-tauri/migrations/0001_initial_schema.sql`
- `src-tauri/migrations/0002_projects.sql`
- `src-tauri/migrations/0003_documents.sql`

## Migration behavior

- `schema_migrations` is created automatically if it does not exist.
- `0001_initial_schema` is applied on the first clean bootstrap.
- `0002_projects` creates the initial persisted project container used by C1.
- `0003_documents` creates the persisted document registry used by C2.
- Later startups reopen the same encrypted database and skip already applied versions.
- The current bootstrap establishes the minimum operational schema plus persisted project and document repository tables.

## Runtime expectations

- The desktop shell bootstraps the database during Tauri setup.
- The healthcheck command reports the database path, the applied migrations, and whether the initial schema is ready.
- Project commands persist project metadata and the active project selection through the same encrypted database.
- Document commands persist imported document metadata plus their project association through the same encrypted database.
- Rust tests validate first initialization, second initialization without reapplying migrations, the presence of the `projects` and `documents` tables, project persistence across reopen, and document persistence across reopen.

## Current limits

- Key protection is currently implemented for Windows with DPAPI because Translat targets Windows 11 at this stage.
- The current SQLCipher toolchain depends on Strawberry Perl during compilation. If the project later changes SQLite encryption strategy, revisit this prerequisite together with the build pipeline.
- If the project later expands beyond Windows, revisit key storage behind the persistence boundary instead of changing repository code ad hoc.
- This stage now adds persisted projects plus the first document registry and local copied file storage. Normalization, segmentation, FTS, and later business tables remain out of scope.
