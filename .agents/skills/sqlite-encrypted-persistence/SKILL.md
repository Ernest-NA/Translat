# sqlite-encrypted-persistence

## Purpose
Use this skill for work related to Translat local persistence, encrypted SQLite usage, migrations, repositories, and transactional safety.

## When to use
- adding or updating schema and migrations
- implementing repositories
- changing task_runs, segments, corpora, suggestions, or glossary persistence
- reviewing transaction boundaries
- improving local storage safety

## Expected outcomes
- schema changes remain traceable and migration-friendly
- repositories reflect domain language rather than raw SQL semantics
- critical writes happen transactionally
- large payloads are not pushed into the database without justification

## Working rules
- preserve strong traceability for versions, costs, and task runs
- prefer normalized core domain structures over broad JSON blobs
- document indexes and performance-sensitive queries when they matter
- avoid mixing repository responsibilities with business rules

## Recommended checklist
1. Identify affected tables and migration implications.
2. Confirm repository contracts still match domain intent.
3. Protect multi-entity writes with explicit transactions.
4. Update DDL, repository docs, or architecture notes when persistence changes are structural.
