# App Behavior Diagram

```mermaid
flowchart TD
  Start["Launch app"] --> VaultState{"Selected vault?"}
  VaultState -->|No| Onboarding["Create or choose local vault"]
  VaultState -->|Yes| Scan["Load vault projections"]
  Onboarding --> Scan
  Scan --> Ready["Ready shell"]
  Ready --> Dashboard["Dashboard widgets"]
  Ready --> Modules["Daily modules"]
  Ready --> Settings["Settings: Vault and Data"]
  Ready --> Architect["Architect: advanced diagnostics"]
  Ready --> System["Trash and Archive"]
  Modules --> Notes["Notes"]
  Modules --> Todos["Todos"]
  Modules --> Contacts["Contacts"]
  Modules --> Habits["Habits"]
  Scan --> Issues{"Recovery issue?"}
  Issues -->|Yes| Recovery["Architect Recovery"]
  Issues -->|No| Ready
```

## Ownership Rules

- Daily work belongs in Dashboard and module surfaces.
- Settings stays calm and configuration-focused.
- Architect owns advanced diagnostics, schema/data graph, recovery, widgets, and appearance.
- Recovery is for app metadata problems, not normal Markdown syntax.
