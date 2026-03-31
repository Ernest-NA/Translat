# Tests

This directory contains automated tests and validation notes for Translat.

Current validation baseline:
- frontend formatting and linting through Biome,
- frontend type checking with TypeScript,
- native desktop smoke coverage through the Rust `healthcheck` unit test.

Planned coverage areas:
- repository and persistence tests
- action orchestrator tests
- prompt and contract tests
- translation workflow tests
- UI and integration tests where applicable
