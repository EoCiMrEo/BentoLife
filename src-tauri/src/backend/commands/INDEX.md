# src-tauri/src/backend/commands

## Purpose

Thin Tauri command handlers that call into `bentolife_core` or desktop adapters.

## Start Here

- `notes.rs`, `todos.rs`, `contacts.rs`, `habits.rs` - module command handlers.
- `imports.rs`, `vault.rs`, `recovery.rs`, `widgets.rs`, `themes.rs` - system command handlers.
- `shared.rs` - shared command helpers.

## Do Not Put Here

- Business rules that should be reusable outside Tauri.
- UI-specific behavior.
