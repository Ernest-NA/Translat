# CodeQL and code scanning in Translat

## Objective
Provide the repository-level guidance for GitHub CodeQL and code scanning in Translat.

## Current scope
The repository workflow analyzes:
- JavaScript / TypeScript from the current frontend workspace
- Rust from the desktop shell that lives in `src-tauri/`
- The workflow uses `github/codeql-action@v4`

## Rust-specific behavior
- Rust analysis now runs unconditionally because the repository already contains the real desktop shell in `src-tauri/`.
- The workflow uses `build-mode: none` for Rust.
- This is an intentional stability choice: the current repository does not need generated Rust code or a manual CodeQL build step to analyze the checked-in shell source.
- GitHub documentation states that Rust supports `build-mode: none` and still requires `rustup` and `cargo` on the runner.

## JavaScript / TypeScript behavior
- The workflow analyzes JavaScript / TypeScript without a CodeQL autobuild step.
- This is intentional because the current frontend does not need a build step to make the source analyzable by CodeQL, and removing `autobuild` reduces one moving part in CI.

## Trigger strategy
The workflow runs on:
- pushes to `develop`
- pushes to `main`
- pull requests targeting `develop`
- pull requests targeting `main`
- weekly schedule

## Intentional limits
- CodeQL only analyzes the languages that are present and stable in the repository today: JavaScript / TypeScript and Rust.
- The workflow does not try to detect future Cargo manifests or future application layouts.
- The workflow does not use manual build steps for Rust at this stage.
- If the repository later introduces generated Rust code, extra Cargo workspaces, or build-time code that materially affects analysis accuracy, revisit the Rust job and consider a manual build mode.

## B3.1 manual GitHub UI follow-up
The following items are intentionally outside B3 because they require repository-owner actions in GitHub UI:
1. Ensure CodeQL default setup is disabled so this advanced workflow remains the active configuration.
2. Configure repository rulesets or branch protection with the exact status checks that should become required after this workflow set is considered stable.
3. Review code scanning alert notification preferences and owner-facing security settings.

## Known conflict signal
- If a CodeQL run finishes analysis locally but GitHub rejects the SARIF upload with `CodeQL analyses from advanced configurations cannot be processed when the default setup is enabled`, the remaining issue is in GitHub UI, not in this workflow file.
- The required fix is to disable CodeQL default setup in GitHub so the advanced workflow in `.github/workflows/codeql.yml` is the only active CodeQL configuration.

## Expected review flow
1. CodeQL runs automatically.
2. Results are published in GitHub code scanning.
3. Findings are reviewed like any other repository quality signal.
4. Fixes follow the normal task branch and PR workflow.
