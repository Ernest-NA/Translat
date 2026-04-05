# Git workflow and release strategy

## Objective
Define the official branching, pull request, release, and naming strategy for Translat.

## Main branches
- `develop`: integration branch for approved task work
- `main`: stable release branch

## Task identifier policy
- `TR-*` is the canonical active task identifier system for repository workflow.
- Use the canonical `TR-*` identifier in branch names, pull request titles, commit prefixes, release planning, and traceability notes.
- Older identifiers such as `A1`, `B4`, or `E6` are legacy references only.
- Preserve those legacy references explicitly under labels such as `Legacy refs`, `Adjusts`, `Extends later`, or `Keeps`, but do not use them as the primary workflow identifier.

## Task workflow
1. Create a task branch from `develop`.
2. Commit only to the task branch.
3. Open a pull request into `develop`.
4. Human review.
5. Merge into `develop` when approved.

## Release workflow
1. When the planned set of tasks for a release is complete in `develop`, create a release branch.
2. Open a pull request from the release branch into `main`.
3. Human review.
4. Merge into `main` when approved.

## Branch naming
### Task branches
Format:
- `task/{NOTION_TASK_ID}-{slug}`

Examples:
- `task/TR-2-adapt-agents-md`
- `task/TR-155-sqlite-encrypted-migrations`
- `task/TR-240-translate-chunk-baseline`

### Release branches
Format:
- `release/{RELEASE_ID}`

Examples:
- `release/R0.1`
- `release/R0.2`
- `release/R1.0`

## Pull request naming
### Task PRs
Format:
- `[TR-2] Adapt AGENTS.md to Translat domain`
- `[TR-240] Implement translate_chunk baseline`

### Release PRs
Format:
- `[R0.1] Foundation release`
- `[R0.2] Core translation release`

## Commit naming
Recommended prefix:
- `TR-2: adapt AGENTS.md`
- `TR-240: implement translate_chunk baseline`

## Planned releases
Release contents must be tracked with canonical `TR-*` task identifiers in Notion and in repository workflow artifacts.

The legacy buckets below are preserved only as historical release mappings to older backlog references.

### R0.1 - Foundation
- Legacy refs: `A1`, `A2`, `A3`, `A4`, `B1`, `B4`

### R0.2 - Project and document foundation
- Legacy refs: `C1`, `C2`, `C3`, `C4`, `C5`, `D1`, `D2`, `D3`, `D4`, `D5`

### R0.3 - Core AI workflow
- Legacy refs: `E1`, `E2`, `E3`, `E4`, `E5`, `E6`, `E7`, `E8`, `F1`, `F2`, `F3`, `F4`

### R0.4 - Parallel corpus and document scale
- Legacy refs: `G1`, `G2`, `G3`, `G4`, `G5`, `H1`, `H2`, `H3`

### R0.5 - QA and hardening
- Legacy refs: `I1`, `I2`, `I3`, `I4`

## Notes
- Notion remains the canonical source for backlog items.
- Git branch, PR, and commit naming should reference canonical Notion `TR-*` task identifiers.
- If a task originates from an older backlog item, record the older code explicitly as a legacy reference in the PR body, release notes, or task metadata.
- The repository official URL is `https://github.com/Ernest-NA/Translat`.
- The `docs-governance` branch is an explicit bootstrap exception used to establish repository governance before regular `task/*` branches are enforced.
