# TR-30 Information Architecture

## Scope
Figma required.

TR-30 defines the target information architecture and navigation model for the Release 08 Translat UI/UX redesign. It turns the TR-29 audit findings into a structural target for Figma-driven implementation work.

This task does not implement frontend code. It fixes the navigation model, screen grouping, persistent surfaces, and operational states that TR-32 and TR-33 should implement.

## Figma / FigJam artifacts
- [TR-30 Translat Navigation IA Map](https://www.figma.com/online-whiteboard/create-diagram/920bb8e4-d330-4d97-b2f3-8e2f4839468d?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=7ac97ca2-39dc-49b6-aa16-bdc826aff2d2)
- [TR-30 Translat Operational State Map](https://www.figma.com/online-whiteboard/create-diagram/c751e2b9-23d7-444a-9585-7fb5111c080e?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=a38a6d9e-4419-4b21-a09a-2e89914e3007)

These FigJam diagrams are the structural design artifact for TR-30. TR-31 should build the applied visual system on top of this structure.

## Design thesis
Translat should behave like a document translation workstation, not a vertically stacked admin page.

The redesigned shell should always make one primary operating object clear:
- project
- document
- translation chunk
- job
- QA finding

Everything else is supporting context, library management, or diagnostics.

## Main navigation
Use a persistent shell navigation with five top-level areas:

1. Projects
   - create project
   - open recent or persisted project
   - inspect active project summary
   - enter the document workspace

2. Document Workspace
   - import documents
   - inspect document list
   - segment/process a document
   - build translation chunks
   - evaluate document readiness

3. Translation Workspace
   - inspect the active document readiness header
   - browse chunk list or timeline
   - inspect selected chunk source, context, result, and incident
   - monitor active or last job
   - review QA findings and export readiness

4. Editorial Libraries
   - manage glossaries
   - manage style profiles
   - manage rule sets and action-scoped rules
   - assign project default editorial artifacts

5. Diagnostics
   - inspect runtime health
   - inspect command bridge status
   - inspect operational trace
   - inspect raw ids, task runs, and export snapshots

## Navigation decisions
- Translation Workspace remains the center of the product once a project and document exist.
- Editorial Libraries are not first-screen peers of active document work. They are navigable support areas and contextual drawers/links from project defaults.
- Diagnostics are not part of the primary translator/reviewer path. They are a separate mode or secondary panel.
- Project creation/opening is the entry point, but it should not dominate after a project is active.
- QA and Review are reachable from Translation Workspace because findings are anchored to document/chunk/job results.

## Persistent shell surfaces
Always visible in the shell:
- active project name or no-project state
- active document name or no-document state
- global runtime indicator in compact form
- primary navigation
- current workspace primary action

Visible in Translation Workspace:
- active document readiness and next action
- chunk navigation
- selected chunk detail
- active or last job monitor
- QA finding count and review path

Visible on demand:
- full raw ids
- command wrapper names
- full operational trace
- full context-builder payload
- healthcheck details beyond a compact runtime status

## Primary workflow state matrix
| State | Primary object | Primary action | Required visible context |
|---|---|---|---|
| No project | Project | Create or open project | Recent projects, runtime status |
| Project open, no document | Document | Import document | Active project, editorial defaults summary |
| Document imported | Document | Segment document | Document status, file metadata |
| Document segmented, no chunks | Document | Build chunks | Segment count, section outline |
| Chunk-ready document | Document / chunk | Translate document | Chunk count, selected chunk, editorial context summary |
| Job running | Job / chunk | Monitor or cancel | Progress, current chunk, chunk statuses |
| Completed with incidents | Job / finding | Inspect affected chunks | Failed chunks, resume path, finding anchors |
| Review-ready | Finding / chunk | Review findings or export | QA findings, translated results, export readiness |
| Export-ready | Document | Export markdown | Reconstruction status, open finding count |
| Diagnostics mode | Runtime / trace | Inspect trace | Command bridge, task runs, raw ids |

## Target Translation Workspace layout
The Translation Workspace should be a stable four-zone workspace:

1. Workspace header
   - active document identity
   - document readiness state
   - primary action for the current state
   - compact job progress summary
   - QA/export summary

2. Left rail
   - document selector or compact document list
   - chunk list/timeline for the active document
   - chunk status filters
   - incident-first ordering when errors exist

3. Center detail
   - selected chunk source
   - selected chunk context preview
   - latest result
   - incident panel when applicable
   - segment trace as tab, drawer, or linked detail

4. Right rail
   - job monitor
   - task-run summary
   - unresolved incidents
   - QA findings list or finding handoff
   - collapsible after completion, persistent while running

## Editorial Libraries architecture
Editorial artifacts stay distinct:
- glossary layers are terminology sources
- style profiles are editorial voice and treatment controls
- rules are operational constraints scoped by action

Do not collapse these into a generic settings page. The navigation can group them under Editorial Libraries, but the UI must keep their separate purpose visible.

Project defaults should be edited from:
- the active project summary, as a compact default-link panel
- the Editorial Libraries area, where each artifact can be created or maintained

## Diagnostics architecture
Diagnostics should contain:
- runtime health
- command bridge availability
- web-preview/Tauri-unavailable state
- operational trace
- selected job id and task runs
- export snapshots

The main UI should show only compact diagnostics:
- runtime available / unavailable
- web preview mode
- active job has incidents

Raw errors such as `Cannot read properties of undefined (reading 'invoke')` must not appear as primary product errors. In browser-only preview, actions that require Tauri should show a clear unavailable state.

## State-specific screen requirements for TR-32/TR-33
No project:
- focus on create/open project
- do not show rule/style/glossary management as primary content

Project open, no document:
- focus on import document
- show project editorial defaults as a compact summary

Document imported:
- focus on segmentation
- do not expose translate actions yet

Document segmented:
- focus on build chunks
- show section/segment readiness

Chunk-ready:
- focus on translate document
- show chunk list and selected chunk inspection

Job running:
- keep job monitor visible
- keep chunk detail selectable
- expose cancel/status refresh without hiding progress

Completed with incidents:
- prioritize affected chunks and resume/correction path
- link findings to chunk and segment context

Review-ready:
- prioritize findings, result inspection, and export readiness

Diagnostics:
- provide traceability without occupying the primary workflow

## Implementation guidance carried forward
- Build a shell with navigation first; do not continue stacking every module on the first page.
- Preserve the existing product model: document is the operating object, chunk is the translation unit, segment is the persisted review unit, job is the execution envelope.
- Move implementation-stage copy out of the main UI.
- Keep `PanelHeader`, `StatusBadge`, `ActionButton`, and `PanelMessage` as useful primitives, but TR-31 should revise their visual density and surface rules.
- Treat `OperationalDebugPanel` and healthcheck as diagnostics surfaces, not primary workspace panels.
- Keep QA findings actionable and anchored to chunk/segment context.

## Out of scope
- Final visual styling.
- Component token redesign.
- Frontend code implementation.
- Backend workflow changes.
- New translation behavior.

## Acceptance check
- Figma/FigJam shows the target navigation map.
- Figma/FigJam shows required operational states.
- The repo documents top-level navigation, persistent surfaces, state matrix, and panel demotion/grouping.
- The model preserves document -> chunk -> job -> QA and does not introduce monolithic document translation.
- Debugging and raw technical state are separated from the primary user experience.

