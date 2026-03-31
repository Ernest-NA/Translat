# CodeQL and code scanning in Translat

## Objective
Provide the repository-level guidance for GitHub CodeQL and code scanning in Translat.

## Why this exists now
The repository already has code scanning / CodeQL enabled in GitHub settings. This change adds the repository workflow so analysis runs automatically in CI.

## Current scope
At this stage, the workflow analyzes:
- JavaScript / TypeScript

This is the safest initial scope because the frontend and repository automation already include JavaScript/TypeScript files, while the full Rust application shell is still planned for B1.

## Planned extension
Once B1 introduces the real Rust project structure, the CodeQL workflow can be expanded to include Rust analysis as well.

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
