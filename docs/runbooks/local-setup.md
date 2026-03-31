# Local setup

## Current stage

This repository already includes the minimum Translat desktop shell with Tauri, React, TypeScript, Rust, and the encrypted SQLite bootstrap used by the backend foundation.

## Minimum prerequisites

- Git
- Node.js 20+
- npm 10+
- Rust toolchain via `rustup`
- Windows 11 development environment
- Microsoft C++ Build Tools / Visual Studio Build Tools 2022
- Microsoft Edge WebView2 Runtime
- Strawberry Perl available in `PATH` so vendored SQLCipher can build locally

## Install dependencies

```bash
npm install
```

The Tauri CLI is installed as a local npm dependency, so no global Tauri installation is required.

If `cargo` or `rustc` are not available in the current PowerShell session after installing Rust, prepend the rustup bin directory before running the Translat scripts:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

## Development commands

Run the desktop shell in development mode:

```bash
npm run dev
```

Validate the frontend and backend foundations:

```bash
npm run check
npm run test
```

Create a desktop production build:

```bash
npm run build
```

## What to expect

- `npm run dev` opens the main Translat desktop window.
- The base app shell renders inside the Tauri window instead of the B1 placeholder view.
- The frontend invokes the Rust `healthcheck` command through a shared desktop wrapper.
- The backend bootstraps an encrypted SQLite database on startup and reports the resulting state in the healthcheck panel.
- Command failures surface a normalized frontend error with code and message.

## Database bootstrap

- On the first desktop run, the backend creates the encrypted SQLite database and applies `0001_initial_schema`.
- On later runs, the same database is reopened and `schema_migrations` prevents reapplying `0001`.
- The database files are created under the Tauri `app_data_dir` for the Translat bundle identifier.
- The encryption key is stored separately and protected with Windows DPAPI.

See `docs/runbooks/database-bootstrap.md` for the current persistence strategy and limits.
