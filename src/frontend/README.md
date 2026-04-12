# Frontend

This area contains the React + TypeScript user interface that runs inside the Tauri desktop shell.

Current scope:
- persisted project creation, listing, and opening from C1,
- project-scoped document import and persisted document listing for C2,
- document processing triggers plus document status and segment-count feedback for C3,
- segmented document opening, ordered segment listing, and minimal segment detail for C4,
- persisted section-outline orientation for segmented documents in C5,
- persisted glossary creation, listing, opening, editing, and archiving for D1,
- per-glossary terminology entry creation, listing, editing, and basic variants/forbidden terms for D2,
- reusable style-profile creation, listing, opening, editing, and archiving for D3,
- reusable rule-set creation, listing, opening, editing, and per-rule management for D4,
- project-level default glossary, style-profile, and rule-set association management for D5,
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

Current workflow reference:
- [Translation Workspace frontend workflow](../../docs/product/translation-workspace-frontend-workflow.md)
- [Frontend design system foundations](../../docs/product/frontend-design-system-foundations.md)
