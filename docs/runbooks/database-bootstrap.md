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
- `src-tauri/src/persistence/segments.rs`
- `src-tauri/migrations/0001_initial_schema.sql`
- `src-tauri/migrations/0002_projects.sql`
- `src-tauri/migrations/0003_documents.sql`
- `src-tauri/migrations/0004_segments.sql`

## Migration behavior

- `schema_migrations` is created automatically if it does not exist.
- `0001_initial_schema` is applied on the first clean bootstrap.
- `0002_projects` creates the initial persisted project container used by C1.
- `0003_documents` creates the persisted document registry used by C2.
- `0004_segments` expands document status handling and creates the persisted segment table used by C3.
- Later startups reopen the same encrypted database and skip already applied versions.
- The current bootstrap establishes the minimum operational schema plus persisted project, document, and segment repository tables.

## Runtime expectations

- The desktop shell bootstraps the database during Tauri setup.
- The healthcheck command reports the database path, the applied migrations, and whether the initial schema is ready.
- Project commands persist project metadata and the active project selection through the same encrypted database.
- Document commands persist imported document metadata plus their project association through the same encrypted database, and the copied local file payload is protected at rest with Windows DPAPI before it is written to disk.
- Segment commands read the protected stored payload, normalize it minimally, persist ordered segments against the correct document, and expose ordered segment queries for C4 navigation.
- Rust tests validate first initialization, second initialization without reapplying migrations, the presence of the `projects`, `documents`, and `segments` tables, project persistence across reopen, document persistence across reopen, and persisted segmentation recovery.

## Current limits

- Key protection is currently implemented for Windows with DPAPI because Translat targets Windows 11 at this stage.
- The current SQLCipher toolchain depends on Strawberry Perl during compilation. If the project later changes SQLite encryption strategy, revisit this prerequisite together with the build pipeline.
- If the project later expands beyond Windows, revisit key storage behind the persistence boundary instead of changing repository code ad hoc.
- This stage now adds persisted projects, the first document registry, DPAPI-protected local copied file storage, and deterministic persisted segments. Advanced document parsing, FTS, translation state, and later business tables remain out of scope.
