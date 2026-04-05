# tauri-react-rust-foundation

## Purpose
Use this skill when working on the foundational desktop stack of Translat based on Tauri, React, TypeScript, and Rust.

This skill applies to work that shapes the desktop shell, typed command boundaries, module layout, workflow surfaces, and the interaction between UI, backend orchestration, persistence, and document translation flows.

## When to use
- bootstrapping or extending the desktop shell
- organizing frontend/backend boundaries
- wiring or refactoring Tauri commands
- defining initial or evolved module layout
- setting up shared contracts between UI and Rust services
- introducing chunk-aware document workflows in the desktop app
- exposing document, section, chunk, QA, or job-oriented views in the UI

## Product assumptions
Agents using this skill must assume:

- **segment** is the atomic persistence unit
- **translation chunk** is the primary operational translation unit for long-form documents
- long documents must support **batch/job** flows rather than one-shot translation
- editorial context is layered and may include project defaults, glossary layers, style, rules, memory, and chapter/section context
- the desktop shell must support inspectable and resumable translation workflows

## Expected outcomes
- changes keep the desktop shell modular
- UI concerns stay in frontend code
- orchestration, persistence coordination, and native/system concerns stay in Rust
- contracts between layers remain explicit
- document, section, chunk, QA, and job flows remain visible in the desktop UX
- the app structure supports chunk-based translation and editorial review, not only segment-level actions

## Working rules
- do not mix UI logic with persistence, chunk building, or LLM orchestration
- keep command interfaces typed, explicit, and minimal
- preserve a clear distinction between interactive actions and batch/job workflows
- prefer small vertical slices over broad rewrites
- document structural decisions when introducing new modules or changing boundaries
- do not model the desktop shell around segment-only translation assumptions
- ensure navigation and state can represent project -> document -> section/chapter -> chunk -> result/QA/job

## Recommended module map
When relevant, prefer an explicit structure that leaves room for:

### Frontend
- `src/modules/projects/`
- `src/modules/documents/`
- `src/modules/translation/`
- `src/modules/chunking/` or chunk-related UI under documents/translation
- `src/modules/context/`
- `src/modules/qa/`
- `src/modules/jobs/`

### Backend (Rust / Tauri)
- `src-tauri/src/modules/documents/`
- `src-tauri/src/modules/chunking/`
- `src-tauri/src/modules/context/`
- `src-tauri/src/modules/actions/`
- `src-tauri/src/modules/persistence/`
- `src-tauri/src/modules/qa/`
- `src-tauri/src/modules/jobs/`

The exact names may evolve, but agents should preserve the architectural direction.

## Recommended checklist
1. Identify whether the change belongs to frontend, Rust backend, or shared contracts.
2. Confirm whether the UX/API surface is segment-level, chunk-level, or document/job-level.
3. Keep the public interface between layers explicit.
4. Ensure the desktop flow still supports inspectable chunk translation, QA, and resumable jobs.
5. Add or update repository structure docs if the module map changes.
6. Verify the change does not bypass the intended action-oriented and chunk-aware architecture.
