# src-tauri/core/src/domain

## Purpose

Portable BentoLife domain services and data contracts.

## Start Here

- `workspace_scanner.rs`, `workspace_metadata.rs`, `vault.rs`, `storage.rs` - workspace and vault foundations.
- `markdown_parser.rs`, `markdown_document.rs`, `markdown_assets.rs` - Markdown parsing, identity, and asset handling.
- `notes.rs`, `todo.rs`, `contacts.rs`, `habits.rs` - module domain services.
- `module_schema.rs`, `module_registry.rs` - schema and registry contracts.
- `dashboard_widgets.rs`, `theme.rs`, `import_export.rs`, `recovery.rs` - system domain services.

## Do Not Put Here

- Renderer-specific code or Tauri command wrappers.
