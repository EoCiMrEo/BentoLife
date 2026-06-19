const KNOWN_CONTENT_MODULES: [&str; 4] = ["notes", "todos", "contacts", "habits"];

pub fn is_runtime_path(relative_path: &str) -> bool {
    let path = normalize_import_relative_path(relative_path);
    let parts = path.split('/').collect::<Vec<_>>();
    if path.is_empty() {
        return true;
    }
    if matches!(
        parts.first().copied(),
        Some(".bentolifelayout" | ".git" | ".obsidian" | "node_modules" | "schemas")
    ) {
        return true;
    }
    if matches!(path.as_str(), ".DS_Store" | "Thumbs.db") {
        return true;
    }
    if parts
        .iter()
        .any(|part| matches!(*part, ".DS_Store" | "Thumbs.db"))
    {
        return true;
    }
    path.starts_with("modules/navigator/")
        || path == "modules/navigator"
        || path.starts_with("modules/trash/")
        || path == "modules/trash"
        || path.starts_with("modules/archive/")
        || path == "modules/archive"
}

pub fn is_reserved_system_markdown(relative_path: &str) -> bool {
    let path = normalize_import_relative_path(relative_path);
    if path == "INDEX.md" {
        return true;
    }
    let parts = path.split('/').collect::<Vec<_>>();
    parts.len() == 3 && parts[0] == "modules" && matches!(parts[2], "INDEX.md" | "MODULE.md")
}

pub fn is_user_module_data_path(relative_path: &str) -> bool {
    let path = normalize_import_relative_path(relative_path);
    let parts = path.split('/').collect::<Vec<_>>();
    parts.len() >= 4
        && parts[0] == "modules"
        && KNOWN_CONTENT_MODULES.contains(&parts[1])
        && parts[2] == "data"
        && path.ends_with(".md")
}

pub fn should_stage_for_import_review(relative_path: &str, source_kind: &str) -> bool {
    let path = normalize_import_relative_path(relative_path);
    if has_unsafe_relative_path(&path)
        || is_runtime_path(&path)
        || is_reserved_system_markdown(&path)
    {
        return false;
    }
    match source_kind {
        "bentolife_vault" | "snapshot" => is_user_module_data_path(&path),
        _ => true,
    }
}

pub fn should_show_in_import_review(relative_path: &str, source_kind: &str) -> bool {
    let path = normalize_import_relative_path(relative_path);
    should_stage_for_import_review(&path, source_kind) && path.ends_with(".md")
}

pub fn normalize_import_relative_path(relative_path: &str) -> String {
    relative_path.trim().replace('\\', "/")
}

fn has_unsafe_relative_path(relative_path: &str) -> bool {
    relative_path.starts_with('/')
        || relative_path.contains("//")
        || relative_path
            .split('/')
            .any(|part| matches!(part, "" | "." | "..") || part.contains(':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_runtime_and_system_paths() {
        for path in [
            ".bentolifelayout/index.json",
            "INDEX.md",
            "modules/notes/INDEX.md",
            "modules/notes/MODULE.md",
            "modules/navigator/NAVIGATOR.md",
            "modules/trash/INDEX.md",
            "modules/archive/INDEX.md",
            "schemas/modules/note.schema.json",
            ".git/config",
            ".obsidian/config.json",
            "node_modules/pkg/index.js",
            ".DS_Store",
            "Thumbs.db",
        ] {
            assert!(
                !should_show_in_import_review(path, "markdown_folder"),
                "{path}"
            );
        }
    }

    #[test]
    fn shows_only_module_data_markdown_for_bentolife_sources() {
        assert!(should_show_in_import_review(
            "modules/notes/data/daily.md",
            "bentolife_vault"
        ));
        assert!(should_show_in_import_review(
            "modules/habits/data/morning.md",
            "snapshot"
        ));
        assert!(!should_show_in_import_review("Loose.md", "bentolife_vault"));
        assert!(!should_show_in_import_review(
            "assets/banner.png",
            "snapshot"
        ));
    }

    #[test]
    fn generic_markdown_folders_can_stage_assets_but_not_show_them() {
        assert!(should_stage_for_import_review(
            "assets/banner.png",
            "markdown_folder"
        ));
        assert!(!should_show_in_import_review(
            "assets/banner.png",
            "markdown_folder"
        ));
        assert!(should_show_in_import_review("Daily.md", "markdown_folder"));
    }
}
