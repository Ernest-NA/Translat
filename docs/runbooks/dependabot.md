# Dependabot in Translat

## Objective
Define how Dependabot is used in Translat to keep repository dependencies reviewed and updated with low operational noise.

## Scope
At this stage, Dependabot is configured for:
- GitHub Actions workflows
- npm dependencies at the repository root
- Cargo dependencies when Rust manifests are present

## Update strategy
- Weekly cadence
- Conservative limit on open pull requests
- Grouped updates where that helps reduce review noise

## Why this exists now
The repository is still in scaffold stage, but enabling Dependabot early helps establish the maintenance pattern before the stack grows.

## Expected review flow
1. Dependabot opens a PR.
2. The PR is reviewed like any other repository change.
3. The change is merged into `develop` if valid.
4. Release consolidation still happens through the normal release flow.

## Notes
- Dependabot alerts and security updates are already enabled in repository settings.
- This file only documents the repository-level behavior and PR generation policy.
