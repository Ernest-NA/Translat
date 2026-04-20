# Translat UI/UX review checklists

## Audit Checklist
- The screen has one clear primary object: project, document, chunk, job, or finding.
- The primary action is obvious for the current state.
- Document readiness is visible before translation actions.
- Chunk status is visible without opening each chunk.
- Job status remains visible while inspecting chunk detail.
- QA findings are actionable and linked to review/correction, not shown as raw logs.
- Debug/trace information is secondary, collapsible, or visually lower priority.
- In-app copy names objects and states; it does not explain the product at length.
- Empty, loading, disabled, warning, and error states are distinct.
- Text does not overflow buttons, pills, cards, or panels.

## Release 08 State Matrix
- No project: create/open project path is clear.
- Project open, no document: import path is clear.
- Document imported: segmentation is the next valid action.
- Document segmented, no chunks: build chunks is the next valid action.
- Chunks ready: translate document is available.
- Job running: progress, current chunk, cancel, and status refresh are visible.
- Job completed: review/export path is visible.
- Job completed with incidents: affected chunks and resume/correction path are visible.
- QA findings present: finding selection, anchor context, and correction action are visible.

## Implementation Review Checklist
- The change preserves segment vs chunk vs document distinctions.
- It does not replace chunk-based document translation with monolithic translation.
- It keeps layered editorial artifacts visible where relevant.
- It keeps rules scoped by action when rules are shown or edited.
- It preserves task/job traceability in UI state.
- It avoids unrelated backend or persistence changes.
- It uses existing local UI conventions unless there is a concrete reason to change them.
- It includes a visual verification note in the final response or PR notes.

