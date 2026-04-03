# Frontend

This area contains the React + TypeScript user interface that runs inside the Tauri desktop shell.

Current scope:
- persisted project creation, listing, and opening from C1,
- project-scoped document import and persisted document listing for C2,
- document processing triggers plus document status and segment-count feedback for C3,
- reusable frontend wrapper for Tauri command invocation,
- normalized desktop command errors and healthcheck state handling,
- and a workspace layout that stops before advanced segment navigation in C4.

Planned responsibility areas:
- project workspace UI
- document and segment navigation
- glossary, style, and rule management UI
- translation workspace UI
- historical review and diff UI
- corpus and search UI
