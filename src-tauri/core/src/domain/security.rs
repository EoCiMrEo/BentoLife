use std::path::{Component, Path, PathBuf};

pub const VAULT_FOLDER_NAME: &str = ".bentolifevault";
pub const LAYOUT_FOLDER: &str = ".bentolifelayout";
pub const TRASH_FOLDER: &str = ".bentolifelayout/trash";
pub const ARCHIVE_FOLDER: &str = ".bentolifelayout/archive";

const KNOWN_CONTENT_MODULES: [&str; 4] = ["notes", "todos", "contacts", "habits"];

pub fn validate_vault_root_path(vault_path: &Path) -> Result<PathBuf, String> {
    if vault_path.file_name().and_then(|name| name.to_str()) != Some(VAULT_FOLDER_NAME) {
        return Err("Target path must be the .bentolifevault folder itself.".to_string());
    }
    Ok(vault_path.to_path_buf())
}

pub fn validate_known_module_id(module_id: &str) -> Result<String, String> {
    let module_id = module_id.trim();
    if KNOWN_CONTENT_MODULES.contains(&module_id) {
        Ok(module_id.to_string())
    } else {
        Err(format!("Unknown or unsupported module id: {module_id}."))
    }
}

pub fn validate_user_module_markdown_path(relative_path: &str) -> Result<String, String> {
    let normalized = validate_basic_relative_path(relative_path)?;
    let parts = normalized.split('/').collect::<Vec<_>>();
    if parts.len() < 4
        || parts[0] != "modules"
        || parts[2] != "data"
        || !normalized.ends_with(".md")
    {
        return Err(
            "User content mutations must target modules/<module>/data/*.md files.".to_string(),
        );
    }
    validate_known_module_id(parts[1])?;
    reject_runtime_or_system_path(&normalized)?;
    Ok(normalized)
}

pub fn validate_recovery_markdown_path(relative_path: &str) -> Result<String, String> {
    validate_user_module_markdown_path(relative_path)
}

pub fn validate_staged_import_path(relative_path: &str) -> Result<String, String> {
    let normalized = validate_basic_relative_path(relative_path)?;
    if normalized == ".bentolifelayout/imports/staged/import-index.json"
        || !normalized.starts_with(".bentolifelayout/imports/staged/")
    {
        return Err("Import review actions must target staged import files only.".to_string());
    }
    Ok(normalized)
}

pub fn validate_snapshot_relative_path(relative_path: &str) -> Result<String, String> {
    validate_basic_relative_path(relative_path)
}

pub fn validate_runtime_import_target(relative_path: &str) -> Result<String, String> {
    let normalized = validate_basic_relative_path(relative_path)?;
    let allowed = [
        ".bentolifelayout/imports/layouts/",
        ".bentolifelayout/imports/dashboard-widgets/",
        ".bentolifelayout/themes/",
    ];
    if allowed.iter().any(|prefix| normalized.starts_with(prefix)) {
        Ok(normalized)
    } else {
        Err("Runtime import targets must use BentoLife-owned import/theme folders.".to_string())
    }
}

pub fn validate_lifecycle_entry_id(
    entry_id: &str,
    folder: &str,
    suffix: &str,
) -> Result<String, String> {
    let normalized = validate_basic_relative_path(entry_id)?;
    if normalized.starts_with(&format!("{folder}/")) && normalized.ends_with(suffix) {
        Ok(normalized)
    } else {
        Err("Lifecycle entry id does not point to the expected metadata record.".to_string())
    }
}

pub fn validate_lifecycle_file_path(relative_path: &str, folder: &str) -> Result<String, String> {
    let normalized = validate_basic_relative_path(relative_path)?;
    if normalized.starts_with(&format!("{folder}/files/")) {
        Ok(normalized)
    } else {
        Err("Lifecycle file path does not point to the expected internal file folder.".to_string())
    }
}

pub fn require_confirmation_token(expected: &str, provided: &str) -> Result<(), String> {
    if provided == expected {
        Ok(())
    } else {
        Err("Destructive action confirmation token is missing or invalid.".to_string())
    }
}

pub fn trash_confirmation_token(relative_path: &str) -> Result<String, String> {
    Ok(format!(
        "trash:{}",
        validate_user_module_markdown_path(relative_path)?
    ))
}

pub fn archive_confirmation_token(relative_path: &str) -> Result<String, String> {
    Ok(format!(
        "archive:{}",
        validate_user_module_markdown_path(relative_path)?
    ))
}

pub fn delete_trash_confirmation_token(entry_id: &str) -> Result<String, String> {
    Ok(format!(
        "permanently-delete-trash-entry:{}",
        validate_lifecycle_entry_id(entry_id, TRASH_FOLDER, ".trash.json")?
    ))
}

pub fn empty_trash_confirmation_token() -> &'static str {
    "empty-trash"
}

pub fn restore_snapshot_confirmation_token() -> &'static str {
    "restore-vault-snapshot"
}

pub fn apply_entity_upgrade_confirmation_token() -> &'static str {
    "apply-entity-upgrade"
}

pub fn repair_vault_confirmation_token() -> &'static str {
    "repair-vault-structure"
}

fn validate_basic_relative_path(relative_path: &str) -> Result<String, String> {
    if relative_path.contains('\0') {
        return Err("Vault-relative paths must not contain null bytes.".to_string());
    }
    let normalized = relative_path.trim().replace('\\', "/");
    if normalized.is_empty() {
        return Err("Vault-relative path is required.".to_string());
    }
    if normalized.starts_with('/') || normalized.contains("//") {
        return Err("Vault-relative paths must not be absolute.".to_string());
    }
    let candidate = Path::new(&normalized);
    if candidate.is_absolute() {
        return Err("Vault-relative paths must not be absolute.".to_string());
    }
    for component in candidate.components() {
        match component {
            Component::ParentDir => {
                return Err("Vault-relative paths must not escape the vault.".to_string());
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(
                    "Vault-relative paths must not include drive or root prefixes.".to_string(),
                );
            }
            Component::Normal(part) if part.to_string_lossy().contains(':') => {
                return Err(
                    "Vault-relative paths must not contain URL or drive prefixes.".to_string(),
                );
            }
            _ => {}
        }
    }
    Ok(normalized)
}

fn reject_runtime_or_system_path(relative_path: &str) -> Result<(), String> {
    if relative_path.starts_with(LAYOUT_FOLDER)
        || relative_path.starts_with("modules/trash/")
        || relative_path.starts_with("modules/archive/")
        || relative_path.starts_with("modules/navigator/")
        || relative_path == "INDEX.md"
        || relative_path.ends_with("/INDEX.md")
        || relative_path.ends_with("/MODULE.md")
    {
        return Err(
            "User content mutations must not target BentoLife runtime or system files.".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_vault_root_name() {
        assert!(validate_vault_root_path(Path::new("C:/tmp/.bentolifevault")).is_ok());
        assert!(validate_vault_root_path(Path::new("C:/tmp/not-a-vault")).is_err());
    }

    #[test]
    fn rejects_traversal_absolute_and_runtime_paths() {
        for path in [
            "../outside.md",
            "/absolute/path.md",
            "C:/absolute/path.md",
            "modules/notes/data/../../outside.md",
            ".bentolifelayout/index.json",
            "modules/notes/INDEX.md",
            "modules/navigator/NAVIGATOR.md",
        ] {
            assert!(validate_user_module_markdown_path(path).is_err(), "{path}");
        }
        assert!(validate_user_module_markdown_path("modules/notes/data/daily.md").is_ok());
    }

    #[test]
    fn validates_confirmation_tokens() {
        let token = trash_confirmation_token("modules/notes/data/daily.md").expect("token");
        assert!(require_confirmation_token(&token, &token).is_ok());
        assert!(require_confirmation_token(&token, "trash:wrong.md").is_err());
    }

    #[test]
    fn validates_staged_import_and_lifecycle_ids() {
        assert!(
            validate_staged_import_path(".bentolifelayout/imports/staged/folder/Daily.md").is_ok()
        );
        assert!(validate_staged_import_path(".bentolifelayout/index.json").is_err());
        assert!(validate_lifecycle_entry_id(
            ".bentolifelayout/trash/modules-notes-data-daily-md.trash.json",
            TRASH_FOLDER,
            ".trash.json",
        )
        .is_ok());
        assert!(validate_lifecycle_entry_id(
            "modules/notes/data/daily.md",
            TRASH_FOLDER,
            ".trash.json"
        )
        .is_err());
        assert!(validate_lifecycle_file_path(
            ".bentolifelayout/trash/files/modules/notes/data/daily.md",
            TRASH_FOLDER,
        )
        .is_ok());
        assert!(validate_lifecycle_file_path("modules/notes/data/daily.md", TRASH_FOLDER).is_err());
    }
}
