# Rust security audit with cargo-audit

## Objective
Provide the local and CI usage guide for Rust dependency auditing with `cargo-audit` in Translat.

## Current repository target
The Rust security audit workflow now audits the lockfile that belongs to the real desktop shell: `src-tauri/Cargo.lock`.

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
- CI installs `cargo-audit` and runs it against `src-tauri/Cargo.lock`.
- The workflow no longer contains scaffold-era detection logic for hypothetical lockfiles elsewhere in the repository.

## Scheduled auditing
Security advisories can appear after dependencies were last changed. For that reason, the workflow also includes a scheduled audit on the default branch.

## B3.1 note
Security alert settings, Dependabot security update preferences, and any owner-level notification tuning remain outside this workflow and belong to GitHub UI follow-up in B3.1.
