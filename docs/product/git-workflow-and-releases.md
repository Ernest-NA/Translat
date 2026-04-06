# Git workflow and release strategy

## Objective
Define the official branching, pull request, release, and naming strategy for Translat.

## Main branches
- `develop`: integration branch for approved task work
- `main`: stable release branch

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
- `task/TR-11-document-structure-base`
- `task/TR-12-chunk-builder-and-basic-chunking`
- `task/TR-155-translate-document-job`

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
- `[TR-11] Final validation for document structure base`
- `[TR-12] Add translation chunk builder`

### Release PRs
Format:
- `[R0.1] Foundation release`
- `[R0.2] Core translation release`

## Commit naming
Recommended prefix:
- `TR-11: validate document structure base`
- `TR-12: add translation chunk builder`

## Planned releases
### R0.1 - Foundation
Legacy refs:
- A1
- A2
- A3
- A4
- B1
- B4

### R0.2 - Project and document foundation
Legacy refs:
- C1
- C2
- C3
- C4
- C5
- D1
- D2
- D3
- D4
- D5

### R0.3 - Core AI workflow
Legacy refs:
- E1
- E2
- E3
- E4
- E5
- E6
- E7
- E8
- F1
- F2
- F3
- F4

### R0.4 - Parallel corpus and document scale
Legacy refs:
- G1
- G2
- G3
- G4
- G5
- H1
- H2
- H3

### R0.5 - QA and hardening
Legacy refs:
- I1
- I2
- I3
- I4

## Notes
- Notion remains the canonical source for backlog items.
- Git branch, PR, and commit naming should reference canonical `TR-*` Notion task identifiers.
- Legacy IDs remain release-planning references only and should be preserved as historical mappings, not reused for new task branches or PRs.
- The repository official URL is `https://github.com/Ernest-NA/Translat`.
- The `docs-governance` branch is an explicit bootstrap exception used to establish repository governance before regular `task/*` branches are enforced.
