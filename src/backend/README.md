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

Current C1/C2/C3 backend slice in `src-tauri/`:
- encrypted SQLite bootstrap with versioned migrations,
- project repository wiring for create/list/open flows,
- document repository wiring for import/list flows inside a project,
- segment repository wiring for deterministic normalization and persisted segmentation,
- and desktop commands that expose the minimal project, document, and processing workspace state to the frontend.
