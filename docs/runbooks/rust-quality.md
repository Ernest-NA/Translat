# Rust quality with Clippy

## Objective
Provide the local and CI usage guide for Rust linting with cargo clippy in Translat.

## Current repository target
The Rust quality workflow now targets the real desktop shell manifest at `src-tauri/Cargo.toml`.

## Recommended CI command
Use Clippy through cargo in CI.

Typical strict command:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Current repository behavior
- CI runs Clippy directly against `src-tauri/Cargo.toml`.
- The workflow no longer includes scaffold-era detection logic for hypothetical Cargo manifests elsewhere in the repository.
