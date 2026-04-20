# Frontend UI/UX redesign guidelines

## Purpose
TR-28 prepares agents and maintainers for Release 08 UI/UX redesign work.

The current frontend is functionally broad but visually overloaded. Release 08 should turn it into a professional desktop translation workstation without breaking the product model already established by the PRD, architecture, data model, and AI action contracts.

## Figma policy
Figma required.

Release 08 changes affect structural UI, navigation, multi-panel layout, reusable components, and operational state design. Figma or Figma MCP must be used before implementing broad shell or Translation Workspace changes.

Figma is not required only for narrow fixes such as copy edits, small spacing corrections, or technical refactors that do not reshape UX.

## Product model to preserve
- Segment is the atomic persisted review unit.
- Translation chunk is the AI execution and inspection unit.
- Document is the primary operating object.
- Job is the execution envelope for document translation.
- QA findings are review anchors, not afterthought logs.
- Glossaries, style profiles, and rules are editorial artifacts with operational effect.
- Human review, cost awareness, auditability, and controlled reuse remain mandatory.

## Target experience
The UI should make this path understandable:

Project -> document -> segmentation -> chunks -> document translation -> job monitor -> QA/review -> export.

The user should not need to read long explanatory copy inside the app to understand the next action.

## Release 08 task flow
1. TR-28: prepare agent/frontend UI-UX guidance.
2. TR-29: audit the current UI visually and functionally.
3. TR-30: define information architecture and navigation in Figma.
4. TR-31: define the applied visual system for shell and workspace.
5. TR-32: implement redesigned shell and primary navigation.
6. TR-33: rebuild Translation Workspace hierarchy around document, chunk, job, and QA.
7. TR-34: validate visual quality, responsive behavior, and operational states.

## Design constraints
- Do not make a landing page; the first screen should be the usable product.
- Do not stack all product modules as a long sequence of equal cards.
- Do not use debug panels as the primary user experience.
- Do not hide job progress when a chunk or finding is selected.
- Do not collapse glossary, style, and rules into one generic settings area unless their distinct roles remain clear.
- Do not add decorative backgrounds or one-note palettes that reduce readability.
- Keep desktop density high enough for professional work, but preserve clear grouping and hierarchy.

## Visual validation baseline
Use the in-app browser for visual inspection whenever it is available.

Validate at least:
- empty app / no project
- project open / no document
- document imported
- document segmented
- chunk-ready document
- selected chunk
- job running
- job completed
- incidents / failed chunks
- QA findings
- export-ready state

When Tauri cannot run, document that separately from visual findings. Browser-only validation can confirm layout and state presentation, but not native command behavior.

## Related references
- `AGENTS.md`
- `.agents/skills/frontend-ui-ux/SKILL.md`
- `docs/product/translation-workspace-frontend-workflow.md`
- `docs/product/frontend-design-system-foundations.md`
- `docs/runbooks/frontend-quality.md`

