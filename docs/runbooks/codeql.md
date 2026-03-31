# CodeQL and code scanning in Translat

## Objective
Provide the repository-level guidance for GitHub CodeQL and code scanning in Translat.

## Why this exists now
The repository already has code scanning / CodeQL enabled in GitHub settings. This change adds the repository workflow so analysis runs automatically in CI.

## Current scope
At this stage, the workflow analyzes:
- JavaScript / TypeScript
- Rust

This broader scope is viable now because GitHub CodeQL supports Rust and documents `build-mode: none` for Rust analysis. In that mode, CodeQL does not invoke a full build, but it does require a `Cargo.toml` or `rust-project.json` to be present when Rust code exists.

## Rust-specific notes
- Rust is included from the start so the repository does not need a second CodeQL redesign once B1 begins.
- The workflow uses a matrix and explicitly sets `build-mode: none` for Rust.
- GitHub documents that Rust analysis requires `rustup` and `cargo` to be installed on the runner.

## Trigger strategy
The workflow runs on:
- pushes to `develop`
- pushes to `main`
- pull requests targeting `develop`
- pull requests targeting `main`
- weekly schedule

## Expected review flow
1. CodeQL runs automatically.
2. Results are published in GitHub code scanning.
3. Findings are reviewed like any other repository quality signal.
4. Fixes follow the normal task branch and PR workflow.
