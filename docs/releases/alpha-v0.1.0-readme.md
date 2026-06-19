# BentoLife

BentoLife is a desktop-first, local-first life dashboard built around a readable Markdown vault. It organizes notes, todos, contacts, habits, and dashboard widgets while keeping user content portable.

`bentolife-dev` is the private development source of truth. `BentoLife` is the public release repository generated from a safe allowlist.

## Alpha Status

Alpha is feature-complete and documentation-locked for release preparation. Begin with [the documentation governance entry point](docs/00-START-HERE.md). Historical trackers and CPO conversations are evidence only; they do not define current behavior.

## Developer Navigation

- [Documentation start here](docs/00-START-HERE.md)
- [Repository map](INDEX.md)
- [Current documentation map](docs/INDEX.md)
- [Developer operational guide](docs/DEV-README.md)
- [Manual release flow](docs/release/ci-cd-release-flow.md) — private

## Core Model

- Desktop app: Tauri, React, Vite, TypeScript, and Rust.
- User content: local Markdown vault.
- App metadata: `.bentolifelayout/` for layouts, registry state, themes, import review, search, Trash, and Archive.
- Core surfaces: Dashboard, Notes, Todos, Contacts, Habits, Settings, Architect, Trash, and Archive.

## Development Commands

```bash
corepack pnpm test
corepack pnpm run schemas:check
corepack pnpm run build
corepack pnpm e2e --workers=1 --reporter=line
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --workspace --manifest-path src-tauri/Cargo.toml
cargo clippy --workspace --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Release Model

Development and internal evidence stay in `bentolife-dev`. The manual GitHub Actions release exports only public-safe source and user-facing documents, builds the Windows NSIS installer, and publishes it through `BentoLife`. The export excludes internal docs, task history, checklists, logs, CPO notes, and archives.
