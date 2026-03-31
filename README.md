# Translat

Translat es una aplicación de escritorio para Windows 11 que orquesta traducciones del inglés al castellano con IA. Gestiona proyectos, glosarios, perfiles de estilo, reglas editoriales, corpus paralelos EN/ES, búsquedas de traducciones específicas, control de costes, trazabilidad y aprendizaje supervisado a partir de la edición humana.

## Estado actual

La base desktop ya está inicializada con:
- shell nativo en Tauri + Rust,
- frontend React + TypeScript cargado dentro del contenedor desktop,
- un patrón reutilizable para comandos frontend-backend,
- y manejo básico de errores del shell para seguir creciendo sin rehacer la base.

## Arranque rápido

```bash
npm install
npm run dev
```

La guía breve de setup y validación local está en `docs/runbooks/local-setup.md`.
