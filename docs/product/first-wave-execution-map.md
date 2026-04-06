# First-wave execution map

## Objective
Translate the earliest backlog work into repository-operable slices.

This document preserves early backlog groupings as historical references only.

Any executable work created from this map must use a canonical `TR-*` task identifier in Notion, branch names, pull requests, commits, and release traceability, while keeping the older code under an explicit `Legacy refs` mapping.

## First executable wave
### Foundation and bootstrap
- Legacy ref `A1`: scaffold repository from ForgeSeed
- Legacy ref `A2`: adapt AGENTS and define Git/release workflow
- Legacy ref `A3`: create initial skill set
- Legacy ref `A4`: align repository structure and workflow artifacts

### Technical foundation
- Legacy ref `B1`: create Tauri shell
- Legacy ref `B4`: integrate encrypted SQLite and migrations

### Early document flow
- Legacy ref `C1`: CRUD projects
- Legacy ref `C2`: document import
- Legacy ref `C3`: normalization and segmentation

### Early terminology support
- Legacy ref `D1`: glossary CRUD
- Legacy ref `D2`: glossary entries and variants

### First AI baseline
- Legacy ref `E1`: action registry and action definition
- Legacy ref `E2`: base action orchestrator
- Legacy ref `E3`: context builder
- Legacy ref `E4`: model policy service
- Legacy ref `E5`: estimate_cost
- Legacy ref `E6`: translate_segment
- Legacy ref `E8`: task run persistence

## Repo-oriented interpretation
These tasks should now be executable through:
- a task branch from `develop`
- a PR against `develop`
- Codex guided by `AGENTS.md` and `.agents/skills/`
- repository docs under `docs/`
- backlog traceability through canonical Notion `TR-*` task identifiers, with explicit legacy mappings where needed

## Immediate next practical outcome
After the canonical `TR-*` repository-alignment task that keeps legacy ref `A4`, the repository should be ready to start implementing the technical foundation tasks without relying on a generic starter layout.
