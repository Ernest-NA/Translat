# AGENTS.md

## Project context
- Project name: Translat
- Slug: translat
- Description: Desktop application for orchestrating English-to-Spanish translation workflows with AI.
- Owner: Ernesto

## Operating model
This repository is designed for:
- Codex as the execution layer for coding tasks
- GitHub as the source of truth for code and pull requests
- Notion as the source of truth for roadmap, specs, and decisions

## Working rules
- Prefer small, reviewable diffs.
- Preserve existing conventions unless the task requires a change.
- Update documentation when behavior, structure, or public APIs change.
- Do not introduce new dependencies unless clearly justified.
- Do not rewrite large areas of code when a local change is enough.

## Expected workflow
1. Read the related docs in `docs/`.
2. Select the relevant skill from `.agents/skills/`.
3. Implement the minimum safe change.
4. Add or update tests.
5. Summarize files changed, risks, and follow-ups.

## Repository map
- `src/`: code
- `tests/`: tests
- `docs/architecture/`: technical guidance
- `docs/product/`: requirements and roadmap excerpts
- `docs/runbooks/`: operational procedures
- `.agents/skills/`: reusable workflows
- `.github/`: GitHub workflows and templates
- `notion/`: Notion setup guidance and exported references

## Done means
- The requested scope is implemented.
- Tests were added or updated where applicable.
- Relevant docs were updated.
- Risks and follow-up items are explicitly called out.
