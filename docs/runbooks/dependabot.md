# Dependabot in Translat

## Objective
Define how Dependabot is used in Translat to keep repository dependencies reviewed and updated with low operational noise.

## Scope
Dependabot is configured for:
- GitHub Actions workflows
- npm dependencies at the repository root
- Cargo dependencies in `src-tauri/`

## Update strategy
- Weekly cadence
- Conservative limit on open pull requests
- Grouped updates where that helps reduce review noise

## Current repository alignment
- npm updates target the root `package.json`
- Cargo updates target `src-tauri/Cargo.toml`
- GitHub Actions updates target workflow actions declared in `.github/workflows/`

## Expected review flow
1. Dependabot opens a PR.
2. The PR is reviewed like any other repository change.
3. The change is merged into `develop` if valid.
4. Release consolidation still happens through the normal release flow.

## Notes
- Dependabot alerts and security updates are already enabled in repository settings.
- This file only documents the repository-level behavior and PR generation policy.
- Repository-owner alert preferences and security update settings remain part of B3.1 because they are configured in GitHub UI.
