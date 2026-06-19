//! V3 recovery and entity upgrade migrations.

use std::{collections::BTreeSet, fs, path::Path};

use crate::{
    markdown::{
        ensure_identity_comment_at_end, find_identity_comment, parse_frontmatter,
        remove_identity_comments, with_frontmatter,
    },
    rebuild_cache_from_vault,
    utils::{
        copy_file_verified, current_timestamp_label, generate_core_document_id, normalize_relative,
        resolve_vault_relative_path, slugify, unique_relative_path, vault_relative_path,
        write_json_atomic, write_text_atomic,
    },
    EntityUpgradeChange, EntityUpgradePreview, EntityUpgradeReport, ENTITY_UPGRADE_MANIFEST_FOLDER,
    TRASH_FOLDER,
};

pub fn preview_entity_upgrade(
    vault_path: &Path,
    collect_vault_markdown: impl Fn(&Path) -> Result<Vec<std::path::PathBuf>, String>,
    existing_vault_paths: impl Fn(&Path) -> Result<BTreeSet<String>, String>,
    ensure_vault_folder: impl Fn(&Path) -> Result<std::path::PathBuf, String>,
    markdown_title: impl Fn(&str, &str) -> String,
) -> Result<EntityUpgradePreview, String> {
    let vault_path = ensure_vault_folder(vault_path)?;
    let mut changes = Vec::new();
    let mut legacy_paths = Vec::new();
    let mut occupied = existing_vault_paths(&vault_path)?;

    for markdown_path in collect_vault_markdown(&vault_path)? {
        let relative_path = vault_relative_path(&vault_path, &markdown_path)?;
        if !is_legacy_entity_path(&relative_path) {
            continue;
        }
        legacy_paths.push(relative_path.clone());
        let markdown = fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let parsed = parse_frontmatter(&markdown);
        let body = remove_identity_comments(&parsed.body);
        let source_identity = find_identity_comment(&markdown).map(|identity| identity.document_id);
        changes.extend(plan_upgrade_changes_for_legacy_path(
            &relative_path,
            &body,
            source_identity.as_deref(),
            &mut occupied,
            &markdown_title,
        ));
    }

    Ok(EntityUpgradePreview {
        schema_version: 1,
        vault_path: vault_path.to_string_lossy().to_string(),
        changes,
        legacy_paths,
        warnings: vec![
            "Upgrade is explicit and creates a backup manifest before moving legacy files."
                .to_string(),
            "Normal reads keep MVP aggregate files compatible until this upgrade is applied."
                .to_string(),
        ],
    })
}

pub fn apply_entity_upgrade(
    vault_path: &Path,
    collect_vault_markdown: impl Fn(&Path) -> Result<Vec<std::path::PathBuf>, String>,
    existing_vault_paths: impl Fn(&Path) -> Result<BTreeSet<String>, String>,
    ensure_vault_folder: impl Fn(&Path) -> Result<std::path::PathBuf, String>,
    markdown_title: impl Fn(&str, &str) -> String,
    write_minimal_document_metadata: impl Fn(&Path, &str, &str, &str, &str) -> Result<(), String>,
) -> Result<EntityUpgradeReport, String> {
    let vault_path = ensure_vault_folder(vault_path)?;
    let preview = preview_entity_upgrade(
        &vault_path,
        collect_vault_markdown,
        existing_vault_paths,
        &ensure_vault_folder,
        markdown_title,
    )?;
    let timestamp = current_timestamp_label();
    let backup_root = format!(
        "{TRASH_FOLDER}/entity-upgrades/{}",
        timestamp.replace(':', "-")
    );
    let mut trashed_legacy_paths = Vec::new();

    for change in &preview.changes {
        let target_path = resolve_vault_relative_path(&vault_path, &change.target_path)?;
        if target_path.exists() {
            continue;
        }
        let body = if change.markdown_body.trim().is_empty() {
            format!("# {}\n", change.title)
        } else {
            change.markdown_body.clone()
        };
        let metadata_path = format!(".bentolifelayout/documents/{}.json", change.document_id);
        let managed_markdown = ensure_identity_comment_at_end(
            &with_frontmatter(&body, &metadata_path),
            &change.document_id,
        );
        write_text_atomic(&target_path, &managed_markdown)?;
        write_minimal_document_metadata(
            &vault_path,
            &change.document_id,
            &change.target_path,
            &change.entity_type,
            &managed_markdown,
        )?;
    }

    for legacy_path in &preview.legacy_paths {
        let source_path = resolve_vault_relative_path(&vault_path, legacy_path)?;
        if !source_path.is_file() {
            continue;
        }
        let backup_relative_path = format!("{backup_root}/{}", normalize_relative(legacy_path));
        let backup_path = resolve_vault_relative_path(&vault_path, &backup_relative_path)?;
        copy_file_verified(&source_path, &backup_path)?;
        fs::remove_file(&source_path)
            .map_err(|error| format!("Unable to move legacy file {legacy_path}: {error}"))?;
        trashed_legacy_paths.push(backup_relative_path);
    }

    let cache = rebuild_cache_from_vault(&vault_path)?;
    let manifest = EntityUpgradeReport {
        schema_version: 1,
        vault_path: vault_path.to_string_lossy().to_string(),
        upgraded_at: timestamp.clone(),
        manifest_path: format!(
            "{ENTITY_UPGRADE_MANIFEST_FOLDER}/entity-upgrade-{}.json",
            timestamp.replace(':', "-")
        ),
        backup_root,
        changes: preview.changes,
        trashed_legacy_paths,
        cache,
    };
    write_json_atomic(&vault_path.join(&manifest.manifest_path), &manifest)?;
    Ok(manifest)
}

pub(crate) fn reject_older_vault_target(vault_path: &Path) -> Result<(), String> {
    if vault_path.exists() && has_older_vault_shape(vault_path) {
        return Err(
            "Older BentoLife vault structure detected. Back up or snapshot this vault, then create a fresh V3 vault and use copy-only import/recovery instead of in-place migration."
                .to_string(),
        );
    }
    Ok(())
}

fn has_older_vault_shape(vault_path: &Path) -> bool {
    if !vault_path.is_dir() {
        return false;
    }
    if vault_path.join("notes").is_dir() {
        return true;
    }
    for legacy_file in [
        "modules/todos.md",
        "modules/contacts.md",
        "modules/habits.md",
    ] {
        if vault_path.join(legacy_file).is_file() {
            return true;
        }
    }
    for module in ["notes", "todos", "contacts", "habits"] {
        let module_path = vault_path.join("modules").join(module);
        if contains_legacy_module_markdown(&module_path) {
            return true;
        }
    }
    false
}

fn contains_legacy_module_markdown(module_path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(module_path) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("md")
            && path.file_name().and_then(|name| name.to_str()) != Some("INDEX.md")
            && path.file_name().and_then(|name| name.to_str()) != Some("MODULE.md")
        {
            return true;
        }
    }
    false
}

fn is_legacy_entity_path(path: &str) -> bool {
    let normalized = normalize_relative(path);
    (normalized.starts_with("notes/") && normalized.ends_with(".md"))
        || normalized == "modules/todos.md"
        || normalized == "modules/contacts.md"
        || normalized == "modules/habits.md"
}

fn plan_upgrade_changes_for_legacy_path(
    source_path: &str,
    markdown_body: &str,
    source_identity: Option<&str>,
    occupied: &mut BTreeSet<String>,
    markdown_title: &impl Fn(&str, &str) -> String,
) -> Vec<EntityUpgradeChange> {
    let normalized = normalize_relative(source_path);
    if normalized.starts_with("notes/") {
        let title = markdown_title(markdown_body, source_path);
        let target_path = unique_relative_path(
            &format!("modules/notes/data/{}.md", slugify(&title)),
            occupied,
        );
        return vec![EntityUpgradeChange {
            source_path: normalized,
            target_path,
            entity_type: "note".to_string(),
            title,
            document_id: source_identity
                .map(str::to_string)
                .unwrap_or_else(|| generate_core_document_id(source_path)),
            markdown_body: normalize_h1_body(markdown_body, "Note"),
            action: "move_note_to_v2_entity".to_string(),
        }];
    }
    if normalized == "modules/todos.md" {
        return markdown_body
            .lines()
            .filter_map(parse_todo_checkbox_for_upgrade)
            .map(|(title, completed)| {
                let target_path = unique_relative_path(
                    &format!("modules/todos/data/{}.md", slugify(&title)),
                    occupied,
                );
                let checkbox = if completed { "- [x]" } else { "- [ ]" };
                EntityUpgradeChange {
                    source_path: normalized.clone(),
                    target_path,
                    entity_type: "todos".to_string(),
                    title: title.clone(),
                    document_id: generate_core_document_id(&format!("todos:{title}")),
                    markdown_body: format!("# {title}\n\n{checkbox} {title}\n"),
                    action: "split_todo_task_to_v2_entity".to_string(),
                }
            })
            .collect();
    }
    if normalized == "modules/contacts.md" {
        return split_heading_records(markdown_body)
            .into_iter()
            .map(|record| {
                let title = markdown_title(&record, source_path);
                let target_path = unique_relative_path(
                    &format!("modules/contacts/data/{}.md", slugify(&title)),
                    occupied,
                );
                EntityUpgradeChange {
                    source_path: normalized.clone(),
                    target_path,
                    entity_type: "contact".to_string(),
                    title,
                    document_id: generate_core_document_id(&record),
                    markdown_body: normalize_h1_body(&record, "Contact"),
                    action: "split_contact_to_v2_entity".to_string(),
                }
            })
            .collect();
    }
    if normalized == "modules/habits.md" {
        return split_heading_records(markdown_body)
            .into_iter()
            .map(|record| {
                let title = markdown_title(&record, source_path);
                let target_path = unique_relative_path(
                    &format!("modules/habits/data/{}.md", slugify(&title)),
                    occupied,
                );
                EntityUpgradeChange {
                    source_path: normalized.clone(),
                    target_path,
                    entity_type: "habit".to_string(),
                    title,
                    document_id: generate_core_document_id(&record),
                    markdown_body: normalize_h1_body(&record, "Habit"),
                    action: "split_habit_to_v2_entity".to_string(),
                }
            })
            .collect();
    }
    Vec::new()
}

fn parse_todo_checkbox_for_upgrade(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim();
    for (prefix, completed) in [
        ("- [ ] ", false),
        ("- [x] ", true),
        ("- [X] ", true),
        ("* [ ] ", false),
        ("* [x] ", true),
        ("* [X] ", true),
    ] {
        if let Some(label) = trimmed.strip_prefix(prefix) {
            let label = label.trim();
            if !label.is_empty() {
                return Some((label.to_string(), completed));
            }
        }
    }
    None
}

fn split_heading_records(markdown_body: &str) -> Vec<String> {
    let lines = markdown_body.lines().collect::<Vec<_>>();
    let starts = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.trim().starts_with("## ").then_some(index))
        .collect::<Vec<_>>();
    let mut records = Vec::new();
    for (position, start) in starts.iter().enumerate() {
        let end = starts.get(position + 1).copied().unwrap_or(lines.len());
        let mut record = lines[*start..end].join("\n");
        if record.starts_with("## ") {
            record.replace_range(0..3, "# ");
        }
        records.push(format!("{}\n", record.trim_end()));
    }
    records
}

fn normalize_h1_body(markdown_body: &str, fallback: &str) -> String {
    let body = remove_identity_comments(markdown_body).trim().to_string();
    if body.lines().any(|line| line.trim_start().starts_with("# ")) {
        format!("{body}\n")
    } else {
        format!("# {fallback}\n\n{body}\n")
    }
}
