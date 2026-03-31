# AGENTS.md

## Project context
- Project name: Translat
- Slug: translat
- Description: Desktop application for orchestrating English-to-Spanish translation workflows with AI.
- Owner: Ernesto
- Official repository: https://github.com/Ernest-NA/Translat

## Canonical sources of truth
- GitHub is the source of truth for code, branches, pull requests, and release history.
- Notion is the source of truth for backlog, PRD, architecture, decisions, and roadmap.

## Product focus
Translat is not a generic translator. It is a desktop translation workstation focused on:
- project-based translation workflows,
- glossary and terminology control,
- style profiles and editorial rules,
- segment-level translation and retraduction,
- historical comparison and supervised refinement,
- parallel corpora EN/ES,
- cost traceability,
- and typed AI actions orchestrated through a controlled backend architecture.

## Required reading before changing code
1. `docs/architecture/system-overview.md`
2. `docs/product/git-workflow-and-releases.md`
3. The relevant documentation in Notion for the feature or task being changed
4. The matching skill in `.agents/skills/`

## Branching and pull request workflow
### Task work
- Create a task branch from `develop`.
- Expected naming: `task/{NOTION_TASK_ID}-{slug}`.
- Open the pull request against `develop`.
- Human review is required before merge.

### Release work
- Create a release branch from `develop` when the intended release scope is complete.
- Expected naming: `release/{RELEASE_ID}`.
- Open the pull request against `main`.
- Human review is required before merge.

### Traceability expectations
- Use the Notion task identifier in the branch name when possible.
- Use the Notion task identifier in the pull request title.
- Keep repository work traceable to backlog items and releases.

## Working rules
- Prefer small, reviewable diffs.
- Preserve existing conventions unless the task requires a justified change.
- Update documentation when behavior, structure, workflows, or public interfaces change.
- Do not introduce new dependencies unless clearly justified.
- Do not rewrite large areas of code when a local change is enough.
- Do not bypass the typed action architecture with ad hoc LLM integration.
- Keep persistence, orchestration, UI, and prompting concerns separated.

## Expected execution model for Codex
1. Read the related docs in `docs/` and the corresponding Notion material.
2. Select the relevant skill from `.agents/skills/`.
3. Implement the minimum safe change for the requested task.
4. Add or update tests.
5. Summarize files changed, risks, and follow-ups.

## Repository map
- `src/`: application source code
- `tests/`: automated tests
- `docs/architecture/`: technical architecture guidance
- `docs/product/`: workflow, releases, and product-facing repo guidance
- `docs/runbooks/`: operational procedures
- `.agents/skills/`: reusable Codex workflows
- `.github/`: GitHub workflows and templates
- `notion/`: Notion setup guidance and exported references

## Validation commands for scaffold stage
```bash
npm install
npm run check:scaffold
npm run test
npm run build
```

## Done means
- The requested scope is implemented.
- Tests were added or updated where applicable.
- Relevant docs were updated.
- Risks and follow-up items are explicitly called out.
- The change remains traceable to its Notion task and pull request.
