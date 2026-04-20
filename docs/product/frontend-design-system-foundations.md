# Frontend design system foundations

## Purpose
TR-24 consolidates the visual language already used by the shell and the Translation Workspace without opening a full product redesign.

The goal is to reduce repeated frontend styling debt and leave a small reusable base for:
- panel headers and toolbar actions
- operational status badges
- loading, empty, warning, and error states
- detail and metric grids
- workspace surfaces used across document, chunk, QA, job, and export inspection

## Operational baseline
This foundation is aligned to the current frontend reality in `develop`, especially:
- `AppShell`
- `ProjectWorkspace`
- `DocumentList`
- `DocumentImporter`
- `ChunkBrowser`
- `FindingReviewPanel`
- `TranslationJobMonitor`
- `OperationalDebugPanel`
- `SegmentBrowser`

TR-24 intentionally keeps the product recognizable and incremental:
- no new shell
- no new navigation mode
- no exhaustive enterprise component library
- no TR-27-style performance or architecture work

## Consolidated foundations

### Tokens
TR-24 makes the following frontend foundations explicit in `src/frontend/styles.css`:
- spacing scale: `8 12 16 20 24 28 32`
- surface radii: `16 18 24 28`
- text roles: eyebrow, body, title, hero
- semantic tones: `neutral`, `info`, `success`, `warning`, `danger`
- shared border, panel, and elevated surface colors

### Reusable UI primitives
TR-24 introduces a minimal reusable UI layer in `src/frontend/components/ui/`:
- `PanelHeader`
- `StatusBadge`
- `ActionButton`
- `PanelMessage`

These primitives are meant to cover the repeated shell/workspace patterns already present in the repo, not to become a large component library on their own.

### Shared state language
TR-24 aligns the visible state system across workspace panels:

| Surface | Canonical states |
|---|---|
| Workspace | `ready`, `blocked`, `running`, `review`, `incidents`, `stale`, `empty` |
| Jobs | `pending`, `running`, `completed`, `completed_with_errors`, `failed`, `cancelled` |
| Chunks | `pending`, `running`, `completed`, `failed`, `cancelled` |
| Findings | severity + resolution status |
| Export / trace | latest snapshot, linked job, open findings, warnings |

The key visual rule is:
- success = completed / ready
- info = active progress / context / linked metadata
- warning = blocked / stale / incomplete / incident-adjacent
- danger = explicit failure or error path

## Figma references
TR-24 reuses the lightweight workflow artifacts from TR-16 and adds one new consolidation artifact.

### Previous workflow references from TR-16
- [TR-16 Translation Workspace Wireflow](https://www.figma.com/online-whiteboard/create-diagram/59c4d33d-8f16-48b1-bc90-8cb781067e12?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=1b7c6f03-dcb2-4209-927a-e35e1df4c87a)
- [TR-16 Workspace Panel Layout](https://www.figma.com/online-whiteboard/create-diagram/7ab4b308-0620-43ce-a419-516d79956a19?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=567b1a30-9fbb-4f8a-a8da-e10508106657)

### New TR-24 artifact
- [TR-24 Frontend Design System Map](https://www.figma.com/online-whiteboard/create-diagram/201230b3-2b9e-481e-b0df-131f9247adb1?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=17c73076-c3e8-4cfe-b424-e641420c0b47)

This new board is intentionally small. It fixes:
- tokens and semantic state colors
- reusable component families
- the mapping from those families to the shell and Translation Workspace surfaces

## Scope left for later
TR-24 does not attempt:
- a full visual redesign of Translat
- a dedicated token build pipeline
- Storybook or a separate design system package
- richer animation or interaction polish
- TR-27 performance or rendering optimization work

## Release 08 visual direction
TR-31 adjusts this foundation for the full Release 08 redesign:

- `docs/product/TR-31-applied-visual-system.md`
- [TR-31 Translat Applied Visual System](https://www.figma.com/online-whiteboard/create-diagram/5b9301bf-0e81-48c8-aa05-e851cf9cdc15?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=61d095ed-8ac3-4e3d-929c-44845990735c)
- [TR-31 Translat Screen State Visual Coverage](https://www.figma.com/online-whiteboard/create-diagram/476c6055-271f-4cd6-b244-acada480e38f?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=cad55a03-c991-4c64-bb7c-402db35d38af)

TR-24 remains the baseline for reusable primitives and semantic state language. TR-31 supersedes the current visual treatment where it calls for lower radius, less decorative gradient usage, denser rows/lists, clearer shell navigation, and diagnostics outside the primary UX.
