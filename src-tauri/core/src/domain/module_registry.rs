use super::{
    layout_folder::LayoutFolderService,
    module_schema::{ModuleSchema, WidgetTypeDefinition},
    storage::resolve_vault_relative_path,
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryState {
    pub modules: Vec<ModuleDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleDefinition {
    pub id: String,
    pub display_name: String,
    pub kind: String,
    pub document_type: String,
    pub default_path: String,
    pub schema_path: Option<String>,
    pub index_path: String,
    pub data_path: Option<String>,
    pub default_view: String,
    pub enabled_by_default: bool,
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub available: bool,
    #[serde(default = "default_true")]
    pub installed: bool,
    pub storage_kind: String,
    pub capabilities: Vec<String>,
    pub implementation_status: String,
    #[serde(default)]
    pub schema_warnings: Vec<String>,
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub schema_migration_version: Option<u32>,
    #[serde(default)]
    pub widget_types: Vec<WidgetTypeDefinition>,
}

impl Default for ModuleDefinition {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            kind: String::new(),
            document_type: String::new(),
            default_path: String::new(),
            schema_path: None,
            index_path: String::new(),
            data_path: None,
            default_view: String::new(),
            enabled_by_default: false,
            enabled: false,
            available: true,
            installed: true,
            storage_kind: String::new(),
            capabilities: Vec::new(),
            implementation_status: String::new(),
            schema_warnings: Vec::new(),
            schema_version: None,
            schema_migration_version: None,
            widget_types: Vec::new(),
        }
    }
}

pub struct ModuleRegistry;

impl ModuleRegistry {
    pub fn core_modules() -> Vec<ModuleDefinition> {
        vec![
            ModuleDefinition {
                id: "navigator".to_string(),
                display_name: "Navigator".to_string(),
                kind: "system".to_string(),
                document_type: "navigator".to_string(),
                default_path: "modules/navigator/INDEX.md".to_string(),
                schema_path: None,
                index_path: "modules/navigator/INDEX.md".to_string(),
                data_path: None,
                default_view: "system".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "hybrid_managed_markdown_document".to_string(),
                capabilities: vec![
                    "graph_health".to_string(),
                    "backlinks".to_string(),
                    "search".to_string(),
                    "entity_upgrade".to_string(),
                ],
                implementation_status: "implemented".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "trash".to_string(),
                display_name: "Trash".to_string(),
                kind: "system".to_string(),
                document_type: "trash".to_string(),
                default_path: "modules/trash/INDEX.md".to_string(),
                schema_path: None,
                index_path: "modules/trash/INDEX.md".to_string(),
                data_path: None,
                default_view: "system".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "internal".to_string(),
                capabilities: vec![],
                implementation_status: "planned".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "archive".to_string(),
                display_name: "Archive".to_string(),
                kind: "system".to_string(),
                document_type: "archive".to_string(),
                default_path: "modules/archive/INDEX.md".to_string(),
                schema_path: None,
                index_path: "modules/archive/INDEX.md".to_string(),
                data_path: None,
                default_view: "system".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "internal".to_string(),
                capabilities: vec![],
                implementation_status: "planned".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "settings".to_string(),
                display_name: "Settings".to_string(),
                kind: "system".to_string(),
                document_type: "settings".to_string(),
                default_path: ".bentolifelayout/settings/INDEX.md".to_string(),
                schema_path: None,
                index_path: ".bentolifelayout/settings/INDEX.md".to_string(),
                data_path: None,
                default_view: "system".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "internal".to_string(),
                capabilities: vec![],
                implementation_status: "planned".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "vault".to_string(),
                display_name: "Vault".to_string(),
                kind: "system".to_string(),
                document_type: "vault".to_string(),
                default_path: ".bentolifelayout/vault/INDEX.md".to_string(),
                schema_path: None,
                index_path: ".bentolifelayout/vault/INDEX.md".to_string(),
                data_path: None,
                default_view: "system".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "internal".to_string(),
                capabilities: vec![],
                implementation_status: "planned".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "themes".to_string(),
                display_name: "Themes".to_string(),
                kind: "system".to_string(),
                document_type: "themes".to_string(),
                default_path: ".bentolifelayout/themes/INDEX.md".to_string(),
                schema_path: None,
                index_path: ".bentolifelayout/themes/INDEX.md".to_string(),
                data_path: None,
                default_view: "system".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "internal".to_string(),
                capabilities: vec![],
                implementation_status: "planned".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "architect".to_string(),
                display_name: "Architect Mode".to_string(),
                kind: "system".to_string(),
                document_type: "architect".to_string(),
                default_path: ".bentolifelayout/architect/INDEX.md".to_string(),
                schema_path: None,
                index_path: ".bentolifelayout/architect/INDEX.md".to_string(),
                data_path: None,
                default_view: "system".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "internal".to_string(),
                capabilities: vec![],
                implementation_status: "planned".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "notes".to_string(),
                display_name: "Notes".to_string(),
                kind: "starter".to_string(),
                document_type: "note".to_string(),
                default_path: "modules/notes/INDEX.md".to_string(),
                schema_path: Some("modules/notes/module.schema.json".to_string()),
                index_path: "modules/notes/INDEX.md".to_string(),
                data_path: Some("modules/notes/data".to_string()),
                default_view: "cards".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "per_entity_markdown_documents".to_string(),
                capabilities: vec![
                    "create".to_string(),
                    "read".to_string(),
                    "update".to_string(),
                    "rename".to_string(),
                    "dashboard_layout".to_string(),
                ],
                implementation_status: "implemented".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "todos".to_string(),
                display_name: "Todos".to_string(),
                kind: "starter".to_string(),
                document_type: "todos".to_string(),
                default_path: "modules/todos/INDEX.md".to_string(),
                schema_path: Some("modules/todos/module.schema.json".to_string()),
                index_path: "modules/todos/INDEX.md".to_string(),
                data_path: Some("modules/todos/data".to_string()),
                default_view: "cards".to_string(),
                enabled_by_default: true,
                enabled: true,
                storage_kind: "per_entity_markdown_documents".to_string(),
                capabilities: vec![
                    "parse_checkboxes".to_string(),
                    "create_task".to_string(),
                    "toggle_task".to_string(),
                    "dashboard_summary".to_string(),
                ],
                implementation_status: "implemented".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "contacts".to_string(),
                display_name: "Contacts".to_string(),
                kind: "optional".to_string(),
                document_type: "contact".to_string(),
                default_path: "modules/contacts/INDEX.md".to_string(),
                schema_path: Some("modules/contacts/module.schema.json".to_string()),
                index_path: "modules/contacts/INDEX.md".to_string(),
                data_path: Some("modules/contacts/data".to_string()),
                default_view: "cards".to_string(),
                enabled_by_default: false,
                enabled: false,
                storage_kind: "per_entity_markdown_documents".to_string(),
                capabilities: vec![
                    "markdown_friendly_records".to_string(),
                    "create_contact".to_string(),
                    "update_contact".to_string(),
                    "read_contacts".to_string(),
                    "tags".to_string(),
                    "relationship_fields".to_string(),
                    "dashboard_summary".to_string(),
                ],
                implementation_status: "implemented".to_string(),
                ..ModuleDefinition::default()
            },
            ModuleDefinition {
                id: "habits".to_string(),
                display_name: "Habits".to_string(),
                kind: "optional".to_string(),
                document_type: "habit".to_string(),
                default_path: "modules/habits/INDEX.md".to_string(),
                schema_path: Some("modules/habits/module.schema.json".to_string()),
                index_path: "modules/habits/INDEX.md".to_string(),
                data_path: Some("modules/habits/data".to_string()),
                default_view: "cards".to_string(),
                enabled_by_default: false,
                enabled: false,
                storage_kind: "per_entity_markdown_documents".to_string(),
                capabilities: vec![
                    "markdown_friendly_records".to_string(),
                    "create_habit".to_string(),
                    "update_habit".to_string(),
                    "daily_checkins".to_string(),
                    "streak_summary".to_string(),
                    "dashboard_summary".to_string(),
                ],
                implementation_status: "implemented".to_string(),
                ..ModuleDefinition::default()
            },
        ]
    }
}

pub struct ModuleRegistryService;

impl ModuleRegistryService {
    pub fn registry_path(vault_path: &Path) -> std::path::PathBuf {
        LayoutFolderService::layout_path(vault_path)
            .join("modules")
            .join("registry.json")
    }

    pub fn load_registry(vault_path: &Path) -> Result<RegistryState, String> {
        let path = Self::registry_path(vault_path);
        if !path.exists() {
            let state = Self::recovered_registry(vault_path)?;
            Self::save_registry(vault_path, &state)?;
            return Ok(state);
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read registry.json: {}", e))?;
        let loaded = serde_json::from_str::<RegistryState>(&content)
            .map_err(|e| format!("Failed to parse registry.json: {}", e));
        let state = match loaded {
            Ok(state) => {
                let state = Self::normalize_loaded_registry(state);
                if Self::validate_registry(vault_path, &state).is_ok() {
                    Self::repair_missing_modules(vault_path, state)?
                } else {
                    Self::recovered_registry(vault_path)?
                }
            }
            Err(_) => Self::recovered_registry(vault_path)?,
        };
        Self::save_registry(vault_path, &state)?;
        Ok(state)
    }

    pub fn save_registry(vault_path: &Path, state: &RegistryState) -> Result<(), String> {
        Self::validate_registry(vault_path, state)?;
        let path = Self::registry_path(vault_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create registry folder: {}", e))?;
        }
        let content = serde_json::to_string_pretty(state)
            .map_err(|e| format!("Failed to serialize registry state: {}", e))?;
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, content)
            .map_err(|e| format!("Failed to write registry.json: {}", e))?;
        fs::rename(&temp_path, &path)
            .map_err(|e| format!("Failed to rename registry.json: {}", e))?;
        Ok(())
    }

    pub fn set_module_enabled(
        vault_path: &Path,
        module_id: &str,
        enabled: bool,
    ) -> Result<RegistryState, String> {
        let mut state = Self::load_registry(vault_path)?;
        let module_id = canonical_module_id(module_id);
        if let Some(module) = state.modules.iter_mut().find(|m| m.id == module_id) {
            if module.kind == "system" && !enabled {
                return Err(format!("System module {} cannot be disabled", module_id));
            }
            if enabled && !module.installed {
                return Err(format!(
                    "Module {} must be installed before it can be enabled",
                    module_id
                ));
            }
            module.enabled = enabled;
            Self::save_registry(vault_path, &state)?;
        }
        Ok(state)
    }

    fn recovered_registry(vault_path: &Path) -> Result<RegistryState, String> {
        let mut modules = ModuleRegistry::core_modules();
        for module in &mut modules {
            module.enabled = module.enabled_by_default;
            hydrate_module_from_schema(vault_path, module);
        }
        let state = RegistryState { modules };
        Self::validate_registry(vault_path, &state)?;
        Ok(state)
    }

    fn repair_missing_modules(
        vault_path: &Path,
        mut state: RegistryState,
    ) -> Result<RegistryState, String> {
        let known_ids = ModuleRegistry::core_modules()
            .into_iter()
            .map(|module| module.id)
            .collect::<std::collections::BTreeSet<_>>();
        for module in &mut state.modules {
            if !known_ids.contains(&module.id) {
                let warning = format!(
                    "Unknown module package '{}' is preserved but needs review before it can be trusted.",
                    module.id
                );
                if !module
                    .schema_warnings
                    .iter()
                    .any(|existing| existing == &warning)
                {
                    module.schema_warnings.push(warning);
                }
            }
        }
        let existing_ids = state
            .modules
            .iter()
            .map(|module| module.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        for mut module in ModuleRegistry::core_modules() {
            if existing_ids.contains(&module.id) {
                continue;
            }
            hydrate_module_from_schema(vault_path, &mut module);
            module.enabled = module.enabled_by_default;
            state.modules.push(module);
        }
        Self::validate_registry(vault_path, &state)?;
        Ok(state)
    }

    fn validate_registry(vault_path: &Path, state: &RegistryState) -> Result<(), String> {
        let mut ids = std::collections::BTreeSet::new();
        for module in &state.modules {
            if !safe_id(&module.id) {
                return Err(format!("Invalid module ID '{}'.", module.id));
            }
            if !ids.insert(module.id.clone()) {
                return Err(format!("Duplicate module ID '{}'.", module.id));
            }
            if !matches!(module.kind.as_str(), "system" | "starter" | "optional") {
                return Err(format!(
                    "Invalid module kind '{}' for {}.",
                    module.kind, module.id
                ));
            }
            if module.installed && !module.available {
                return Err(format!(
                    "Installed module {} must also be available.",
                    module.id
                ));
            }
            if module.enabled && !module.installed {
                return Err(format!("Enabled module {} must be installed.", module.id));
            }
            validate_vault_path(vault_path, &module.default_path)?;
            validate_vault_path(vault_path, &module.index_path)?;
            if let Some(schema_path) = &module.schema_path {
                validate_vault_path(vault_path, schema_path)?;
            }
            if let Some(data_path) = &module.data_path {
                validate_vault_path(vault_path, data_path)?;
            }
            if module.kind == "system" && !module.enabled {
                return Err(format!("System module {} cannot be disabled.", module.id));
            }
        }
        Ok(())
    }

    fn normalize_loaded_registry(state: RegistryState) -> RegistryState {
        let mut modules = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for mut module in state.modules {
            module.id = canonical_module_id(&module.id).to_string();
            if seen.insert(module.id.clone()) {
                modules.push(module);
            }
        }
        RegistryState { modules }
    }
}

fn default_true() -> bool {
    true
}

fn canonical_module_id(module_id: &str) -> &str {
    if module_id == "todo" {
        "todos"
    } else {
        module_id
    }
}

fn hydrate_module_from_schema(vault_path: &Path, module: &mut ModuleDefinition) {
    let Some(schema_path) = &module.schema_path else {
        return;
    };
    match ModuleSchema::load(vault_path, schema_path) {
        Ok(schema) => {
            module.schema_warnings = schema
                .diagnostics()
                .into_iter()
                .map(|diagnostic| diagnostic.message)
                .collect();
            module.document_type = schema.entity_type;
            if let Some(default_view) = schema.default_view {
                module.default_view = default_view;
            }
            if let Some(display_name) = schema.display_name {
                module.display_name = display_name;
            }
            module.schema_version = Some(schema.schema_version);
            module.schema_migration_version = schema.migration_version;
            module.widget_types = schema.widgets;
        }
        Err(error) => {
            module.schema_warnings = vec![format!(
                "Module schema could not be loaded from {}: {}",
                schema_path, error
            )];
            module.schema_version = None;
            module.schema_migration_version = None;
            module.widget_types = Vec::new();
        }
    }
}

fn safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '-'
        })
}

fn validate_vault_path(vault_path: &Path, relative_path: &str) -> Result<(), String> {
    if relative_path.trim().is_empty()
        || relative_path.contains("..")
        || relative_path.starts_with('/')
        || relative_path.starts_with('\\')
        || relative_path.contains(':')
    {
        return Err(format!(
            "Registry path '{}' is not vault-relative and safe.",
            relative_path
        ));
    }
    let _ = resolve_vault_relative_path(vault_path, relative_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_modules_are_preinstalled_with_portable_storage_paths() {
        let modules = ModuleRegistry::core_modules();

        assert_eq!(
            modules
                .iter()
                .map(|module| module.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "navigator",
                "trash",
                "archive",
                "settings",
                "vault",
                "themes",
                "architect",
                "notes",
                "todos",
                "contacts",
                "habits"
            ]
        );
        assert_eq!(modules[8].default_path, "modules/todos/INDEX.md");
        assert_eq!(modules[9].kind, "optional");
        assert_eq!(
            modules[9].data_path.as_deref(),
            Some("modules/contacts/data")
        );
        assert_eq!(modules[1].index_path, "modules/trash/INDEX.md");
        assert_eq!(modules[2].index_path, "modules/archive/INDEX.md");
        assert!(!modules[10].enabled_by_default);
        assert!(modules
            .iter()
            .filter(|m| m.kind != "system")
            .all(|module| !module.default_path.contains(".bentolifelayout")));
        assert_eq!(modules[9].implementation_status, "implemented");
        assert_eq!(modules[10].implementation_status, "implemented");
        assert!(modules.iter().all(|module| module.available));
        assert!(modules.iter().all(|module| module.installed));
        let ids = modules
            .iter()
            .map(|module| module.id.as_str())
            .collect::<Vec<_>>();
        let mut unique_ids = ids.clone();
        unique_ids.sort_unstable();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len());
    }

    #[test]
    fn repairs_invalid_registry_from_safe_descriptors() {
        let mut vault_path = std::env::temp_dir();
        vault_path.push(format!(
            "bentolife-registry-{}",
            crate::domain::storage::current_timestamp_label().replace(':', "-")
        ));
        vault_path.push(".bentolifevault");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        let path = ModuleRegistryService::registry_path(&vault_path);
        std::fs::create_dir_all(path.parent().expect("registry parent")).expect("parent");
        std::fs::write(
            &path,
            r#"{"modules":[{"id":"../bad","display_name":"Bad","kind":"starter","document_type":"bad","default_path":"../outside.md","schema_path":null,"index_path":"../outside.md","data_path":null,"default_view":"cards","enabled_by_default":true,"enabled":true,"storage_kind":"bad","capabilities":[],"implementation_status":"bad"}]}"#,
        )
        .expect("fixture");

        let repaired = ModuleRegistryService::load_registry(&vault_path).expect("registry");

        assert!(repaired.modules.iter().any(|module| module.id == "notes"));
        assert!(!repaired.modules.iter().any(|module| module.id == "../bad"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn loads_legacy_todo_registry_as_canonical_todos_without_losing_enabled_state() {
        let mut vault_path = std::env::temp_dir();
        vault_path.push(format!(
            "bentolife-registry-legacy-{}",
            crate::domain::storage::current_timestamp_label().replace(':', "-")
        ));
        vault_path.push(".bentolifevault");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        let path = ModuleRegistryService::registry_path(&vault_path);
        std::fs::create_dir_all(path.parent().expect("registry parent")).expect("parent");
        let mut state = RegistryState {
            modules: ModuleRegistry::core_modules(),
        };
        for module in &mut state.modules {
            if module.id == "todos" {
                module.id = "todo".to_string();
                module.enabled = false;
            }
        }
        std::fs::write(&path, serde_json::to_string_pretty(&state).expect("json"))
            .expect("fixture");

        let repaired = ModuleRegistryService::load_registry(&vault_path).expect("registry");

        assert!(!repaired.modules.iter().any(|module| module.id == "todo"));
        let todos = repaired
            .modules
            .iter()
            .find(|module| module.id == "todos")
            .expect("todos");
        assert!(!todos.enabled);
        assert!(todos.installed);
        assert!(todos.available);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn preserves_unknown_safe_modules_with_review_warning() {
        let mut vault_path = std::env::temp_dir();
        vault_path.push(format!(
            "bentolife-registry-unknown-{}",
            crate::domain::storage::current_timestamp_label().replace(':', "-")
        ));
        vault_path.push(".bentolifevault");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        let path = ModuleRegistryService::registry_path(&vault_path);
        std::fs::create_dir_all(path.parent().expect("registry parent")).expect("parent");
        let mut state = RegistryState {
            modules: ModuleRegistry::core_modules(),
        };
        state.modules.push(ModuleDefinition {
            id: "projects".to_string(),
            display_name: "Projects".to_string(),
            kind: "optional".to_string(),
            document_type: "project".to_string(),
            default_path: "modules/projects/INDEX.md".to_string(),
            schema_path: None,
            index_path: "modules/projects/INDEX.md".to_string(),
            data_path: Some("modules/projects/data".to_string()),
            default_view: "cards".to_string(),
            enabled_by_default: false,
            enabled: false,
            storage_kind: "per_entity_markdown_documents".to_string(),
            capabilities: Vec::new(),
            implementation_status: "external".to_string(),
            ..ModuleDefinition::default()
        });
        std::fs::write(&path, serde_json::to_string_pretty(&state).expect("json"))
            .expect("fixture");

        let loaded = ModuleRegistryService::load_registry(&vault_path).expect("registry");
        let projects = loaded
            .modules
            .iter()
            .find(|module| module.id == "projects")
            .expect("projects");

        assert!(projects
            .schema_warnings
            .iter()
            .any(|warning| warning.contains("needs review")));
        assert!(!projects.enabled);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
