# Translation Workspace frontend workflow

## Purpose
TR-16 defines the operational frontend workflow for the future Translation Workspace without prematurely implementing `translate_document`, job orchestration, or a full UI rewrite.

This document translates the current backend reality into a concrete frontend workflow that directly prepares:
- `TR-17` document-level translation launch
- `TR-18` progress, resume, and cancellation basics
- `TR-19` a minimal operable Translation Workspace

## Current baseline to respect
The repo already provides the following backend and shell capabilities:
- project-scoped document import and segmentation
- persisted `DocumentSection` and segment navigation
- persisted translation chunks and chunk-to-segment links
- persisted editorial defaults, glossary entries, style profiles, and action-scoped rules
- `build_translation_context`
- `translate_chunk`
- persisted `task_runs`, `chapter_contexts`, and `qa_findings`

The current frontend shell already exposes:
- document intake
- document segmentation
- section/segment navigation
- chunk build/list/detail browsing

The current shell does not yet expose:
- document-wide translation launch
- job monitor UX
- chunk execution states in the workspace
- review-ready translation workspace behavior

## Backend surfaces this workflow is allowed to assume
TR-16 is grounded on the backend that already exists after TR-15. The frontend workflow may rely on:
- document status and segment/chunk readiness from the current project/document shell
- chunk build/list/detail data from `build_document_translation_chunks` and `list_document_translation_chunks`
- context preview data from `build_translation_context`
- per-chunk translation execution via `translate_chunk`
- persisted `task_runs`, `chapter_contexts`, and `qa_findings` for progress, incidents, and later review flows

This means the next frontend block should expose and compose real backend behavior, not invent a disconnected demo flow.

## Translation Workspace workflow
The next workspace should treat the document as the primary operating object, the job as the execution envelope, and the chunk as the drill-in unit.

### Primary wireflow
1. User selects a project and document.
2. Workspace evaluates document readiness:
   - imported
   - segmented without chunks
   - segmented with chunks ready
   - translation in progress
   - review-ready / incidents
3. If the document is not ready:
   - show the blocking state and the next valid action
   - keep document metadata visible
4. If the document is ready:
   - user launches document translation from the workspace header
   - workspace opens a job monitor view anchored to the document
5. While the job runs:
   - global progress remains visible
   - chunk list updates by state
   - selecting a chunk keeps detail/context visible
6. When a chunk completes or fails:
   - the list reflects status immediately
   - the detail panel shows source, context, latest result, or incident
7. When the job ends:
   - completed without incidents -> document moves to review-ready
   - completed with incidents -> document enters review with incidents
   - failed/stopped -> document stays resumable with visible problem points

### Operational stages inside the same workspace
The Translation Workspace should progress through these stages without changing product mode:

| Stage | Trigger | Primary user question | Primary visible surface |
|---|---|---|---|
| Readiness | document selected | "Can I translate this document yet?" | header + readiness summary |
| Launch | document is chunk-ready | "What action starts translation?" | header CTA + job rail seed |
| Execution | translation job active | "What is moving, what is blocked?" | chunk list + job rail |
| Inspection | chunk selected | "Why did this chunk behave like this?" | chunk detail panel |
| Incident handling | chunk/job error | "What failed and what can I resume?" | incident container + job rail |
| Review handoff | job finished | "What is ready and what still needs attention?" | header state + chunk/result detail |

## Views and panels
The Translation Workspace should be a three-zone layout inside the existing shell rather than a separate application mode.

### 1. Workspace header
Persistent region. Always visible for the active document.

Must show:
- document identity: name, id, current state
- readiness badge
- primary action for current state
- global counters or progress summary
- last job status summary when one exists

Typical primary actions by state:
- `Segment` when document is imported
- `Build chunks` when segmented but not chunked
- `Translate document` when chunk-ready
- `View progress` while a job is running
- `Review results` when translation is complete
- `Resume with incidents` when the last run finished with errors

### 2. Left rail: document and chunk navigation
Persistent navigation region.

Must contain:
- document selector or document list
- chunk list or chunk timeline for the active document
- chunk status filtering

Chunk row minimum content:
- chunk sequence
- core segment range
- status badge
- translated / total marker
- incident hint when relevant

Recommended ordering:
- failed or incident-bearing chunks first when a job has issues
- otherwise natural chunk sequence order
- filtering by `all`, `pending`, `running`, `completed`, `error`

### 3. Center panel: chunk workspace detail
Primary inspection region.

Must show the selected chunk:
- chunk summary
- core source text
- overlap/context before and after
- context-builder preview summary
- latest translation result
- incident/error container when applicable

Recommended sub-tabs or stacked sections:
- `Source`
- `Context`
- `Result`
- `Incident`

### 4. Right rail: job monitor
Persistent execution region while translation is active; collapsible afterwards.

Must contain:
- active or last job summary
- task-run progression at chunk granularity
- counters for pending/running/completed/error
- last update timestamp
- resume/retry guidance when incidents exist

Recommended sections:
- job summary
- progress counters
- latest task-run events
- unresolved incidents
- next available action

## Critical states
The following states must be explicitly represented in TR-19 UI and are already fixed by this workflow:

| Entity | State | UI expectation |
|---|---|---|
| Document | Ready to translate | Primary CTA visible, chunk list enabled |
| Document | Translation in progress | Header progress locked to active job, job rail persistent |
| Document | Pending review | Review-ready badge and result-first detail mode |
| Document | Completed with incidents | Warning summary with affected chunk count |
| Document | Empty / no chunks | Empty state with next action guidance |
| Chunk | Pending | Neutral badge, available for inspection |
| Chunk | In progress | Running badge and focusable detail |
| Chunk | Completed | Success badge and result preview |
| Chunk | Error | Error badge and incident panel |
| Job | Running | Global progress, recent task runs, no ambiguous idle state |
| Job | Completed with incidents | Completion status plus unresolved chunk count |
| Job | Failed / stopped | Resume/retry guidance and preserved chunk context |

## Information hierarchy
The workspace must make the following hierarchy explicit:

### Always visible
- active document identity
- document readiness / current state
- primary action
- global progress summary
- chunk navigation

### Visible on selection
- selected chunk source
- selected chunk context
- selected chunk latest result
- selected chunk incident details

### Visible on demand
- full context-builder payload
- raw task-run details
- accumulated chapter context internals

The key rule is:
- document and progress stay persistent
- chunk detail changes with selection
- low-level execution detail stays contextual

## Minimum implementation slices unlocked by this workflow
The workflow is intentionally split so the next tasks can land without reopening structure:

### Slice for TR-17
- add document-level translation launch to the workspace header
- seed a document job record and initial progress summary
- route the first translated chunk results into the existing detail surface

### Slice for TR-18
- introduce running/completed/error job state in the right rail
- add resume/cancel/retry affordances in the monitor
- surface chunk state changes in the left rail without redesigning the shell

### Slice for TR-19
- consolidate the workspace header, left rail, center detail, and right monitor into one operable Translation Workspace
- promote existing chunk/detail components into the new layout
- keep context/result/error inspection inside the selected chunk panel

## Minimum reusable components
TR-19 should build from these components, not from a new visual system.

### Required components
- workspace header
- readiness badge group
- primary action cluster
- progress summary card
- chunk timeline or chunk list row
- chunk status badge
- chunk detail panel
- source/context/result section container
- job monitor card
- task-run event row
- incident/error container
- empty state block

### Component notes
- `DocumentList`, `SegmentBrowser`, and `ChunkBrowser` already provide useful structural seeds.
- TR-19 should evolve those patterns instead of replacing the entire shell layout.
- The chunk row and job event row should share status badge language.

## Interaction criteria
The workspace should obey these interaction rules:

### Navigation
- document change resets chunk/job detail to the new document context
- chunk selection never hides document/job status
- job monitor can be collapsed after completion but not while running

### Focus
- primary focus is document-level execution state
- secondary focus is selected chunk inspection
- tertiary focus is task-run detail and incidents

### Selection behavior
- selecting a chunk updates center detail without leaving the workspace
- if a chunk enters error state while selected, the incident panel becomes the first visible subsection
- after job completion, preserve the last selected chunk if it still exists

### Progress behavior
- global progress must remain readable even while inspecting a chunk
- chunk status changes should not require re-opening the workspace
- the monitor should distinguish `job active` from `no active job but historical runs exist`

### Actions by state
- no translate CTA before chunks exist
- no resume/retry CTA when there is no failed or incident-bearing run
- review-oriented actions appear only after at least one translated result exists

## Figma MCP artifacts
TR-16 used Figma MCP to fix the workflow shape before implementation:

- FigJam wireflow:
  [TR-16 Translation Workspace Wireflow](https://www.figma.com/online-whiteboard/create-diagram/59c4d33d-8f16-48b1-bc90-8cb781067e12?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=1b7c6f03-dcb2-4209-927a-e35e1df4c87a)
- FigJam panel map:
  [TR-16 Workspace Panel Layout](https://www.figma.com/online-whiteboard/create-diagram/7ab4b308-0620-43ce-a419-516d79956a19?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=567b1a30-9fbb-4f8a-a8da-e10508106657)

These artifacts are intentionally lightweight: they fix wireflow and panel structure, not the final visual design system.

What each artifact contributes:
- `Wireflow`: stage transitions from document readiness to execution, incidents, and review handoff
- `Panel Layout`: stable information hierarchy for header, chunk rail, chunk detail, and job monitor

## How TR-16 reduces friction for the next tasks
### TR-17
This workflow fixes where `translate_document` starts, what the user sees before launch, and how document readiness should gate the action.

### TR-18
This workflow fixes where progress, resume, cancellation, and incident handling must live in the UI, so state work can target a concrete shell surface instead of abstract status models.

### TR-19
This workflow fixes the minimum Translation Workspace layout, component inventory, and interaction model, so implementation can stay incremental and avoid a full-shell redesign.

## Decisions carried forward
- The Translation Workspace remains embedded in the existing shell and project workspace.
- Document is the primary operating unit; job is the execution envelope; chunk is the drill-in unit.
- Job monitor is a persistent rail during execution, not a modal.
- Chunk detail is the main inspection surface for context, result, and incidents.
- Review-ready is a document state surfaced after execution, not a separate product area.
- Existing frontend pieces such as document selection, chunk browsing, and detail panes should be evolved in place instead of replaced wholesale.
