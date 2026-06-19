# src/services

## Purpose

Typed frontend service adapters for Tauri commands with deterministic browser fallbacks for tests and preview.

## Start Here

- `backendCore.ts` - backend invocation helper boundary.
- `notes.ts`, `todo.ts`, `contacts.ts`, `habits.ts` - module data adapters.
- `imports.ts`, `vault.ts`, `dashboard.ts`, `widgets.ts`, `theme.ts` - system and workspace adapters.
- `widgetRendererRegistry.tsx` - Dashboard widget renderer contracts and quick-action handlers.
- `markdownPreview.ts` - safe Markdown preview mapping used by editor surfaces.
- `../scripts/check-dead-exports.mjs` scans runtime exports here as part of the V5.7 cleaner pass.

## Tests

Service tests live in `src/test/services/`.

## Do Not Put Here

- React components.
- App-level routing.
- Rust/Tauri command implementations.
