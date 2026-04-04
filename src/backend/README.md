# Backend

This area is reserved for Translat backend modules and services that will grow beyond the initial shell.

For the current foundation stage, the executable Rust entrypoint and the encrypted SQLite bootstrap live in `src-tauri/`.

Planned responsibility areas:
- command handling and desktop integration
- persistence bootstrap, migrations, and repositories
- action orchestrator and typed AI actions
- context building and validation
- corpus alignment and search services
- job queue and background execution

Current C1/C2/C3/C4/C5 backend slice in `src-tauri/`:
- encrypted SQLite bootstrap with versioned migrations,
- project repository wiring for create/list/open flows,
- document repository wiring for import/list flows inside a project,
- segment repository wiring for deterministic normalization and persisted segmentation,
- document-section repository wiring for a minimal persisted outline over segmented documents,
- glossary repository wiring for create/list/open/update flows with logical archive state,
- glossary-entry repository wiring for per-glossary create/list/update flows with persisted variants and forbidden terms,
- style-profile repository wiring for reusable editorial-profile create/list/open/update flows,
- desktop commands that expose project, document, segment, and document-structure state to the frontend,
- and read-side segment queries that can backfill document structure without mutating document text.
