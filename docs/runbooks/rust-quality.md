# Rust quality with Clippy

## Objective
Provide the local and CI usage guide for Rust linting with cargo clippy in Translat.

## Why this exists now
The full Rust application shell is still planned for B1, but the repository can already prepare the expected quality guardrail for Rust code.

## Recommended CI command
Use Clippy through cargo in CI.

Typical strict command:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Current repository behavior
At this stage, the GitHub workflow checks whether a Cargo manifest exists in one of the expected locations:
- `Cargo.toml`
- `src-tauri/Cargo.toml`
- `src/backend/Cargo.toml`

If a manifest exists, the workflow runs Clippy.
If no manifest exists yet, the workflow exits cleanly and reports that Rust bootstrap has not started.

## Expected next step
Once B1 starts the real desktop and Rust shell, this workflow should begin linting real Rust code without needing a redesign.
