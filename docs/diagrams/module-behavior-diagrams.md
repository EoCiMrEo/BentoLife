# Module Behavior Diagrams

## Notes

```mermaid
flowchart TD
  Browse["Browse notes"] --> Select["Select note"]
  Select --> View["Rendered view"]
  View --> Edit["Edit Markdown"]
  View --> Rename["Inline rename"]
  Edit --> Preview["Safe Markdown preview"]
  Edit --> Save["Save content"]
  Rename --> SaveTitle["Save title or cancel"]
  Save --> View
  SaveTitle --> View
```

## Todos

```mermaid
flowchart TD
  Browse["Browse tasks"] --> Add["Add Task drawer"]
  Add --> Create["Create managed task Markdown"]
  Browse --> Select["Select task"]
  Select --> Toggle["Toggle checklist item"]
  Select --> Edit["Edit task drawer"]
  Edit --> Conflict{"Raw Markdown conflict?"}
  Conflict -->|No| Save["Save structured fields"]
  Conflict -->|Yes| Choice["Choose preserve, raw, note copy, or cancel"]
  Save --> Select
  Choice --> Select
```

## Contacts And Habits

```mermaid
flowchart TD
  Browse["Browse records"] --> New["New drawer"]
  New --> Create["Create managed Markdown record"]
  Browse --> Select["Select record"]
  Select --> Detail["Detail view"]
  Detail --> Edit["Edit drawer"]
  Edit --> Save["Save structured fields and preserved raw content"]
  Detail --> Inspector["Inspector shows warnings and preserved unknowns"]
  Save --> Detail
```

## Shared Module Contract

- Markdown remains the durable user-content source of truth.
- Structured fields are conveniences over managed Markdown.
- Unknown safe content is preserved and surfaced through Inspector/fallback UI.
