# Frontend quality with Biome

## Objective
Provide the local usage guide for frontend linting and formatting checks with Biome during the scaffold stage.

## Why this exists now
Translat does not yet have the full React application initialized, but the repository can already enforce frontend quality conventions before B1 starts.

## Recommended local install
Biome documentation recommends installing Biome as a development dependency and pinning the version.

Example:
```bash
npm i -D -E @biomejs/biome
```

## Local commands
### Check frontend files
```bash
npx @biomejs/biome check src/frontend
```

### Lint frontend files
```bash
npx @biomejs/biome lint src/frontend
```

### Format frontend files
```bash
npx @biomejs/biome format --write src/frontend
```

## Current scope
At this stage, Biome is configured to target JavaScript and TypeScript files under `src/frontend/`.

## Next expected step
When B1 initializes the real frontend shell, Biome can be extended with repository scripts and broader coverage if needed.
