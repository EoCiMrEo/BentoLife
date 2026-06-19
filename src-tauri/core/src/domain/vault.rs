use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use super::{
    dashboard::DashboardService, layout_folder::LayoutFolderService,
    workspace_metadata::WorkspaceMetadataService,
};

const VAULT_FOLDER_NAME: &str = ".bentolifevault";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VaultState {
    Missing,
    InvalidPath,
    OlderVersionDetected,
    LayoutMissing,
    ScaffoldIncomplete,
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VaultInspection {
    pub path: String,
    pub state: VaultState,
    pub exists: bool,
    pub is_bentolife_vault: bool,
    pub layout_exists: bool,
    pub older_version_detected: bool,
    pub missing_paths: Vec<String>,
    pub message: String,
}

pub struct VaultService;

impl VaultService {
    pub fn default_vault_path_from_home(home_path: PathBuf) -> PathBuf {
        home_path.join("Documents").join(VAULT_FOLDER_NAME)
    }

    pub fn inspect(path: impl Into<PathBuf>) -> VaultInspection {
        let path = path.into();
        Self::inspect_path(&path)
    }

    pub fn create_vault(path: impl Into<PathBuf>) -> Result<VaultInspection, String> {
        let path = path.into();
        Self::ensure_vault_path(&path)?;
        if path.exists() && !Self::is_empty_folder(&path)? && Self::detect_older_vault_shape(&path)
        {
            return Err("Older BentoLife vault structure detected. Back up or snapshot this folder, then create a fresh V3 vault or use copy-only import instead of in-place migration.".to_string());
        }
        fs::create_dir_all(&path)
            .map_err(|error| format!("Unable to create vault at {}: {error}", path.display()))?;
        Self::create_vault_scaffold(&path)?;
        Ok(Self::inspect_path(&path))
    }

    pub fn repair_vault_structure(path: impl Into<PathBuf>) -> Result<VaultInspection, String> {
        let path = path.into();
        Self::ensure_vault_path(&path)?;
        if !path.is_dir() {
            return Err(format!("Vault folder does not exist at {}", path.display()));
        }
        if Self::detect_older_vault_shape(&path) {
            return Err("Repair will not convert older MVP/V2 vault paths into V3. Back up or snapshot the vault, then reinitialize a fresh V3 vault and import copied data explicitly.".to_string());
        }

        Self::create_vault_scaffold(&path)?;
        Ok(Self::inspect_path(&path))
    }

    fn create_vault_scaffold(path: &Path) -> Result<(), String> {
        fs::create_dir_all(path.join("assets"))
            .map_err(|error| format!("Unable to create assets folder: {error}"))?;
        LayoutFolderService::create_or_repair(path)?;
        WorkspaceMetadataService::write_bootstrap_files(path)?;
        DashboardService::ensure_v3_vault_scaffold(path)
    }

    fn inspect_path(path: &Path) -> VaultInspection {
        let missing_paths = Self::missing_required_paths(path);
        let exists = path.exists();
        let layout_exists = LayoutFolderService::layout_path(path).is_dir();
        let is_bentolife_vault = Self::is_named_vault(path);
        let older_version_detected =
            exists && path.is_dir() && Self::detect_older_vault_shape(path);

        if !is_bentolife_vault {
            return VaultInspection {
                path: path.to_string_lossy().to_string(),
                state: VaultState::InvalidPath,
                exists,
                is_bentolife_vault,
                layout_exists,
                older_version_detected,
                missing_paths,
                message: "Select or create the .bentolifevault folder itself.".to_string(),
            };
        }

        if !exists {
            return VaultInspection {
                path: path.to_string_lossy().to_string(),
                state: VaultState::Missing,
                exists,
                is_bentolife_vault,
                layout_exists,
                older_version_detected,
                missing_paths,
                message: "Vault folder has not been created yet.".to_string(),
            };
        }

        if !path.is_dir() {
            return VaultInspection {
                path: path.to_string_lossy().to_string(),
                state: VaultState::Blocked,
                exists,
                is_bentolife_vault,
                layout_exists,
                older_version_detected,
                missing_paths,
                message: "A file exists at the vault path, but BentoLife needs a folder."
                    .to_string(),
            };
        }

        if older_version_detected {
            return VaultInspection {
                path: path.to_string_lossy().to_string(),
                state: VaultState::OlderVersionDetected,
                exists,
                is_bentolife_vault,
                layout_exists,
                older_version_detected,
                missing_paths,
                message: "Older BentoLife vault paths were detected. Back up or snapshot this vault, then create a fresh V3 vault and import copied content explicitly.".to_string(),
            };
        }

        if !layout_exists {
            return VaultInspection {
                path: path.to_string_lossy().to_string(),
                state: VaultState::LayoutMissing,
                exists,
                is_bentolife_vault,
                layout_exists,
                older_version_detected,
                missing_paths,
                message: "Markdown content may be safe, but .bentolifelayout is missing."
                    .to_string(),
            };
        }

        if !missing_paths.is_empty() {
            return VaultInspection {
                path: path.to_string_lossy().to_string(),
                state: VaultState::ScaffoldIncomplete,
                exists,
                is_bentolife_vault,
                layout_exists,
                older_version_detected,
                missing_paths,
                message:
                    "Vault exists but required metadata folders or bootstrap files are missing."
                        .to_string(),
            };
        }

        VaultInspection {
            path: path.to_string_lossy().to_string(),
            state: VaultState::Ready,
            exists,
            is_bentolife_vault,
            layout_exists,
            older_version_detected,
            missing_paths,
            message: "Vault is ready.".to_string(),
        }
    }

    fn missing_required_paths(path: &Path) -> Vec<String> {
        let mut required = vec![
            path.join("assets"),
            LayoutFolderService::layout_path(path),
            LayoutFolderService::layout_path(path).join("schema.json"),
            LayoutFolderService::layout_path(path).join("index.json"),
            LayoutFolderService::layout_path(path).join("workspace_state.json"),
            path.join("modules/trash/INDEX.md"),
            path.join("modules/archive/INDEX.md"),
        ];
        required.extend(LayoutFolderService::required_paths(path));
        required.extend(DashboardService::required_v3_paths(path));

        required
            .into_iter()
            .filter(|required_path| !required_path.exists())
            .map(|required_path| {
                required_path
                    .strip_prefix(path)
                    .unwrap_or(&required_path)
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect()
    }

    fn ensure_vault_path(path: &Path) -> Result<(), String> {
        if !Self::is_named_vault(path) {
            return Err("Vault path must point to the .bentolifevault folder itself.".to_string());
        }

        Ok(())
    }

    fn is_named_vault(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == VAULT_FOLDER_NAME)
    }

    fn is_empty_folder(path: &Path) -> Result<bool, String> {
        if !path.exists() {
            return Ok(true);
        }
        if !path.is_dir() {
            return Ok(false);
        }
        let mut entries = fs::read_dir(path)
            .map_err(|error| format!("Unable to inspect {}: {error}", path.display()))?;
        Ok(entries.next().is_none())
    }

    fn detect_older_vault_shape(path: &Path) -> bool {
        if path.join("notes").is_dir() {
            return true;
        }
        for legacy_file in [
            "modules/todos.md",
            "modules/contacts.md",
            "modules/habits.md",
        ] {
            if path.join(legacy_file).is_file() {
                return true;
            }
        }
        for module in ["notes", "todos", "contacts", "habits"] {
            let module_path = path.join("modules").join(module);
            let Ok(entries) = fs::read_dir(module_path) else {
                continue;
            };
            for entry in entries.flatten() {
                let candidate = entry.path();
                if candidate.is_file()
                    && candidate
                        .extension()
                        .and_then(|extension| extension.to_str())
                        == Some("md")
                    && candidate.file_name().and_then(|name| name.to_str()) != Some("INDEX.md")
                    && candidate.file_name().and_then(|name| name.to_str()) != Some("MODULE.md")
                {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::storage::current_timestamp_label;
    use std::env;

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "bentolife-test-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(VAULT_FOLDER_NAME);
        path
    }

    #[test]
    fn default_vault_path_uses_documents_folder() {
        let path = VaultService::default_vault_path_from_home(PathBuf::from("C:/Users/Test"));

        assert_eq!(
            path,
            PathBuf::from("C:/Users/Test/Documents/.bentolifevault")
        );
    }

    #[test]
    fn rejects_paths_that_are_not_the_vault_folder() {
        let result = VaultService::create_vault(env::temp_dir().join("not-a-vault"));

        assert!(result.is_err());
    }

    #[test]
    fn creates_required_vault_structure() {
        let path = unique_temp_vault("create");

        let inspection = VaultService::create_vault(&path).expect("vault is created");

        assert_eq!(inspection.state, VaultState::Ready);
        assert!(path.join("assets").is_dir());
        assert!(path.join(".bentolifelayout/documents").is_dir());
        assert!(path.join(".bentolifelayout/layouts").is_dir());
        assert!(path.join(".bentolifelayout/index.json").is_file());
        assert!(path.join("modules/notes/data").is_dir());
        assert!(path.join("modules/notes/MODULE.md").is_file());
        assert!(path.join("modules/notes/module.schema.json").is_file());
        assert!(path.join("modules/trash/INDEX.md").is_file());
        assert!(path.join("modules/archive/INDEX.md").is_file());
        assert!(path.join(".bentolifelayout/archive").is_dir());

        let _ = fs::remove_dir_all(path.parent().expect("test parent exists"));
    }

    #[test]
    fn repair_restores_missing_layout_folder() {
        let path = unique_temp_vault("repair");
        fs::create_dir_all(&path).expect("test vault folder exists");

        let before = VaultService::inspect(&path);
        assert_eq!(before.state, VaultState::LayoutMissing);

        let after = VaultService::repair_vault_structure(&path).expect("vault repairs");
        assert_eq!(after.state, VaultState::Ready);

        let _ = fs::remove_dir_all(path.parent().expect("test parent exists"));
    }

    #[test]
    fn classifies_incomplete_scaffold() {
        let path = unique_temp_vault("incomplete");
        fs::create_dir_all(path.join(".bentolifelayout")).expect("partial layout exists");

        let inspection = VaultService::inspect(&path);

        assert_eq!(inspection.state, VaultState::ScaffoldIncomplete);
        assert!(inspection.missing_paths.contains(&"assets".to_string()));

        let _ = fs::remove_dir_all(path.parent().expect("test parent exists"));
    }

    #[test]
    fn detects_older_vault_shapes_without_repairing_them() {
        let path = unique_temp_vault("older");
        fs::create_dir_all(path.join("notes")).expect("legacy notes");
        fs::write(path.join("notes/daily.md"), "# Daily\n").expect("legacy note");

        let inspection = VaultService::inspect(&path);

        assert_eq!(inspection.state, VaultState::OlderVersionDetected);
        assert!(inspection.older_version_detected);
        assert!(VaultService::repair_vault_structure(&path).is_err());

        let _ = fs::remove_dir_all(path.parent().expect("test parent exists"));
    }
}
