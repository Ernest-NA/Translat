# CodeQL and code scanning in Translat

## Objective
Provide the repository-level guidance for GitHub CodeQL and code scanning in Translat.

## Why this exists now
The repository already has code scanning / CodeQL enabled in GitHub settings. This change adds the repository workflow so analysis runs automatically in CI.

## Current scope
At this stage, the workflow analyzes:
- JavaScript / TypeScript
- Rust, when a Rust manifest exists in the repository

## Rust-specific behavior
- Rust analysis is enabled from the start, but only runs when the repository contains `Cargo.toml` in one of the expected locations.
- This avoids failing CodeQL runs before B1 introduces the real Rust shell.
- When Rust is present, the workflow uses `build-mode: none`.
- GitHub documents that Rust analysis requires `rustup` and `cargo` on the runner and that `Cargo.toml` or `rust-project.json` must be present.

## Trigger strategy
The workflow runs on:
- pushes to `develop`
- pushes to `main`
- pull requests targeting `develop`
- pull requests targeting `main`
- weekly schedule

## Important setup note
If the repository still has CodeQL default setup enabled in GitHub settings, advanced workflow uploads will fail. In that case, switch the repository from default setup to advanced setup so the workflow file becomes the active CodeQL configuration.

## Expected review flow
1. CodeQL runs automatically.
2. Results are published in GitHub code scanning.
3. Findings are reviewed like any other repository quality signal.
4. Fixes follow the normal task branch and PR workflow.
