# Frontend quality with Biome

## Objective
Provide the local and CI usage guide for frontend linting and formatting checks with Biome.

## Recommended local install
Biome documentation recommends installing Biome as a development dependency and pinning the version.

Example:
```bash
npm i -D -E @biomejs/biome
```

## Local commands
### Run the repository script
```bash
npm run lint
```

### Check frontend files directly
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
- Biome targets JavaScript and TypeScript files under `src/frontend/`.
- CI uses the repository-pinned Biome version through `npm ci` and `npm run lint` instead of a separate action-managed version.
