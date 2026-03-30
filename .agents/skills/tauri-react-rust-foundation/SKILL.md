# tauri-react-rust-foundation

## Purpose
Use this skill when working on the foundational desktop stack of Translat based on Tauri, React, TypeScript, and Rust.

## When to use
- bootstrapping the desktop shell
- organizing frontend/backend boundaries
- wiring Tauri commands
- defining initial module layout
- setting up shared contracts between UI and Rust services

## Expected outcomes
- changes keep the desktop shell modular
- UI concerns stay in frontend code
- orchestration and native/system concerns stay in Rust
- contracts between layers remain explicit

## Working rules
- do not mix UI logic with persistence or LLM orchestration
- keep command interfaces typed and minimal
- prefer small vertical slices over broad rewrites
- document structural decisions when introducing new modules

## Recommended checklist
1. Identify whether the change belongs to frontend, Rust backend, or shared contracts.
2. Keep the public interface between layers explicit.
3. Add or update repository structure docs if the module map changes.
4. Verify the change does not bypass the intended action-oriented architecture.
