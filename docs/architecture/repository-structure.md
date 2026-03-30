# Repository structure

## Objective
Describe the intended operational repository structure for Translat after the initial bootstrap stage.

## High-level layout
- `src/frontend/`: React and TypeScript application code
- `src/backend/`: Rust-side application services and orchestration adapters documentation
- `src/shared/`: shared contracts, DTOs, and cross-layer conventions
- `tests/unit/`: focused automated tests for isolated modules
- `tests/integration/`: integration and workflow-level tests
- `docs/architecture/`: architecture and structure decisions
- `docs/product/`: workflow, backlog execution, and product-facing repository guidance
- `docs/runbooks/`: local setup and operational procedures
- `.agents/skills/`: reusable Codex skills
- `.github/`: workflows, templates, and collaboration automation
- `notion/`: exported references and workspace-oriented notes when needed

## Structural intent
This structure is meant to bridge the gap between:
- the architecture already defined in Notion,
- the execution workflow in GitHub,
- and the actual file system shape that Codex and humans will work with.

## Immediate next use
This structure supports the first implementation waves for:
- technical foundation
- project/document ingestion
- glossary and rule management
- action orchestrator and typed AI actions
