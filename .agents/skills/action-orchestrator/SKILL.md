# action-orchestrator

## Purpose
Use this skill when implementing or changing the typed AI action system of Translat.

This skill assumes the current Translat operating model:
- `translate_chunk` is the primary operational action for long-form documents
- `translate_segment` is a fine-grained action for local corrections or narrow workflows
- `translate_document` is a batch/job-oriented flow built on top of chunk execution
- editorial context is layered and must be resolved explicitly before execution
- task runs, jobs, and QA findings are part of the traceability backbone

## Canonical references
Before making changes, align with:
1. PRD
2. Technical architecture
3. Data model
4. Detailed AI action contracts
5. `AGENTS.md`

If any old code or docs still imply segment-only orchestration, prefer the updated product documents.

## When to use
- adding a new AI action
- changing `ActionRequest`, `ActionEstimate`, `ActionResult`, or execution modes
- updating `ActionRegistry`, `ActionHandler`, `ContextBuilder`, `OutputValidator`, or rule resolution flows
- modifying `task_run` persistence tied to AI execution
- changing batch, resumable, or job-driven translation behavior
- wiring `translate_chunk`, `translate_segment`, `translate_document`, `run_consistency_qa`, or `estimate_cost`

## Expected outcomes
- AI work remains routed through typed actions
- `translate_chunk` is treated as the main operational translation path for long-form content
- context assembly stays explicit, layered, and reusable
- outputs are validated before mutating state
- task runs remain the traceability backbone
- jobs are resumable and auditable
- QA side effects remain explicit and persistable

## Working rules
- do not introduce ad hoc calls to the model outside the orchestrator flow
- separate estimation, execution, validation, persistence, and QA side effects
- preserve action versioning and prompt version traceability
- keep `interactive`, `batch`, and `job` modes explicit
- do not collapse segment persistence units with chunk execution units
- resolve active glossary layers and active rules by `action_scope` before prompt assembly
- preserve explicit linkage between `task_run`, `chunk_id`, `job_id`, and `qa_findings`
- document structural changes if action contracts or orchestration logic evolve

## Specific assumptions for Translat
### Persistence unit vs execution unit
- segment = atomic persistence unit
- chunk = operational translation unit

### Context model
The orchestrator must assume editorial context may include:
- master glossary
- work glossary
- project glossary
- active glossary layer precedence
- active style profile
- active rules for the current action scope
- translation memory
- nearby segment/chunk context
- chapter/section accumulated context
- user comment or revision note when applicable

### Action scope
Rules and validators must be able to differ for:
- `translation`
- `retranslation`
- `qa`
- `export`
- `consistency_review`

## Recommended checklist
1. Confirm whether the change belongs to action definition, handler, context resolution, validation, persistence, or QA side effects.
2. Verify whether the primary path should be `translate_chunk` rather than `translate_segment`.
3. Ensure the action contract stays typed and versioned.
4. Ensure glossary layers and action-scoped rules are resolved explicitly.
5. Keep side effects explicit and auditable.
6. Confirm `task_runs` preserve links to `segment_id`, `chunk_id`, `job_id`, and prompt/action versions when relevant.
7. Confirm job flows remain resumable and partial failures remain inspectable.
8. Update the corresponding product documentation if the action contract or orchestration flow changes.

## Red flags
- a new model call added directly in UI or random service code
- context assembled implicitly from project defaults only
- `translate_segment` used as the default large-document workflow
- batch and job execution treated as the same thing
- outputs persisted before structural validation
- QA findings returned informally instead of through typed artifacts
