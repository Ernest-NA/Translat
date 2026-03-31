# Local setup

## Current stage

This repository already includes the minimum Translat desktop shell with Tauri, React, TypeScript, and Rust.

## Minimum prerequisites

- Git
- Node.js 20+
- npm 10+
- Rust toolchain via `rustup`
- Windows 11 development environment
- Microsoft C++ Build Tools / Visual Studio Build Tools 2022
- Microsoft Edge WebView2 Runtime

## Install dependencies

```bash
npm install
```

The Tauri CLI is installed as a local npm dependency, so no global Tauri installation is required.

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
- The placeholder screen renders inside the Tauri shell.
- The frontend invokes the Rust `healthcheck` command and shows the result on screen.
