# TR-31 Applied Visual System

## Scope
Figma required.

TR-31 defines the applied visual direction for the Release 08 shell and Translation Workspace. It extends TR-24 from a minimal component foundation into an implementation-ready visual system for a professional editorial translation workstation.

This task does not implement frontend code. It fixes the visual rules that TR-32 and TR-33 should apply.

## Figma / FigJam artifacts
- [TR-31 Translat Applied Visual System](https://www.figma.com/online-whiteboard/create-diagram/5b9301bf-0e81-48c8-aa05-e851cf9cdc15?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=61d095ed-8ac3-4e3d-929c-44845990735c)
- [TR-31 Translat Screen State Visual Coverage](https://www.figma.com/online-whiteboard/create-diagram/476c6055-271f-4cd6-b244-acada480e38f?utm_source=chatgpt&utm_content=edit_in_figjam&oai_id=&request_id=cad55a03-c991-4c64-bb7c-402db35d38af)

These artifacts complement the TR-30 navigation and operational state maps. TR-30 defines where things live; TR-31 defines how they should look and behave visually.

## Visual thesis
Translat should read as a precise desktop workstation:
- restrained
- dense
- scannable
- state-led
- document-first
- built for long editing/review sessions

The UI should not look like a marketing dashboard, a documentation page, or a collection of decorative cards.

## Palette direction
Use a neutral dark workstation palette with restrained semantic accents.

Target roles:
- `bg-base`: near-black blue-gray workspace background
- `bg-shell`: fixed navigation and topbar background
- `bg-surface`: primary work surface
- `bg-raised`: selected/active panels
- `bg-subtle`: rows, inactive chips, secondary panels
- `border-subtle`: default separators
- `border-strong`: selected or focused separators
- `text-primary`: primary reading text
- `text-secondary`: panel support text
- `text-muted`: metadata labels

Semantic tones:
- `info`: active progress, linked metadata, current chunk
- `success`: ready, completed, exported, resolved
- `warning`: blocked, stale, incomplete, needs action
- `danger`: failed, incident, destructive action
- `neutral`: inactive, idle, historical, secondary metadata

Avoid:
- radial decorative backgrounds
- large blue/orange gradients
- one-note blue/purple surface systems
- blurred glass panels as a default surface

## Surface hierarchy
TR-32/TR-33 should implement fewer and clearer surface levels:

1. App shell
   - fixed layout frame
   - no decorative card treatment
   - contains navigation, top context, and active workspace

2. Workspace surface
   - main active route content
   - one per screen
   - not nested in another card

3. Panel
   - functional regions such as chunk rail, chunk detail, job rail, QA rail
   - subtle border, small radius, minimal shadow or none

4. Row / item
   - repeated selectable units such as projects, documents, chunks, findings, task runs
   - compact height, state marker, no heavy card styling

5. Overlay / drawer
   - diagnostics, raw ids, full context payload, detailed trace
   - used for secondary inspection

Rules:
- Do not place cards inside cards.
- Do not give every page section the same border/radius/shadow.
- Reserve high emphasis for the active document state, current chunk, running job, and unresolved incidents.

## Radius and density
Current radii are too large for the target product. Use:
- `4px`: tables, compact rows, input groups
- `6px`: buttons, status chips, row selection
- `8px`: panels, dialogs, repeated cards
- `12px`: rare large shell containers only

Spacing target:
- `4px`: icon/text gaps, tight metadata
- `8px`: row internals, chip groups
- `12px`: panel internals
- `16px`: panel gaps
- `20px`: workspace section gaps
- `24px`: shell gutters

Avoid defaulting to `24px+` internal padding for every panel. Professional density matters because the user must compare document, chunk, job, and QA state at once.

## Typography
Use one practical system font stack:
- `"Segoe UI Variable", "Segoe UI", system-ui, sans-serif`

Target type scale:
- app title / route title: `24-28px`
- workspace title: `20-22px`
- panel title: `15-17px`
- body: `14px`
- dense row body: `13px`
- metadata label: `11-12px`

Rules:
- Letter spacing should be `0` for normal UI text.
- Use uppercase sparingly for short metadata labels only.
- Do not use large hero typography inside the product shell.
- Source/target translation text should prioritize comfortable reading line-height over compact metadata density.

## Shell layout
The redesigned shell should use:
- fixed left navigation rail
- compact top context bar
- active workspace content area
- optional right diagnostics drawer only when requested

Left navigation:
- Projects
- Documents
- Translation
- Editorial Libraries
- QA / Review
- Diagnostics

Top context bar:
- active project
- active document
- document state
- compact runtime status
- current job state when present

Primary workspace:
- shows exactly one active mode
- never stacks every module vertically
- exposes one clear primary action for the current state

## Translation Workspace visual layout
Use the TR-30 four-zone model:

1. Header
   - active document name
   - readiness state
   - primary action
   - compact progress
   - QA/export summary

2. Left rail
   - document selector if needed
   - chunk list/timeline
   - status filters
   - incident-first ordering when needed

3. Center detail
   - selected chunk source/context/result/incident
   - source and result text must be readable
   - use tabs or segmented controls for Source, Context, Result, Incident

4. Right rail
   - job monitor while running
   - QA findings when review-ready
   - incidents when unresolved
   - collapsible after completion, never hidden while running

## Component direction
Keep the TR-24 primitives, but revise their appearance and usage:

### PanelHeader
- compact by default
- no long explanatory descriptions in primary screens
- title plus optional one-line support text
- action cluster aligned to the current state

### StatusBadge
- compact chips for state only
- neutral metadata should be text, not badges
- strong emphasis only for current job, active document readiness, incidents, and selected chunk

### ActionButton
- primary: one per active workflow state
- secondary: standard outline or subtle fill
- destructive: danger tone, not gradient
- diagnostic: quiet/secondary style
- avoid pill-only visual language for every command

### PanelMessage
- use for empty, warning, disabled, and error states
- short title plus one action-led sentence
- no implementation history or release notes

### Lists and rows
- project/document/chunk/finding/task-run rows should share a compact row language
- selected row uses border + background + left state marker
- failed rows use danger marker and incident summary
- running row uses info marker and progress cue

## State language
State colors and copy should be consistent:

| State family | Visual tone | Copy style |
|---|---|---|
| Ready / completed / resolved | success | "Ready", "Completed", "Resolved" |
| Running / active / selected | info | "Running", "Current", "In progress" |
| Blocked / stale / missing setup | warning | "Needs chunks", "Needs refresh", "Blocked" |
| Failed / incident / destructive | danger | "Failed", "Incident", "Cancel" |
| Idle / unavailable / historical | neutral | "No job", "Idle", "Not available" |

Rules:
- show the reason for disabled actions near the action
- distinguish web-preview unavailable from product/backend failure
- never show raw JavaScript errors as the primary user-facing message

## Copy rules
Replace implementation-stage copy with operational labels.

Avoid in primary UI:
- "D4 adds..."
- "C1 keeps..."
- "automated execution and AI integration out of scope"
- command wrapper names
- raw implementation details

Use:
- "Import document"
- "Build chunks"
- "Translate document"
- "Review findings"
- "Export markdown"
- "Runtime unavailable in web preview"
- "No document selected"

Help text should be:
- one sentence
- action-led
- tied to the current missing requirement

## Responsive desktop and mobile
Primary target remains desktop. Minimum responsive behavior:

Wide desktop:
- left nav fixed
- Translation Workspace uses header + left rail + center detail + right rail

Narrow desktop / tablet:
- left nav collapses to icons or compact labels
- right rail becomes collapsible drawer
- chunk rail and detail remain visible as two columns when possible

Mobile:
- not a full productivity target, but must not overflow
- top context becomes stacked compact summary
- workspace zones become tabs: Document, Chunks, Job, QA
- diagnostics hidden behind explicit navigation

## Applied screen coverage
TR-32/TR-33 should implement and visually validate:

1. No project
   - primary action: create/open project
   - no editorial library panels on first screen

2. Project open, no document
   - primary action: import document
   - project defaults shown as compact summary

3. Document imported
   - primary action: segment document
   - document metadata visible

4. Document segmented, no chunks
   - primary action: build chunks
   - segment/section readiness visible

5. Chunk-ready document
   - primary action: translate document
   - chunk rail and selected chunk detail visible

6. Job running
   - progress and current chunk visible
   - cancel/refresh available
   - job rail persistent

7. Completed with incidents
   - affected chunks first
   - resume/correct path visible
   - warning/danger hierarchy clear

8. Review-ready
   - QA findings visible
   - result inspection and export readiness visible

9. Diagnostics
   - runtime bridge state, trace tables, copyable ids
   - not mixed with primary translation workflow

## Implementation tokens to introduce in TR-32/TR-33
Suggested token direction:

```css
:root {
  --color-bg-base: #101418;
  --color-bg-shell: #151a20;
  --color-bg-surface: #1b2128;
  --color-bg-raised: #222a33;
  --color-bg-subtle: #171d23;
  --color-border-subtle: #2b333d;
  --color-border-strong: #44505e;
  --color-text-primary: #f2f4f6;
  --color-text-secondary: #b9c0c8;
  --color-text-muted: #858f99;
  --color-info: #6bb8ff;
  --color-success: #5fc891;
  --color-warning: #e6b450;
  --color-danger: #ef6f64;
  --radius-row: 4px;
  --radius-control: 6px;
  --radius-panel: 8px;
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 20px;
  --space-6: 24px;
}
```

These are implementation starting points, not a requirement to preserve every current CSS variable name.

## Relationship to TR-24
TR-24 remains the baseline for reusable primitives and semantic states. TR-31 adjusts the visual direction:
- reduce radius
- remove decorative gradients
- lower shadow/glass usage
- tighten density
- move debug panels out of primary UX
- make rows/lists first-class, not just cards

## Acceptance check
- Figma/FigJam defines applied visual system and screen-state coverage.
- The repo documents palette, typography, surface hierarchy, components, states, copy, density, and responsive behavior.
- The direction supports shell, project/document workspace, Translation Workspace, job/QA, empty/error states, and diagnostics.
- The design preserves segment vs chunk vs document, job traceability, QA findings, layered editorial artifacts, and action-scoped rules.
- The UI direction is a workstation, not documentation pasted into the product.

