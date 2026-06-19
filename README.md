<h1 align="center">BentoLife</h1>

<p align="center">
  <strong>A calm, local-first desktop life dashboard powered by Markdown.</strong>
</p>

<p align="center">
  Notes · Todos · Contacts · Habits · Dashboard Widgets
</p>

<!--
README preview assets to add after capturing real app screenshots:

assets/readme/bentolife-icon.png
assets/readme/bentolife-dashboard.png
assets/readme/dashboard.png
assets/readme/modules.png
assets/readme/architect.png

Suggested hero placement after assets exist:

<p align="center">
  <img src="assets/readme/bentolife-icon.png" width="96" alt="BentoLife icon" />
</p>

![BentoLife Dashboard](assets/readme/bentolife-dashboard.png)
-->

## ✨ What is BentoLife?

BentoLife is a desktop-first personal life dashboard built around a local Markdown vault. It helps you organize notes, todos, contacts, habits, and dashboard widgets while keeping your content readable, portable, and owned by you.

It is designed to feel calm by default, powerful when needed, and Markdown-native underneath.

## 🧩 What BentoLife helps you do

- Organize notes, todos, contacts, and habits in one local workspace.
- Build a calm personal dashboard with useful widgets.
- Keep your content readable as Markdown instead of locking it inside a proprietary database.
- Switch between English and Vietnamese app UI without rewriting your Markdown.
- Use app-owned metadata for layouts, themes, search, import review, Trash, and Archive while keeping user content separate.

## 🔒 Local-first by design

Your content lives in a BentoLife vault as Markdown. BentoLife stores app state inside the vault metadata folder, so your personal content stays portable and readable.

```text
.bentolifevault/          Your local BentoLife workspace
  modules/               Notes, todos, contacts, habits, and other module content
  .bentolifelayout/      App-owned metadata such as layout, widgets, themes, search, Trash, and Archive
```

## 🌏 Language support

BentoLife supports English and Vietnamese app UI. Switching the interface language does not rewrite user-authored Markdown.

## 🚀 Download

Download the latest Windows installer from the **Releases** page.

> BentoLife is currently in Alpha. It is usable, but you should keep backups of important vaults and personal data.

## 📚 Documentation

After each release publish, this repository is updated from `bentolife-dev` with public-safe app source and user-facing documentation.

Start with:

- `docs/instructions/README.md`
- `docs/diagrams/README.md`
- `docs/i18n/vi-glossary.md`

<!--
## 🖼 Preview images

Real app screenshots should be added under `assets/readme/` after they are captured from the released build:

```text
assets/readme/bentolife-icon.png
assets/readme/bentolife-dashboard.png
assets/readme/dashboard.png
assets/readme/modules.png
assets/readme/architect.png
```
Once those files exist, this README can show the product visually without changing the core copy.
-->
