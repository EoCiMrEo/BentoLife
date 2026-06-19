# src-tauri/src/backend

## Purpose

Desktop backend boundary for Tauri command registration and OS-specific adapters.

## Start Here

- `commands/` - command handlers exposed to the renderer.
- `adapters/` - desktop-only adapters such as platform paths.
- `mod.rs` - backend module wiring.

## Do Not Put Here

- Portable domain logic. Put that in `src-tauri/core/src/domain/`.
