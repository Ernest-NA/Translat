# Tests

This directory contains automated tests and validation notes for Translat.

Current validation baseline:
- frontend formatting and linting through Biome,
- frontend type checking with TypeScript,
- native desktop smoke coverage through the Rust `healthcheck` unit test,
- reusable Rust fixtures for the document pipeline under `src-tauri/src/test_support`,
- backend smoke coverage for the main document workflow through a Rust end-to-end smoke test,
- and shell validation through `npm run check`, `npm run test`, and `npm run build`.

Planned coverage areas:
- repository and persistence tests
- action orchestrator tests
- prompt and contract tests
- translation workflow tests
- UI and integration tests where applicable
