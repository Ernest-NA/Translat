# chunked-document-translation

## Purpose
Use this skill when implementing or refining Translat workflows for long-form document translation based on translation chunks.

This skill assumes:
- long documents should not depend on one-shot monolithic translation
- chunks are the operational translation unit
- segments remain the atomic persistence unit
- chunking may include overlap and structure-aware grouping
- document translation may run interactively, in batch, or as resumable jobs

## Canonical references
Before changing anything, align with:
1. PRD
2. Technical architecture
3. Data model
4. Detailed AI action contracts
5. `AGENTS.md`

## When to use
- implementing chunk builders
- changing chunking strategy or overlap behavior
- wiring chapter/section-aware translation
- implementing resumable document translation jobs
- refining chunk-level traceability
- reviewing whether a feature should operate on segments, chunks, or whole-document jobs

## Expected outcomes
- long-form translation remains chunk-based and traceable
- chunking stays reproducible
- overlap/context rules remain explicit
- chunk results can be validated and persisted cleanly
- document translation jobs remain resumable and inspectable

## Working rules
- do not treat long documents as a single AI request by default
- do not erase the distinction between chunk execution and segment persistence
- keep chunk strategy explicit and configurable
- preserve job-level progress and partial failure visibility
- ensure chunk flows can support editorial QA and human review

## Recommended checklist
1. Clarify whether the change affects chunk building, chunk execution, batch orchestration, or job resumability.
2. Preserve the relation between document structure, segment order, and chunk membership.
3. Keep overlap/context rules explicit.
4. Ensure chunk outputs can be validated against the expected segment set.
5. Preserve `chunk_id`, `job_id`, and `task_run_id` traceability.
6. Update architecture or action-contract docs if chunk behavior changes materially.

## Red flags
- document translation implemented as one monolithic model call
- chunk boundaries implied but not persisted or reproducible
- overlap added implicitly without policy
- batch translation without resumability or partial-failure handling
- QA designed only at segment level for long-form flows
