---
name: frontend-ui-ux
description: Guide Translat frontend UI/UX work for planning, auditing, Figma design, implementation, and review. Use when Codex is asked to redesign or improve the Translat UI, create frontend UX plans, evaluate visual quality, implement React/Tauri workspace layout changes, update frontend design guidance, or validate browser/desktop UI states.
---

# frontend-ui-ux

## Purpose
Use this skill for Translat frontend UI/UX work. It keeps visual decisions aligned with the product model: document-first operation, chunk-based translation, layered editorial context, job traceability, QA review, and human-in-the-loop editing.

## Canonical Inputs
Before structural UI decisions, consult in order:
1. PRD
2. technical architecture
3. data model
4. detailed AI action contracts
5. backlog / roadmap
6. `AGENTS.md`
7. `docs/product/translation-workspace-frontend-workflow.md`
8. `docs/product/frontend-design-system-foundations.md`
9. `docs/product/frontend-ui-ux-redesign-guidelines.md`

## Figma Rule
State one of these in every UI-related task:
- `Figma required`: new workflows, primary screens, multi-panel layout changes, operational state design, reusable component families, or implementation-shaping UX decisions.
- `Figma not required`: small bug fixes, copy-only changes, spacing tweaks, minor responsive fixes, or technical refactors without meaningful UX change. Include a short justification.

For Release 08 redesign work, assume `Figma required` unless the task is explicitly narrow.

## Working Model
Design the app as a desktop translation workstation, not a generic admin dashboard.

Keep these distinctions visible:
- project: workspace container
- document: primary operating object
- section/chapter: structural orientation
- segment: atomic persisted review unit
- translation chunk: AI execution and inspection unit
- job: execution envelope with progress, cancellation, resume, and incidents
- QA finding: review anchor that can lead to focused correction
- glossary/style/rules: editorial artifacts, not technical settings only

## UI/UX Priorities
Prioritize:
- clear navigation over showing every panel at once
- document state and next action over raw metadata
- chunk inspection over segment-only translation assumptions
- job and QA status as persistent operational context
- short in-app labels over explanatory documentation inside the UI
- dense but scannable layouts for professional desktop use
- visible disabled/loading/error states for every operational action

Avoid:
- turning the app into a long page of cards
- hiding document/job status while inspecting a chunk
- making debug panels part of the primary user experience
- using decorative visuals that do not clarify workflow state
- flattening glossary layers, style profiles, and action-scoped rules into generic notes

## Implementation Guidance
When implementing frontend changes:
1. Identify the user workflow state first: no project, no document, imported, segmented, chunk-ready, job running, review-ready, incidents, exportable.
2. Decide the primary action for that state.
3. Keep status visible while changing the selected chunk, finding, or document.
4. Use existing command wrappers and shared types from `src/shared/desktop.ts` and `src/frontend/lib/desktop.ts`.
5. Prefer local reusable UI primitives only when they reduce real duplication or clarify state language.
6. Do not add broad frontend architecture unless the current components make the UX impossible to express cleanly.

## Validation
Use the checklist in `references/review-checklists.md` for audits, implementation reviews, and before closing UI tasks.

Minimum checks for code changes:
- run the relevant frontend checks (`npm run lint`, `npm run typecheck`) when code changes
- inspect the UI in the in-app browser when available
- validate the Tauri desktop window when the Rust/Tauri toolchain is healthy
- document runtime blockers separately from visual defects

