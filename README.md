# Translat

Translat is a Windows 11 desktop application for orchestrating English-to-Spanish translation workflows with AI. It is designed around project workspaces, glossaries, style rules, editorial constraints, EN/ES parallel corpora, translation traceability, and supervised refinement.

## Current status

The foundation already includes:
- a native desktop shell with Tauri and Rust,
- a React and TypeScript frontend rendered inside the desktop container,
- a reusable frontend-backend command pattern,
- encrypted local persistence with SQLite and versioned migrations,
- and baseline shell error handling for the next modules.

## Quick start

```bash
npm install
npm run dev
```

The local setup guide lives in `docs/runbooks/local-setup.md`.
The current database bootstrap strategy lives in `docs/runbooks/database-bootstrap.md`.
