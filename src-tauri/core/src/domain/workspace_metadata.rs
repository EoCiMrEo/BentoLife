use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    document_metadata::{DocumentMetadata, DOCUMENT_METADATA_VERSION},
    layout_folder::LAYOUT_FOLDER,
    layout_metadata::LAYOUT_METADATA_VERSION,
    storage::{current_timestamp_label, read_json, write_json_atomic},
    theme::ThemeService,
};

pub const WORKSPACE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentitySchema {
    pub primary: String,
    pub comment_prefix: String,
    pub frontmatter_reference_key: String,
    pub path_mapping_is_authoritative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoragePolicy {
    pub all_workspace_data_lives_inside_vault: bool,
    pub paths_are_vault_relative: bool,
    pub markdown_frontmatter: String,
    pub markdown_content_source_of_truth: bool,
    pub index_is_rebuildable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationPolicy {
    pub backup_before_migration: bool,
    pub never_delete_unknown_metadata_during_scan: bool,
    pub fallback_to_heading_layout_when_layout_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaMetadata {
    pub schema_version: u32,
    pub vault_folder: String,
    pub app_folder: String,
    pub documents_version: u32,
    pub layouts_version: u32,
    pub workspace_state_version: u32,
    pub identity: IdentitySchema,
    pub storage_policy: StoragePolicy,
    pub migration_policy: MigrationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppMetadataPolicy {
    pub paths_are_vault_relative: bool,
    pub markdown_frontmatter: String,
    pub layout_storage: String,
    pub document_registry: String,
    pub identity_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnownDocument {
    pub document_id: String,
    pub metadata_path: String,
    pub current_path: String,
    pub layout_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceState {
    pub schema_version: u32,
    pub workspace_name: String,
    pub vault_folder: String,
    pub vault_root_policy: String,
    pub canonical_app_folder: String,
    pub schema_path: String,
    pub index_path: String,
    pub app_metadata_policy: AppMetadataPolicy,
    pub default_theme: String,
    #[serde(default = "default_workspace_language")]
    pub language: String,
    pub known_documents: Vec<KnownDocument>,
    pub open_tabs: Vec<String>,
    pub recent_files: Vec<String>,
    #[serde(default)]
    pub architect_active_tab: ArchitectTabId,
    #[serde(default)]
    pub architect_sections: BTreeMap<String, bool>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_workspace_language() -> String {
    "en".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectTabId {
    #[default]
    Modules,
    Dashboard,
    Appearance,
    Schemas,
    DataGraph,
    Recovery,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentIndexEntry {
    pub current_path: String,
    pub metadata_path: String,
    pub layout_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RebuildPolicy {
    pub rebuild_from_documents_folder: bool,
    pub rebuild_from_markdown_uuid_comments: bool,
    pub treat_index_as_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceIndex {
    pub schema_version: u32,
    pub path_policy: String,
    pub documents_by_id: BTreeMap<String, DocumentIndexEntry>,
    pub document_ids_by_path: BTreeMap<String, String>,
    pub orphaned_document_ids: Vec<String>,
    pub duplicate_identity_conflicts: Vec<String>,
    pub updated_at: String,
    pub rebuild_policy: RebuildPolicy,
}

pub struct WorkspaceMetadataService;

impl WorkspaceMetadataService {
    pub fn schema_path(vault_path: &Path) -> std::path::PathBuf {
        vault_path.join(LAYOUT_FOLDER).join("schema.json")
    }

    pub fn index_path(vault_path: &Path) -> std::path::PathBuf {
        vault_path.join(LAYOUT_FOLDER).join("index.json")
    }

    pub fn workspace_state_path(vault_path: &Path) -> std::path::PathBuf {
        vault_path.join(LAYOUT_FOLDER).join("workspace_state.json")
    }

    pub fn default_schema() -> SchemaMetadata {
        SchemaMetadata {
            schema_version: WORKSPACE_SCHEMA_VERSION,
            vault_folder: ".bentolifevault".to_string(),
            app_folder: LAYOUT_FOLDER.to_string(),
            documents_version: DOCUMENT_METADATA_VERSION,
            layouts_version: LAYOUT_METADATA_VERSION,
            workspace_state_version: WORKSPACE_SCHEMA_VERSION,
            identity: IdentitySchema {
                primary: "hidden_markdown_uuid_comment".to_string(),
                comment_prefix: "bentolife:document_id=".to_string(),
                frontmatter_reference_key: "bentolife_metadata".to_string(),
                path_mapping_is_authoritative: false,
            },
            storage_policy: StoragePolicy {
                all_workspace_data_lives_inside_vault: true,
                paths_are_vault_relative: true,
                markdown_frontmatter: "reference_only".to_string(),
                markdown_content_source_of_truth: true,
                index_is_rebuildable: true,
            },
            migration_policy: MigrationPolicy {
                backup_before_migration: false,
                never_delete_unknown_metadata_during_scan: true,
                fallback_to_heading_layout_when_layout_missing: true,
            },
        }
    }

    pub fn default_workspace_state() -> WorkspaceState {
        let now = current_timestamp_label();
        WorkspaceState {
            schema_version: WORKSPACE_SCHEMA_VERSION,
            workspace_name: "BentoLife".to_string(),
            vault_folder: ".bentolifevault".to_string(),
            vault_root_policy: "all_markdown_assets_and_metadata_live_inside_vault".to_string(),
            canonical_app_folder: LAYOUT_FOLDER.to_string(),
            schema_path: ".bentolifelayout/schema.json".to_string(),
            index_path: ".bentolifelayout/index.json".to_string(),
            app_metadata_policy: AppMetadataPolicy {
                paths_are_vault_relative: true,
                markdown_frontmatter: "reference_only".to_string(),
                layout_storage: ".bentolifelayout/layouts".to_string(),
                document_registry: ".bentolifelayout/documents".to_string(),
                identity_strategy: "hidden_markdown_uuid_comment".to_string(),
            },
            default_theme: ThemeService::DEFAULT_THEME.to_string(),
            language: default_workspace_language(),
            known_documents: Vec::new(),
            open_tabs: Vec::new(),
            recent_files: Vec::new(),
            architect_active_tab: ArchitectTabId::Modules,
            architect_sections: BTreeMap::from([
                ("appearance_expanded".to_string(), false),
                ("dashboard_customization_expanded".to_string(), true),
                ("dashboard_layout_expanded".to_string(), false),
                ("dashboard_widgets_expanded".to_string(), true),
                ("data_graph_expanded".to_string(), false),
                ("modules_expanded".to_string(), true),
                ("modules_system_expanded".to_string(), false),
                ("modules_starter_expanded".to_string(), true),
                ("modules_optional_expanded".to_string(), true),
                ("recovery_expanded".to_string(), false),
                ("schemas_expanded".to_string(), false),
            ]),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn default_index() -> WorkspaceIndex {
        WorkspaceIndex {
            schema_version: WORKSPACE_SCHEMA_VERSION,
            path_policy: "vault_relative".to_string(),
            documents_by_id: BTreeMap::new(),
            document_ids_by_path: BTreeMap::new(),
            orphaned_document_ids: Vec::new(),
            duplicate_identity_conflicts: Vec::new(),
            updated_at: current_timestamp_label(),
            rebuild_policy: RebuildPolicy {
                rebuild_from_documents_folder: true,
                rebuild_from_markdown_uuid_comments: true,
                treat_index_as_cache: true,
            },
        }
    }

    pub fn write_bootstrap_files(vault_path: &Path) -> Result<(), String> {
        if !Self::schema_path(vault_path).exists() {
            Self::write_schema(vault_path, &Self::default_schema())?;
        }
        if !Self::index_path(vault_path).exists() {
            Self::write_index(vault_path, &Self::default_index())?;
        }
        if !Self::workspace_state_path(vault_path).exists() {
            Self::write_workspace_state(vault_path, &Self::default_workspace_state())?;
        }
        Ok(())
    }

    pub fn read_schema(vault_path: &Path) -> Result<SchemaMetadata, String> {
        let schema = read_json::<SchemaMetadata>(&Self::schema_path(vault_path))?;
        schema.validate()?;
        Ok(schema)
    }

    pub fn write_schema(vault_path: &Path, schema: &SchemaMetadata) -> Result<(), String> {
        schema.validate()?;
        write_json_atomic(&Self::schema_path(vault_path), schema)
    }

    pub fn read_workspace_state(vault_path: &Path) -> Result<WorkspaceState, String> {
        let mut state = read_json::<WorkspaceState>(&Self::workspace_state_path(vault_path))?;
        state.normalize();
        state.validate()?;
        Ok(state)
    }

    pub fn write_workspace_state(vault_path: &Path, state: &WorkspaceState) -> Result<(), String> {
        let mut state = state.clone();
        state.normalize();
        state.validate()?;
        write_json_atomic(&Self::workspace_state_path(vault_path), &state)
    }

    pub fn read_index(vault_path: &Path) -> Result<WorkspaceIndex, String> {
        let index = read_json::<WorkspaceIndex>(&Self::index_path(vault_path))?;
        index.validate()?;
        Ok(index)
    }

    pub fn write_index(vault_path: &Path, index: &WorkspaceIndex) -> Result<(), String> {
        index.validate()?;
        write_json_atomic(&Self::index_path(vault_path), index)
    }

    pub fn rebuild_index_from_documents(
        documents: &[DocumentMetadata],
    ) -> Result<WorkspaceIndex, String> {
        let mut index = Self::default_index();

        for document in documents {
            document.validate()?;
            index.documents_by_id.insert(
                document.document_id.clone(),
                DocumentIndexEntry {
                    current_path: document.current_path.clone(),
                    metadata_path: document.frontmatter_contract.required_value.clone(),
                    layout_path: document.layout_path.clone(),
                },
            );
            index
                .document_ids_by_path
                .insert(document.current_path.clone(), document.document_id.clone());
        }

        index.validate()?;
        Ok(index)
    }

    pub fn register_document(vault_path: &Path, document: &DocumentMetadata) -> Result<(), String> {
        let mut state = if Self::workspace_state_path(vault_path).exists() {
            Self::read_workspace_state(vault_path)?
        } else {
            Self::default_workspace_state()
        };
        state
            .known_documents
            .retain(|known| known.document_id != document.document_id);
        state.known_documents.push(KnownDocument {
            document_id: document.document_id.clone(),
            metadata_path: document.frontmatter_contract.required_value.clone(),
            current_path: document.current_path.clone(),
            layout_path: document.layout_path.clone(),
        });
        state
            .recent_files
            .retain(|path| path != &document.current_path);
        state.recent_files.insert(0, document.current_path.clone());
        state.updated_at = current_timestamp_label();
        Self::write_workspace_state(vault_path, &state)
    }
}

impl SchemaMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != WORKSPACE_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported schema version {}.",
                self.schema_version
            ));
        }
        if self.identity.frontmatter_reference_key != "bentolife_metadata" {
            return Err(
                "Schema must use bentolife_metadata as the frontmatter reference.".to_string(),
            );
        }
        if self.identity.path_mapping_is_authoritative {
            return Err("Path mapping must not be authoritative.".to_string());
        }
        if !self.storage_policy.index_is_rebuildable {
            return Err("index.json must remain rebuildable.".to_string());
        }
        Ok(())
    }
}

impl WorkspaceState {
    pub fn normalize(&mut self) {
        if !matches!(self.language.as_str(), "en" | "vi") {
            self.language = default_workspace_language();
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != WORKSPACE_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported workspace state version {}.",
                self.schema_version
            ));
        }
        if self.index_path != ".bentolifelayout/index.json" {
            return Err(
                "workspace_state.json must reference .bentolifelayout/index.json.".to_string(),
            );
        }
        if !matches!(self.language.as_str(), "en" | "vi") {
            return Err("workspace_state.json language must be 'en' or 'vi'.".to_string());
        }
        Ok(())
    }
}

impl WorkspaceIndex {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != WORKSPACE_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported index version {}.",
                self.schema_version
            ));
        }
        if !self.rebuild_policy.treat_index_as_cache {
            return Err("index.json must be treated as a cache.".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::document_metadata::DocumentMetadataService;

    #[test]
    fn default_workspace_state_points_to_root_index_json() {
        let state = WorkspaceMetadataService::default_workspace_state();

        assert_eq!(state.index_path, ".bentolifelayout/index.json");
        assert_eq!(state.language, "en");
        assert!(state.validate().is_ok());
    }

    #[test]
    fn workspace_language_roundtrips_and_invalid_values_fall_back_to_english() {
        let mut state = WorkspaceMetadataService::default_workspace_state();
        state.language = "vi".to_string();
        state.normalize();
        assert_eq!(state.language, "vi");
        assert!(state.validate().is_ok());

        state.language = "en".to_string();
        state.normalize();
        assert_eq!(state.language, "en");

        state.language = "fr".to_string();
        state.normalize();
        assert_eq!(state.language, "en");
        assert!(state.validate().is_ok());
    }

    #[test]
    fn workspace_state_read_defaults_missing_or_invalid_language_to_english() {
        let mut vault_path = std::env::temp_dir();
        vault_path.push(format!(
            "bentolife-workspace-language-{}",
            current_timestamp_label().replace(':', "-")
        ));
        vault_path.push(".bentolifevault");
        let state_path = WorkspaceMetadataService::workspace_state_path(&vault_path);
        std::fs::create_dir_all(state_path.parent().expect("state parent")).expect("state parent");

        let mut state = WorkspaceMetadataService::default_workspace_state();
        let mut json = serde_json::to_value(&state).expect("state json");
        json.as_object_mut()
            .expect("state object")
            .remove("language");
        std::fs::write(
            &state_path,
            serde_json::to_string_pretty(&json).expect("json"),
        )
        .expect("write missing");
        let read =
            WorkspaceMetadataService::read_workspace_state(&vault_path).expect("read missing");
        assert_eq!(read.language, "en");

        state.language = "not-supported".to_string();
        std::fs::write(
            &state_path,
            serde_json::to_string_pretty(&state).expect("json"),
        )
        .expect("write invalid");
        let read =
            WorkspaceMetadataService::read_workspace_state(&vault_path).expect("read invalid");
        assert_eq!(read.language, "en");

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rebuilt_index_maps_documents_by_id_and_path() {
        let document =
            DocumentMetadataService::create_default("bl_doc_test", "notes/test.md", "# Test\n")
                .expect("document metadata is valid");

        let index = WorkspaceMetadataService::rebuild_index_from_documents(&[document])
            .expect("index rebuilds");

        assert_eq!(
            index.document_ids_by_path.get("notes/test.md"),
            Some(&"bl_doc_test".to_string())
        );
        assert!(index.rebuild_policy.treat_index_as_cache);
    }
}
