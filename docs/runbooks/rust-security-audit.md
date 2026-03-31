# Rust security audit with cargo-audit

## Objective
Provide the local and CI usage guide for Rust dependency auditing with `cargo-audit` in Translat.

## Why this exists now
The full Rust application shell is still planned for B1, but the repository can already prepare the expected dependency security guardrail for Rust code.

## Recommended local installation
Install `cargo-audit` as a Cargo subcommand:

```bash
cargo install cargo-audit --locked
```

## Typical local usage
Run it at the top level of a Cargo project:

```bash
cargo audit
```

## Current repository behavior
At this stage, the GitHub workflow checks whether a Cargo lockfile exists in one of the expected locations:
- `Cargo.lock`
- `src-tauri/Cargo.lock`
- `src/backend/Cargo.lock`

If a lockfile exists, the workflow installs `cargo-audit` and runs the audit.
If no lockfile exists yet, the workflow exits cleanly and reports that Rust bootstrap has not started.

## Scheduled auditing
Security advisories can appear after dependencies were last changed. For that reason, the workflow also includes a scheduled audit on the default branch.

## Expected next step
Once B1 starts the real desktop and Rust shell, this workflow should begin auditing real Rust dependencies without needing a redesign.
