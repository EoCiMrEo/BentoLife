use std::{fs, path::Path};

pub const LAYOUT_FOLDER: &str = ".bentolifelayout";
pub const REQUIRED_LAYOUT_SUBFOLDERS: [&str; 12] = [
    "documents",
    "layouts",
    "indexes",
    "themes",
    "imports",
    "imports/staged",
    "backups",
    "plugins",
    "orphans",
    "trash",
    "archive",
    "modules",
];

pub struct LayoutFolderService;

impl LayoutFolderService {
    pub fn layout_path(vault_path: &Path) -> std::path::PathBuf {
        vault_path.join(LAYOUT_FOLDER)
    }

    pub fn required_paths(vault_path: &Path) -> Vec<std::path::PathBuf> {
        let layout_path = Self::layout_path(vault_path);
        REQUIRED_LAYOUT_SUBFOLDERS
            .iter()
            .map(|folder| layout_path.join(folder))
            .collect()
    }

    pub fn create_or_repair(vault_path: &Path) -> Result<(), String> {
        let layout_path = Self::layout_path(vault_path);
        fs::create_dir_all(&layout_path).map_err(|error| {
            format!(
                "Unable to create layout folder at {}: {error}",
                layout_path.display()
            )
        })?;

        for path in Self::required_paths(vault_path) {
            fs::create_dir_all(&path)
                .map_err(|error| format!("Unable to create {}: {error}", path.display()))?;
        }

        Ok(())
    }
}
