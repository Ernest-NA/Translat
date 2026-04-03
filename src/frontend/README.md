# Frontend

This area contains the React + TypeScript user interface that runs inside the Tauri desktop shell.

Current scope:
- persisted project creation, listing, and opening from C1,
- project-scoped document import and persisted document listing for C2,
- document processing triggers plus document status and segment-count feedback for C3,
- segmented document opening, ordered segment listing, and minimal segment detail for C4,
- reusable frontend wrapper for Tauri command invocation,
- normalized desktop command errors and healthcheck state handling,
- and a workspace layout that still stops before segment editing, translation actions, and advanced review flows.

Planned responsibility areas:
- project workspace UI
- document and segment navigation
- glossary, style, and rule management UI
- translation workspace UI
- historical review and diff UI
- corpus and search UI
