# src-tauri

## Purpose

Tauri desktop shell, Rust backend command boundary, and reusable BentoLife core crate.

## Start Here

- `tauri.conf.json` - generated/synced Tauri app config.
- `src/backend/` - thin Tauri command handlers and desktop adapters.
- `core/` - portable Rust domain crate.
- `capabilities/` - Tauri permission declarations.
- `gen/` - generated Tauri schema files.

## Do Not Put Here

- React renderer code. Put that in `src/`.
- User vault content.
