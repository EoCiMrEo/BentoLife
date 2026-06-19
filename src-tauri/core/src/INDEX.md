# src-tauri/core/src

## Purpose

Portable Rust core crate for BentoLife domain behavior, independent of the Tauri shell.

## Start Here

- `domain/` - vault, module, parser, schema, widget, theme, import, graph, recovery, and storage domain logic.
- `lib.rs` - crate exports.
- `markdown.rs`, `graph.rs`, `recovery.rs`, `utils.rs` - shared core helpers.

## Do Not Put Here

- Tauri command macros or desktop window behavior.
