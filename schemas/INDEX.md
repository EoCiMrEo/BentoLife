# schemas

## Purpose

Committed product schema sources for modules, widgets, and metadata.

## Start Here

- `modules/` - module schemas for Notes, Todos, Contacts, Habits, and aggregate module schema.
- `widgets/` - Dashboard widget schema sources.
- `metadata/` - workspace, registry, layout, theme, and dashboard metadata schemas.

## Tests

Run `cmd /c corepack pnpm run schemas:check`.

## Do Not Put Here

- Runtime user data.
- Generated TypeScript bindings.
