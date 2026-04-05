# sqlite-encrypted-persistence

## Purpose
Use this skill for work related to Translat local persistence, encrypted SQLite usage, migrations, repositories, and transactional safety.

This skill assumes the current Translat data model:
- segment = atomic persistence unit
- translation chunk = operational translation unit
- editorial context must be reproducible
- jobs, task runs, and QA findings are first-class traceability artifacts

## Canonical references
Before making persistence changes, align with:
1. PRD
2. Technical architecture
3. Data model
4. Detailed AI action contracts
5. `AGENTS.md`

## When to use
- adding or updating schema and migrations
- implementing repositories
- changing `task_runs`, segments, chunks, jobs, corpora, QA findings, glossary layers, or translation memory persistence
- reviewing transaction boundaries
- improving local storage safety

## Expected outcomes
- schema changes remain traceable and migration-friendly
- repositories reflect domain language rather than raw SQL semantics
- critical writes happen transactionally
- editorial and execution traceability remain reproducible
- large payloads are not pushed into the database without justification

## Working rules
- preserve strong traceability for versions, costs, task runs, jobs, and QA findings
- prefer normalized core domain structures over broad JSON blobs
- use JSON only for bounded metadata or explicitly flexible policy/config fields
- document indexes and performance-sensitive queries when they matter
- avoid mixing repository responsibilities with business rules
- do not flatten chunk relationships back into segment-only persistence shortcuts
- preserve reproducible links between glossary layers, task runs, and execution artifacts

## Current entities to keep in mind
At minimum, persistence work may now touch:
- `projects`
- `documents`
- `document_sections`
- `segments`
- `translation_chunks`
- `translation_chunk_segments`
- `chapter_contexts`
- `glossaries`
- `glossary_entries`
- `project_glossary_layers`
- `style_profiles`
- `rule_sets`
- `rules`
- `translation_memory_entries`
- `task_runs`
- `job_queue`
- `qa_findings`

## Recommended checklist
1. Identify affected tables and migration implications.
2. Confirm whether the change affects atomic persistence, operational chunking, or both.
3. Confirm repository contracts still match domain intent.
4. Protect multi-entity writes with explicit transactions.
5. Preserve linkage between `segment_id`, `chunk_id`, `job_id`, and `task_run_id` where applicable.
6. Confirm editorial context can be reconstructed from persisted state.
7. Update DDL, repository docs, or architecture notes when persistence changes are structural.

## Specific guidance
### Editorial layers
Changes involving glossary activation must preserve:
- glossary scope (`master`, `work`, `project`)
- precedence ordering
- active/inactive state per project

### Chunking
Chunk persistence must preserve:
- chunk membership
- segment ordering
- chunk strategy or configuration when relevant
- enough metadata to support reproducible re-runs and QA

### QA
QA findings should remain persistable and queryable by:
- project
- document
- section
- segment
- chunk
- task run
- severity/status

### Jobs
Document jobs should be resumable and inspectable. Persist enough state to understand:
- what was scheduled
- what was completed
- what failed
- what can be retried

## Red flags
- storing chunk execution as unstructured JSON without segment linkage
- losing the ability to reconstruct active glossary layers
- task runs without stable references to chunk/job scope
- QA findings emitted but not persistable
- schema shortcuts that make resumable document jobs harder later
