# src

## Purpose

React/Vite renderer source for BentoLife's desktop shell and browser fallback tests.

## Start Here

- `App.tsx` - application orchestration, shell routing, and top-level view composition.
- `main.tsx` - renderer entrypoint.
- `components/` - UI surfaces and reusable primitives.
- `i18n/` - lightweight English/Vietnamese UI translation provider, dictionaries, and fallback behavior.
- `services/` - typed frontend adapters for Tauri commands and browser fallbacks.
- `state/` - local state and parsing helpers.
- `test/` - Vitest setup plus frontend tests grouped by layer.

## Do Not Put Here

- Tauri command handlers or filesystem/domain logic. Put those under `src-tauri/`.
- New colocated Vitest tests. V5.4 centralizes frontend tests under `src/test/`.
