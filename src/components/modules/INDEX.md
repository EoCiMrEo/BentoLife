# src/components/modules

## Purpose

Daily module surfaces for Notes, Todos, Contacts, Habits, Markdown editing, and shared generated module rendering.

## Start Here

- `NotesPanel.tsx` - Notes browse/view/edit/rename flow.
- `MarkdownEditor.tsx` - source plus safe preview editor.
- `GeneratedModuleUI.tsx` - shared schema-driven module rendering.
- `EntityEditDrawer.tsx` - drawer editing for structured module records.
- `TodosGeneratedUI.tsx`, `ContactsGeneratedUI.tsx`, `HabitsGeneratedUI.tsx` - module-specific generated surfaces.
- `shared/ModuleSurface.tsx` - shared module shell primitives.

## Tests

Module component tests live in `src/test/components/modules/`.

## Do Not Put Here

- Architect diagnostics or Settings import/recovery controls.
