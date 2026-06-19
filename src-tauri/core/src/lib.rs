pub mod domain;
mod graph;
mod markdown;
mod recovery;
mod utils;

// Re-export the public utility API so downstream callers are unaffected.
pub use utils::{
    content_hash, content_hash_bytes, current_timestamp_label, ensure_vault_relative_path,
    read_json, resolve_vault_relative_path, write_json_atomic, write_text_atomic,
};

// Re-export the public markdown API so downstream callers are unaffected.
pub use markdown::{
    ensure_identity_comment_at_end, find_identity_comment, format_identity_comment,
    parse_frontmatter, remove_identity_comments,
};

// Re-export the public graph API.
pub use graph::extract_graph_links;

// Internal imports from utils used by domain logic in this file.
use utils::{
    canonicalize_existing, copy_file_verified, file_kind_for_path, normalize_path,
    normalize_relative, slug_from_path, unique_relative_path, vault_relative_path,
};

// Internal imports from graph used by domain logic in this file.
use graph::{
    extract_tags, graph_health_warnings, markdown_excerpt, markdown_headings, resolve_cache_graph,
};

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const LAYOUT_FOLDER: &str = ".bentolifelayout";
pub const DOCUMENTS_FOLDER: &str = ".bentolifelayout/documents";
pub const INDEX_PATH: &str = ".bentolifelayout/index.json";
pub const SEARCH_INDEX_PATH: &str = ".bentolifelayout/indexes/search.json";
pub const SNAPSHOT_MANIFEST: &str = "bentolife-snapshot-manifest.json";
pub const IMPORT_MANIFEST_FOLDER: &str = ".bentolifelayout/imports/folders";
pub const IMPORT_STAGING_FOLDER: &str = ".bentolifelayout/imports/staged";
pub const STAGED_IMPORT_INDEX_PATH: &str = ".bentolifelayout/imports/staged/import-index.json";
pub const ACCEPTED_IMPORT_MANIFEST_FOLDER: &str = ".bentolifelayout/imports/accepted";
pub const ENTITY_UPGRADE_MANIFEST_FOLDER: &str = ".bentolifelayout/imports/entity-upgrades";
pub const TRASH_FOLDER: &str = ".bentolifelayout/trash";
pub const ARCHIVE_FOLDER: &str = ".bentolifelayout/archive";
pub const NAVIGATOR_INDEX_PATH: &str = "modules/navigator/INDEX.md";
pub const NAVIGATOR_DOCUMENT_PATH: &str = "modules/navigator/NAVIGATOR.md";
pub const FRONTMATTER_REFERENCE_KEY: &str = "bentolife_metadata";
pub const IDENTITY_COMMENT_START: &str = "<!-- bentolife:document_id=";
pub const IDENTITY_COMMENT_END: &str = " -->";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct IdentityAnchor {
    pub document_id: String,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ParsedFrontmatter {
    pub metadata_reference: Option<String>,
    pub body: String,
    pub raw_frontmatter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ExternalSourceFile {
    pub source_relative_path: String,
    pub target_relative_path: String,
    pub file_kind: String,
    pub document_id: Option<String>,
    pub title: Option<String>,
    pub content_hash: String,
    pub copied: bool,
    pub collision_renamed: bool,
    pub skipped: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ExternalSourceScan {
    pub source_path: String,
    pub source_kind: String,
    pub markdown_count: u32,
    pub asset_count: u32,
    pub ignored_count: u32,
    pub files: Vec<ExternalSourceFile>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct FolderImportPreview {
    pub source_path: String,
    pub vault_path: String,
    pub scan: ExternalSourceScan,
    pub target_root: String,
    pub planned_files: Vec<ExternalSourceFile>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct FolderImportManifest {
    pub schema_version: u32,
    pub source_path: String,
    pub vault_path: String,
    pub target_root: String,
    pub imported_at: String,
    pub files: Vec<ExternalSourceFile>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SnapshotVaultShape {
    V3ActiveSnapshot,
    LegacyBentolifeSnapshot,
    MixedOrUnknownSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct StagedImportRecord {
    pub staged_file_path: String,
    pub original_source_path: Option<String>,
    pub source_kind: String,
    pub detected_title: Option<String>,
    pub detected_tags: Vec<String>,
    pub detected_links: Vec<String>,
    pub detected_checklists: u32,
    pub suggested_module: String,
    pub accepted: bool,
    pub ignored: bool,
    pub conflict_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct StagedImportIndex {
    pub schema_version: u32,
    pub vault_path: String,
    pub updated_at: String,
    pub records: Vec<StagedImportRecord>,
    #[serde(default)]
    pub hidden_system_count: u32,
    #[serde(default)]
    pub hidden_system_files: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ImportAcceptanceOptions {
    pub target_filename: Option<String>,
    pub tags: Vec<String>,
    pub preserve_source_path: bool,
    pub batch_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ImportAcceptPreview {
    pub vault_path: String,
    pub staged_file_path: String,
    pub target_module: String,
    pub target_relative_path: String,
    pub detected_title: Option<String>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
    pub will_preserve_unknown_content: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ImportAcceptReport {
    pub vault_path: String,
    pub staged_file_path: String,
    pub target_module: String,
    pub accepted_relative_path: String,
    pub accepted_manifest_path: String,
    pub accepted_at: String,
    pub warnings: Vec<String>,
    pub cache: CoreCacheSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct IgnoredStagedImportReport {
    pub vault_path: String,
    pub staged_file_path: String,
    pub ignored_at: String,
    pub index: StagedImportIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct BulkImportAcceptReport {
    pub vault_path: String,
    pub accepted: Vec<ImportAcceptReport>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct SnapshotStageReport {
    pub snapshot_path: String,
    pub target_vault_path: String,
    pub staged_root: String,
    pub staged_at: String,
    pub staged_files: Vec<StagedImportRecord>,
    pub index: StagedImportIndex,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcceptedImportManifest {
    pub schema_version: u32,
    pub accepted_at: String,
    pub source_kind: String,
    pub staged_file_path: String,
    pub target_module: String,
    pub accepted_relative_path: String,
    pub original_source_path: Option<String>,
    pub tags: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct GraphLink {
    pub source_path: String,
    pub target: String,
    pub link_type: String,
    pub raw: String,
    pub status: String,
    pub resolved_document_id: Option<String>,
    pub resolved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct GraphRelationship {
    pub source_path: String,
    pub target: String,
    pub relationship_type: String,
    pub raw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct EntityMetadata {
    pub document_id: Option<String>,
    pub current_path: String,
    pub title: String,
    pub entity_type: String,
    pub metadata_path: Option<String>,
    pub content_hash: String,
    pub backlinks: Vec<GraphLink>,
    pub tags: Vec<String>,
    pub relationships: Vec<GraphRelationship>,
    pub unresolved_links: Vec<GraphLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct GraphHealthWarning {
    pub code: String,
    pub message: String,
    pub document_id: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct DocumentIndexEntry {
    pub current_path: String,
    pub metadata_path: String,
    pub layout_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct RebuildPolicy {
    pub rebuild_from_documents_folder: bool,
    pub rebuild_from_markdown_uuid_comments: bool,
    pub treat_index_as_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct IndexSnapshot {
    pub schema_version: u32,
    pub path_policy: String,
    pub documents_by_id: BTreeMap<String, DocumentIndexEntry>,
    pub document_ids_by_path: BTreeMap<String, String>,
    pub orphaned_document_ids: Vec<String>,
    pub duplicate_identity_conflicts: Vec<String>,
    pub updated_at: String,
    pub rebuild_policy: RebuildPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct CoreCacheSnapshot {
    pub schema_version: u32,
    pub vault_path: String,
    pub entities_by_path: BTreeMap<String, EntityMetadata>,
    pub graph_links: Vec<GraphLink>,
    pub health_warnings: Vec<GraphHealthWarning>,
    pub index: IndexSnapshot,
    pub updated_at: String,
    pub invalidation_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct SearchIndexEntry {
    pub path: String,
    pub document_id: Option<String>,
    pub title: String,
    pub entity_type: String,
    pub tags: Vec<String>,
    pub headings: Vec<String>,
    pub excerpt: String,
    pub searchable_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct SearchIndexSnapshot {
    pub schema_version: u32,
    pub vault_path: String,
    pub index_path: String,
    pub entries: Vec<SearchIndexEntry>,
    pub updated_at: String,
    pub source_cache_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct NavigatorModuleSummary {
    pub module_id: String,
    pub entity_count: u32,
    pub index_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct NavigatorSnapshot {
    pub schema_version: u32,
    pub vault_path: String,
    pub navigator_path: String,
    pub index_path: String,
    pub markdown: String,
    pub module_summaries: Vec<NavigatorModuleSummary>,
    pub health_warnings: Vec<GraphHealthWarning>,
    #[serde(default)]
    pub managed_block_warnings: Vec<String>,
    pub backlinks: Vec<GraphLink>,
    pub search_index_path: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NavigatorRebuildReport {
    pub scan: domain::workspace_scanner::WorkspaceScanResult,
    pub navigator: NavigatorSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct EntityUpgradeChange {
    pub source_path: String,
    pub target_path: String,
    pub entity_type: String,
    pub title: String,
    pub document_id: String,
    pub markdown_body: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct EntityUpgradePreview {
    pub schema_version: u32,
    pub vault_path: String,
    pub changes: Vec<EntityUpgradeChange>,
    pub legacy_paths: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct EntityUpgradeReport {
    pub schema_version: u32,
    pub vault_path: String,
    pub upgraded_at: String,
    pub manifest_path: String,
    pub backup_root: String,
    pub changes: Vec<EntityUpgradeChange>,
    pub trashed_legacy_paths: Vec<String>,
    pub cache: CoreCacheSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct SnapshotFileEntry {
    pub relative_path: String,
    pub file_kind: String,
    pub content_hash: String,
    pub byte_len: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct VaultSnapshotManifest {
    pub schema_version: u32,
    pub source_vault_path: String,
    pub snapshot_path: String,
    pub created_at: String,
    pub source_machine: String,
    pub files: Vec<SnapshotFileEntry>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct VaultSnapshotPreview {
    pub source_vault_path: String,
    pub snapshot_path: String,
    pub file_count: u32,
    pub total_bytes: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct SnapshotRestorePreview {
    pub snapshot_path: String,
    pub target_vault_path: String,
    pub file_count: u32,
    pub snapshot_shape: SnapshotVaultShape,
    pub direct_restore_allowed: bool,
    pub blocked_reason: Option<String>,
    pub legacy_file_count: u32,
    pub active_v3_file_count: u32,
    #[serde(default)]
    pub legacy_file_paths: Vec<String>,
    #[serde(default)]
    pub active_v3_file_paths: Vec<String>,
    #[serde(default)]
    pub hidden_runtime_file_paths: Vec<String>,
    pub recommended_action: String,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct SnapshotRestoreReport {
    pub snapshot_path: String,
    pub target_vault_path: String,
    pub restored_at: String,
    pub restored_files: Vec<SnapshotFileEntry>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
    pub cache: CoreCacheSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct TrashEntry {
    pub original_relative_path: String,
    pub trash_relative_path: String,
    pub trashed_at: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct TrashResult {
    pub action: String,
    pub entry: TrashEntry,
    pub cache: CoreCacheSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ArchiveEntry {
    pub original_relative_path: String,
    pub archive_relative_path: String,
    pub archived_at: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ArchiveResult {
    pub action: String,
    pub entry: ArchiveEntry,
    pub cache: CoreCacheSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct FileLifecycleEntry {
    pub id: String,
    pub original_path: String,
    pub current_path: String,
    pub file_name: String,
    pub module_id: Option<String>,
    pub deleted_or_archived_at: Option<String>,
    pub size_bytes: Option<u64>,
    pub can_restore: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct FileLifecycleMutationReport {
    pub action: String,
    pub message: String,
    pub changed_count: u32,
    pub entries: Vec<FileLifecycleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct WorkspaceSchema {
    pub schema_version: u32,
    pub vault_folder: String,
    pub app_folder: String,
    pub index_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct WorkspaceStateContract {
    pub schema_version: u32,
    pub workspace_name: String,
    pub index_path: String,
    pub default_theme: String,
    pub language: String,
    pub architect_active_tab: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export)]
pub struct ThemeManifest {
    pub schema_version: u32,
    pub theme_id: String,
    pub scope: String,
    pub css_path: Option<String>,
    pub json_path: Option<String>,
    pub active: bool,
}

pub struct CoreCache {
    snapshot: RwLock<Option<CoreCacheSnapshot>>,
}

impl CoreCache {
    pub fn new() -> Self {
        Self {
            snapshot: RwLock::new(None),
        }
    }

    pub fn rebuild_from_vault(
        &self,
        vault_path: &Path,
        reason: &str,
    ) -> Result<CoreCacheSnapshot, String> {
        let mut snapshot = rebuild_cache_from_vault(vault_path)?;
        snapshot.invalidation_reason = Some(reason.to_string());
        let mut guard = self
            .snapshot
            .write()
            .map_err(|_| "Unable to acquire cache write lock.".to_string())?;
        *guard = Some(snapshot.clone());
        Ok(snapshot)
    }

    pub fn snapshot(&self) -> Result<Option<CoreCacheSnapshot>, String> {
        self.snapshot
            .read()
            .map(|guard| guard.clone())
            .map_err(|_| "Unable to acquire cache read lock.".to_string())
    }
}

impl Default for CoreCache {
    fn default() -> Self {
        Self::new()
    }
}

// Markdown parsing functions are now in markdown.rs

pub fn scan_external_source(source_path: &Path) -> Result<ExternalSourceScan, String> {
    let source_path = canonicalize_existing(source_path)?;
    let source_kind = classify_source(&source_path);
    let mut files = Vec::new();
    let mut ignored_count = 0;
    collect_external_files(
        &source_path,
        &source_path,
        &source_kind,
        &mut files,
        &mut ignored_count,
    )?;

    let markdown_count = files
        .iter()
        .filter(|file| file.file_kind == "markdown")
        .count() as u32;
    let asset_count = files
        .iter()
        .filter(|file| file.file_kind == "asset")
        .count() as u32;
    let mut warnings = Vec::new();
    if markdown_count == 0 {
        warnings.push("No Markdown files were found in the selected source.".to_string());
    }

    Ok(ExternalSourceScan {
        source_path: source_path.to_string_lossy().to_string(),
        source_kind,
        markdown_count,
        asset_count,
        ignored_count,
        files,
        warnings,
    })
}

pub fn plan_folder_import(
    source_path: &Path,
    vault_path: &Path,
) -> Result<FolderImportPreview, String> {
    let scan = scan_external_source(source_path)?;
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let target_root = target_root_for_source(&scan);
    let mut occupied = existing_vault_paths(&vault_path)?;
    let mut planned_files = Vec::new();
    let mut conflicts = Vec::new();
    let mut seen_document_ids = BTreeMap::<String, String>::new();

    for mut file in scan.files.clone() {
        if file.skipped {
            planned_files.push(file);
            continue;
        }

        let original_target = format!("{target_root}/{}", file.source_relative_path);
        let unique_target = unique_relative_path(&original_target, &mut occupied);
        if unique_target != original_target {
            file.collision_renamed = true;
            conflicts.push(format!(
                "{original_target} already exists; planned {unique_target}."
            ));
        }
        file.target_relative_path = unique_target.clone();

        if let Some(document_id) = &file.document_id {
            if let Some(first_path) =
                seen_document_ids.insert(document_id.clone(), unique_target.clone())
            {
                conflicts.push(format!(
                    "Document ID {document_id} appears in both {first_path} and {unique_target}; import will copy both for recovery review."
                ));
            }
        }

        planned_files.push(file);
    }

    let mut warnings = scan.warnings.clone();
    warnings
        .push("Import is copy-only; the selected source folder will not be modified.".to_string());
    warnings.push(
        "Imported files are staged under .bentolifelayout/imports and are not active content until accepted into V3 module data folders.".to_string(),
    );

    Ok(FolderImportPreview {
        source_path: scan.source_path.clone(),
        vault_path: vault_path.to_string_lossy().to_string(),
        scan,
        target_root,
        planned_files,
        conflicts,
        warnings,
    })
}

pub fn apply_folder_import(
    source_path: &Path,
    vault_path: &Path,
) -> Result<FolderImportManifest, String> {
    let preview = plan_folder_import(source_path, vault_path)?;
    let source_path = canonicalize_existing(source_path)?;
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let source_kind = preview.scan.source_kind.clone();
    let mut imported_files = Vec::new();

    for mut planned in preview.planned_files {
        if planned.skipped {
            imported_files.push(planned);
            continue;
        }

        let source_file = source_path.join(&planned.source_relative_path);
        let target_file = resolve_vault_relative_path(&vault_path, &planned.target_relative_path)?;
        copy_file_verified(&source_file, &target_file)?;
        planned.copied = true;
        imported_files.push(planned);
    }

    let manifest = FolderImportManifest {
        schema_version: 1,
        source_path: source_path.to_string_lossy().to_string(),
        vault_path: vault_path.to_string_lossy().to_string(),
        target_root: preview.target_root,
        imported_at: current_timestamp_label(),
        files: imported_files,
        conflicts: preview.conflicts,
        warnings: preview.warnings,
    };

    let manifest_path = vault_path.join(IMPORT_MANIFEST_FOLDER).join(format!(
        "folder-import-{}.json",
        manifest.imported_at.replace(':', "-")
    ));
    write_json_atomic(&manifest_path, &manifest)?;
    let mut staged_records = Vec::new();
    for file in manifest.files.iter().filter(|file| file.copied) {
        if !domain::import_policy::should_show_in_import_review(
            &file.source_relative_path,
            &source_kind,
        ) {
            continue;
        }
        staged_records.push(staged_record_from_staged_path(
            &vault_path,
            &file.target_relative_path,
            Some(file.source_relative_path.clone()),
            &source_kind,
        )?);
    }
    let mut index = merge_staged_import_records(&vault_path, staged_records)?;
    if preview.scan.ignored_count > index.hidden_system_count {
        index.hidden_system_count = preview.scan.ignored_count;
        index.warnings.retain(|warning| {
            !warning.starts_with("Hidden system/runtime files omitted from import review:")
        });
        index.warnings.push(format!(
            "Hidden system/runtime files omitted from import review: {}.",
            index.hidden_system_count
        ));
        write_staged_import_index(&vault_path, &index)?;
    }
    let _ = rebuild_cache_from_vault(&vault_path)?;

    Ok(manifest)
}

pub fn list_staged_imports(vault_path: &Path) -> Result<StagedImportIndex, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let index = refresh_staged_import_index(&vault_path)?;
    Ok(index)
}

pub fn preview_accept_import(
    vault_path: &Path,
    staged_file_path: &str,
    target_module: &str,
    options: ImportAcceptanceOptions,
) -> Result<ImportAcceptPreview, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let record = staged_record_for_path(&vault_path, staged_file_path)?;
    ensure_import_record_visible(&record)?;
    let target_module = normalize_import_target_module(target_module)?;
    let target_relative_path =
        accepted_target_relative_path(&vault_path, &record, &target_module, &options)?;
    let conflicts = if vault_path.join(&target_relative_path).exists() {
        vec![format!(
            "{target_relative_path} already exists; accept will choose a safe unique file name."
        )]
    } else {
        Vec::new()
    };
    let mut warnings = Vec::new();
    if record.suggested_module != target_module {
        warnings.push(format!(
            "Suggested module is {}; selected module is {target_module}.",
            record.suggested_module
        ));
    }
    if record.conflict_status.is_some() {
        warnings.push(
            "Imported Markdown has mapping conflicts; content will be preserved.".to_string(),
        );
    }

    Ok(ImportAcceptPreview {
        vault_path: vault_path.to_string_lossy().to_string(),
        staged_file_path: normalize_relative(staged_file_path),
        target_module,
        target_relative_path,
        detected_title: record.detected_title,
        conflicts,
        warnings,
        will_preserve_unknown_content: true,
    })
}

pub fn accept_import_into_module(
    vault_path: &Path,
    staged_file_path: &str,
    target_module: &str,
    options: ImportAcceptanceOptions,
) -> Result<ImportAcceptReport, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let mut index = read_staged_import_index(&vault_path)?;
    let record_index = index
        .records
        .iter()
        .position(|record| record.staged_file_path == normalize_relative(staged_file_path))
        .ok_or_else(|| format!("Staged import {staged_file_path} was not found."))?;
    if index.records[record_index].accepted {
        return Err("Staged import was already accepted.".to_string());
    }
    if index.records[record_index].ignored {
        return Err("Ignored staged import must be reviewed before accepting.".to_string());
    }
    ensure_import_record_visible(&index.records[record_index])?;
    let target_module = normalize_import_target_module(target_module)?;
    let staged_path = resolve_vault_relative_path(&vault_path, staged_file_path)?;
    if staged_path
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("md")
    {
        return Err("Only staged Markdown files can be accepted into modules.".to_string());
    }
    let markdown = fs::read_to_string(&staged_path)
        .map_err(|error| format!("Unable to read {}: {error}", staged_path.display()))?;
    let preview = preview_accept_import(
        &vault_path,
        staged_file_path,
        &target_module,
        options.clone(),
    )?;
    let target_relative_path =
        unique_accept_target_path(&vault_path, &preview.target_relative_path)?;
    let managed = domain::markdown_document::MarkdownDocumentService::manage_document(
        &vault_path,
        &target_relative_path,
        &markdown_with_import_context(&markdown, &index.records[record_index], &options),
    )?;
    restore_dependent_markdown_assets(
        &vault_path,
        &index.records[record_index],
        &markdown,
        &target_module,
        &managed.document_id,
    )?;

    let accepted_at = current_timestamp_label();
    let manifest = AcceptedImportManifest {
        schema_version: 1,
        accepted_at: accepted_at.clone(),
        source_kind: index.records[record_index].source_kind.clone(),
        staged_file_path: index.records[record_index].staged_file_path.clone(),
        target_module: target_module.clone(),
        accepted_relative_path: managed.markdown_relative_path.clone(),
        original_source_path: index.records[record_index].original_source_path.clone(),
        tags: options.tags.clone(),
        warnings: preview.warnings.clone(),
    };
    let accepted_manifest_path = format!(
        "{ACCEPTED_IMPORT_MANIFEST_FOLDER}/accepted-{}.json",
        accepted_at.replace(':', "-")
    );
    write_json_atomic(
        &resolve_vault_relative_path(&vault_path, &accepted_manifest_path)?,
        &manifest,
    )?;

    index.records[record_index].accepted = true;
    index.records[record_index].conflict_status = None;
    index.updated_at = current_timestamp_label();
    write_staged_import_index(&vault_path, &index)?;
    let cache = rebuild_cache_from_vault(&vault_path)?;

    Ok(ImportAcceptReport {
        vault_path: vault_path.to_string_lossy().to_string(),
        staged_file_path: normalize_relative(staged_file_path),
        target_module,
        accepted_relative_path: managed.markdown_relative_path,
        accepted_manifest_path,
        accepted_at,
        warnings: manifest.warnings,
        cache,
    })
}

pub fn ignore_staged_import(
    vault_path: &Path,
    staged_file_path: &str,
) -> Result<IgnoredStagedImportReport, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let mut index = read_staged_import_index(&vault_path)?;
    let normalized = normalize_relative(staged_file_path);
    let record = index
        .records
        .iter_mut()
        .find(|record| record.staged_file_path == normalized)
        .ok_or_else(|| format!("Staged import {staged_file_path} was not found."))?;
    if record.accepted {
        return Err("Accepted staged imports cannot be ignored.".to_string());
    }
    record.ignored = true;
    let ignored_at = current_timestamp_label();
    index.updated_at = ignored_at.clone();
    write_staged_import_index(&vault_path, &index)?;
    Ok(IgnoredStagedImportReport {
        vault_path: vault_path.to_string_lossy().to_string(),
        staged_file_path: normalized,
        ignored_at,
        index,
    })
}

pub fn bulk_accept_imports(
    vault_path: &Path,
    selected_files: Vec<String>,
    target_module: &str,
    options: ImportAcceptanceOptions,
) -> Result<BulkImportAcceptReport, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let mut accepted = Vec::new();
    let mut errors = Vec::new();
    for staged_file in selected_files {
        match accept_import_into_module(&vault_path, &staged_file, target_module, options.clone()) {
            Ok(report) => accepted.push(report),
            Err(error) => errors.push(format!("{staged_file}: {error}")),
        }
    }
    Ok(BulkImportAcceptReport {
        vault_path: vault_path.to_string_lossy().to_string(),
        accepted,
        errors,
    })
}

pub fn rebuild_cache_from_vault(vault_path: &Path) -> Result<CoreCacheSnapshot, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let markdown_files = collect_vault_markdown(&vault_path)?;
    let metadata_by_id = read_document_metadata_index(&vault_path)?;
    let mut paths_by_id = BTreeMap::<String, Vec<String>>::new();
    let mut entities_by_path = BTreeMap::new();
    let mut graph_links = Vec::new();

    for markdown_path in markdown_files {
        let relative_path = vault_relative_path(&vault_path, &markdown_path)?;
        let markdown = fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let parsed = parse_frontmatter(&markdown);
        let identity = find_identity_comment(&markdown);
        if let Some(identity) = &identity {
            paths_by_id
                .entry(identity.document_id.clone())
                .or_default()
                .push(relative_path.clone());
        }
        let links = extract_graph_links(&parsed.body, &relative_path);
        graph_links.extend(links.clone());
        let document_id = identity.map(|identity| identity.document_id);
        let metadata_path = document_id
            .as_ref()
            .and_then(|document_id| metadata_by_id.get(document_id))
            .map(|metadata| metadata.metadata_path.clone())
            .or(parsed.metadata_reference);
        let entity_type = entity_type_for_path(&relative_path);
        let relationships = links
            .iter()
            .filter(|link| link.link_type != "wiki")
            .map(|link| GraphRelationship {
                source_path: link.source_path.clone(),
                target: link.target.clone(),
                relationship_type: link.link_type.clone(),
                raw: link.raw.clone(),
            })
            .collect::<Vec<_>>();
        entities_by_path.insert(
            relative_path.clone(),
            EntityMetadata {
                document_id,
                current_path: relative_path.clone(),
                title: markdown_title(&parsed.body, &relative_path),
                entity_type,
                metadata_path,
                content_hash: content_hash(&markdown),
                backlinks: links,
                tags: extract_tags(&parsed.body),
                relationships,
                unresolved_links: Vec::new(),
            },
        );
    }

    let duplicate_identity_conflicts = paths_by_id
        .iter()
        .filter_map(|(document_id, paths)| (paths.len() > 1).then_some(document_id.clone()))
        .collect::<Vec<_>>();
    let mut index = default_index_snapshot();

    for (path, entity) in &entities_by_path {
        if let (Some(document_id), Some(metadata_path)) =
            (&entity.document_id, &entity.metadata_path)
        {
            if duplicate_identity_conflicts.contains(document_id) {
                continue;
            }
            index.documents_by_id.insert(
                document_id.clone(),
                DocumentIndexEntry {
                    current_path: path.clone(),
                    metadata_path: metadata_path.clone(),
                    layout_path: format!(".bentolifelayout/layouts/{document_id}.layout.json"),
                },
            );
            index
                .document_ids_by_path
                .insert(path.clone(), document_id.clone());
        }
    }
    index.duplicate_identity_conflicts = duplicate_identity_conflicts.clone();
    resolve_cache_graph(&mut entities_by_path, &mut graph_links);
    let mut health_warnings = graph_health_warnings(&entities_by_path, &graph_links);
    for document_id in &duplicate_identity_conflicts {
        health_warnings.push(GraphHealthWarning {
            code: "duplicate_identity".to_string(),
            message: format!("Document ID {document_id} appears in multiple Markdown files."),
            document_id: Some(document_id.clone()),
            path: None,
        });
    }

    write_json_atomic(&vault_path.join(INDEX_PATH), &index)?;

    Ok(CoreCacheSnapshot {
        schema_version: 1,
        vault_path: vault_path.to_string_lossy().to_string(),
        entities_by_path,
        graph_links,
        health_warnings,
        index,
        updated_at: current_timestamp_label(),
        invalidation_reason: None,
    })
}

pub fn rebuild_search_index(vault_path: &Path) -> Result<SearchIndexSnapshot, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let cache = rebuild_cache_from_vault(&vault_path)?;
    let mut entries = Vec::new();

    for (path, entity) in &cache.entities_by_path {
        let markdown_path = resolve_vault_relative_path(&vault_path, path)?;
        let markdown = fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let parsed = parse_frontmatter(&markdown);
        let body = remove_identity_comments(&parsed.body);
        let headings = markdown_headings(&body);
        let excerpt = markdown_excerpt(&body);
        let searchable_text = format!(
            "{} {} {} {}",
            entity.title,
            entity.entity_type,
            entity.tags.join(" "),
            body
        )
        .to_lowercase();
        entries.push(SearchIndexEntry {
            path: path.clone(),
            document_id: entity.document_id.clone(),
            title: entity.title.clone(),
            entity_type: entity.entity_type.clone(),
            tags: entity.tags.clone(),
            headings,
            excerpt,
            searchable_text,
        });
    }

    entries.sort_by_key(|entry| entry.title.to_lowercase());
    let snapshot = SearchIndexSnapshot {
        schema_version: 1,
        vault_path: vault_path.to_string_lossy().to_string(),
        index_path: SEARCH_INDEX_PATH.to_string(),
        entries,
        updated_at: current_timestamp_label(),
        source_cache_updated_at: cache.updated_at,
    };
    write_json_atomic(&vault_path.join(SEARCH_INDEX_PATH), &snapshot)?;
    Ok(snapshot)
}

pub fn search_entities(vault_path: &Path, query: &str) -> Result<SearchIndexSnapshot, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let index_path = vault_path.join(SEARCH_INDEX_PATH);
    let snapshot = if index_path.is_file() {
        read_json::<SearchIndexSnapshot>(&index_path)?
    } else {
        rebuild_search_index(&vault_path)?
    };
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Ok(snapshot);
    }

    Ok(SearchIndexSnapshot {
        entries: snapshot
            .entries
            .into_iter()
            .filter(|entry| entry.searchable_text.contains(&query))
            .collect(),
        ..snapshot
    })
}

pub fn read_navigator(vault_path: &Path) -> Result<NavigatorSnapshot, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    if !vault_path.join(NAVIGATOR_DOCUMENT_PATH).is_file() {
        return rebuild_navigator(&vault_path);
    }
    navigator_snapshot_from_cache(
        &vault_path,
        rebuild_cache_from_vault(&vault_path)?,
        fs::read_to_string(vault_path.join(NAVIGATOR_DOCUMENT_PATH)).map_err(|error| {
            format!(
                "Unable to read {}: {error}",
                vault_path.join(NAVIGATOR_DOCUMENT_PATH).display()
            )
        })?,
        Vec::new(),
    )
}

pub fn rebuild_navigator(vault_path: &Path) -> Result<NavigatorSnapshot, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let cache = rebuild_cache_from_vault(&vault_path)?;
    let _ = rebuild_search_index(&vault_path)?;
    let navigator_path = vault_path.join(NAVIGATOR_DOCUMENT_PATH);
    let existing =
        fs::read_to_string(&navigator_path).unwrap_or_else(|_| "# Navigator\n\n".to_string());
    let (markdown, managed_block_warnings) = render_navigator_markdown(&existing, &cache);
    write_text_atomic(
        &vault_path.join(NAVIGATOR_INDEX_PATH),
        "# Navigator\n\nGraph health and backlinks are rebuilt from active Markdown content.\n",
    )?;
    write_text_atomic(&navigator_path, &markdown)?;
    navigator_snapshot_from_cache(&vault_path, cache, markdown, managed_block_warnings)
}

pub fn scan_and_rebuild_navigator(vault_path: &Path) -> Result<NavigatorRebuildReport, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let scan = domain::workspace_scanner::WorkspaceScanner::scan(&vault_path)?;
    let navigator = rebuild_navigator(&vault_path)?;
    Ok(NavigatorRebuildReport { scan, navigator })
}

pub fn preview_entity_upgrade(vault_path: &Path) -> Result<EntityUpgradePreview, String> {
    recovery::preview_entity_upgrade(
        vault_path,
        collect_vault_markdown,
        existing_vault_paths,
        ensure_vault_folder,
        markdown_title,
    )
}

pub fn apply_entity_upgrade(vault_path: &Path) -> Result<EntityUpgradeReport, String> {
    recovery::apply_entity_upgrade(
        vault_path,
        collect_vault_markdown,
        existing_vault_paths,
        ensure_vault_folder,
        markdown_title,
        write_minimal_document_metadata,
    )
}

pub fn preview_vault_snapshot(
    source_vault_path: &Path,
    snapshot_path: &Path,
) -> Result<VaultSnapshotPreview, String> {
    let source_vault_path = ensure_vault_folder(source_vault_path)?;
    let snapshot_path = normalize_path(snapshot_path)?;
    let files = collect_snapshot_entries(&source_vault_path)?;
    let total_bytes = files.iter().map(|file| file.byte_len).sum();

    Ok(VaultSnapshotPreview {
        source_vault_path: source_vault_path.to_string_lossy().to_string(),
        snapshot_path: snapshot_path.to_string_lossy().to_string(),
        file_count: files.len() as u32,
        total_bytes,
        warnings: vec!["Snapshot creation copies data into the snapshot folder and does not modify the source vault.".to_string()],
    })
}

pub fn create_vault_snapshot(
    source_vault_path: &Path,
    snapshot_path: &Path,
) -> Result<VaultSnapshotManifest, String> {
    let source_vault_path = ensure_vault_folder(source_vault_path)?;
    let snapshot_path = normalize_path(snapshot_path)?;
    let data_path = snapshot_path.join("data");
    fs::create_dir_all(&data_path)
        .map_err(|error| format!("Unable to create {}: {error}", data_path.display()))?;

    let entries = collect_snapshot_entries(&source_vault_path)?;
    for entry in &entries {
        let source_file = source_vault_path.join(&entry.relative_path);
        let target_file = data_path.join(&entry.relative_path);
        copy_file_verified(&source_file, &target_file)?;
    }

    let manifest = VaultSnapshotManifest {
        schema_version: 1,
        source_vault_path: source_vault_path.to_string_lossy().to_string(),
        snapshot_path: snapshot_path.to_string_lossy().to_string(),
        created_at: current_timestamp_label(),
        source_machine: std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string()),
        files: entries,
        warnings: vec![
            "Snapshot is portable folder data; restore copies it into a target .bentolifevault."
                .to_string(),
        ],
    };
    write_json_atomic(&snapshot_path.join(SNAPSHOT_MANIFEST), &manifest)?;
    Ok(manifest)
}

pub fn preview_snapshot_restore(
    snapshot_path: &Path,
    target_vault_path: &Path,
) -> Result<SnapshotRestorePreview, String> {
    let manifest = read_snapshot_manifest(snapshot_path)?;
    let target_vault_path = normalize_path(target_vault_path)?;
    reject_older_vault_target(&target_vault_path)?;
    let shape_details = classify_snapshot_shape(&manifest.files);
    let direct_restore_allowed =
        shape_details.snapshot_shape == SnapshotVaultShape::V3ActiveSnapshot;
    let blocked_reason = if direct_restore_allowed {
        None
    } else {
        Some(
            "This snapshot uses an older or mixed BentoLife structure and must be staged for review."
                .to_string(),
        )
    };
    let recommended_action = if direct_restore_allowed {
        "Direct restore V3 snapshot".to_string()
    } else {
        "Stage snapshot for review".to_string()
    };
    let mut conflicts = Vec::new();
    for file in &manifest.files {
        let relative_path = domain::security::validate_snapshot_relative_path(&file.relative_path)?;
        if target_vault_path.join(&relative_path).exists() {
            conflicts.push(relative_path);
        }
    }

    Ok(SnapshotRestorePreview {
        snapshot_path: manifest.snapshot_path,
        target_vault_path: target_vault_path.to_string_lossy().to_string(),
        file_count: manifest.files.len() as u32,
        snapshot_shape: shape_details.snapshot_shape,
        direct_restore_allowed,
        blocked_reason,
        legacy_file_count: shape_details.legacy_file_paths.len() as u32,
        active_v3_file_count: shape_details.active_v3_file_paths.len() as u32,
        legacy_file_paths: shape_details.legacy_file_paths,
        active_v3_file_paths: shape_details.active_v3_file_paths,
        hidden_runtime_file_paths: shape_details.hidden_runtime_file_paths,
        recommended_action,
        conflicts,
        warnings: vec!["Restore copies snapshot files into the target vault and then rebuilds cache/index state.".to_string()],
    })
}

pub fn restore_vault_snapshot(
    snapshot_path: &Path,
    target_vault_path: &Path,
) -> Result<SnapshotRestoreReport, String> {
    let manifest = read_snapshot_manifest(snapshot_path)?;
    let snapshot_path = normalize_path(snapshot_path)?;
    let target_vault_path = normalize_path(target_vault_path)?;
    reject_older_vault_target(&target_vault_path)?;
    let preview = preview_snapshot_restore(&snapshot_path, &target_vault_path)?;
    if !preview.direct_restore_allowed {
        return Err(preview.blocked_reason.unwrap_or_else(|| {
            "Snapshot cannot be restored directly; stage it for review first.".to_string()
        }));
    }
    let data_path = snapshot_path.join("data");
    fs::create_dir_all(&target_vault_path)
        .map_err(|error| format!("Unable to create {}: {error}", target_vault_path.display()))?;

    let mut restored_files = Vec::new();
    let mut conflicts = Vec::new();
    let mut copied_targets = Vec::new();
    for file in &manifest.files {
        let manifest_relative_path =
            domain::security::validate_snapshot_relative_path(&file.relative_path)?;
        let source_file = data_path.join(&manifest_relative_path);
        let mut target_relative_path = manifest_relative_path.clone();
        if target_vault_path.join(&target_relative_path).exists() {
            let mut occupied = existing_vault_paths(&target_vault_path)?;
            target_relative_path = unique_relative_path(&manifest_relative_path, &mut occupied);
            conflicts.push(format!(
                "{} restored as {}.",
                manifest_relative_path, target_relative_path
            ));
        }
        let target_file = target_vault_path.join(&target_relative_path);
        if let Err(error) = copy_file_verified(&source_file, &target_file) {
            rollback_copied_files(&copied_targets);
            return Err(error);
        }
        copied_targets.push(target_file);
        let mut restored = file.clone();
        restored.relative_path = target_relative_path;
        restored_files.push(restored);
    }
    let cache = match rebuild_cache_from_vault(&target_vault_path) {
        Ok(cache) => cache,
        Err(error) => {
            rollback_copied_files(&copied_targets);
            return Err(error);
        }
    };

    Ok(SnapshotRestoreReport {
        snapshot_path: snapshot_path.to_string_lossy().to_string(),
        target_vault_path: target_vault_path.to_string_lossy().to_string(),
        restored_at: current_timestamp_label(),
        restored_files,
        conflicts,
        warnings: vec!["Source snapshot remains unchanged after restore.".to_string()],
        cache,
    })
}

pub fn stage_snapshot_for_import(
    snapshot_path: &Path,
    target_vault_path: &Path,
) -> Result<SnapshotStageReport, String> {
    let manifest = read_snapshot_manifest(snapshot_path)?;
    let snapshot_path = normalize_path(snapshot_path)?;
    let target_vault_path = ensure_v3_active_vault(target_vault_path)?;
    let staged_at = current_timestamp_label();
    let staged_root = format!(
        "{IMPORT_STAGING_FOLDER}/snapshots/{}",
        staged_at.replace(':', "-")
    );
    let data_path = snapshot_path.join("data");
    let mut records = Vec::new();
    let mut hidden_system_files = Vec::new();
    let mut occupied = existing_vault_paths(&target_vault_path)?;

    for file in &manifest.files {
        let manifest_relative_path =
            domain::security::validate_snapshot_relative_path(&file.relative_path)?;
        let source_file = data_path.join(&manifest_relative_path);
        if is_snapshot_support_asset_path(&manifest_relative_path) {
            let candidate = format!("{staged_root}/{}", manifest_relative_path);
            let staged_relative_path = unique_relative_path(&candidate, &mut occupied);
            let target_file =
                resolve_vault_relative_path(&target_vault_path, &staged_relative_path)?;
            copy_file_verified(&source_file, &target_file)?;
            continue;
        }
        if !domain::import_policy::should_show_in_import_review(&manifest_relative_path, "snapshot")
        {
            hidden_system_files.push(manifest_relative_path);
            continue;
        }
        let candidate = format!("{staged_root}/{}", manifest_relative_path);
        let staged_relative_path = unique_relative_path(&candidate, &mut occupied);
        let target_file = resolve_vault_relative_path(&target_vault_path, &staged_relative_path)?;
        copy_file_verified(&source_file, &target_file)?;
        records.push(staged_record_from_staged_path(
            &target_vault_path,
            &staged_relative_path,
            Some(manifest_relative_path),
            "snapshot",
        )?);
    }

    let mut index = merge_staged_import_records(&target_vault_path, records.clone())?;
    let mut warnings = vec![
        "Snapshot files were staged for review and were not copied into active module folders."
            .to_string(),
    ];
    if !hidden_system_files.is_empty() || !index.hidden_system_files.is_empty() {
        let mut combined_hidden_files = index.hidden_system_files.clone();
        combined_hidden_files.extend(hidden_system_files);
        combined_hidden_files.sort();
        combined_hidden_files.dedup();
        index.hidden_system_files = combined_hidden_files;
        index.hidden_system_count = index.hidden_system_files.len() as u32;
        index.warnings.retain(|warning| {
            !warning.starts_with("Hidden system/runtime files omitted from import review:")
        });
        index.warnings.push(format!(
            "Hidden system/runtime files omitted from import review: {}.",
            index.hidden_system_count
        ));
        write_staged_import_index(&target_vault_path, &index)?;
        warnings.push(format!(
            "{} system/runtime snapshot file(s) were hidden from Import Review.",
            index.hidden_system_count
        ));
    }
    Ok(SnapshotStageReport {
        snapshot_path: snapshot_path.to_string_lossy().to_string(),
        target_vault_path: target_vault_path.to_string_lossy().to_string(),
        staged_root,
        staged_at,
        staged_files: records,
        index,
        warnings,
    })
}

struct SnapshotShapeDetails {
    snapshot_shape: SnapshotVaultShape,
    legacy_file_paths: Vec<String>,
    active_v3_file_paths: Vec<String>,
    hidden_runtime_file_paths: Vec<String>,
}

fn classify_snapshot_shape(files: &[SnapshotFileEntry]) -> SnapshotShapeDetails {
    let mut legacy_file_paths = Vec::new();
    let mut active_v3_file_paths = Vec::new();
    let mut hidden_runtime_file_paths = Vec::new();
    for file in files {
        let path = normalize_relative(&file.relative_path);
        if is_legacy_snapshot_path(&path) {
            legacy_file_paths.push(path.clone());
        }
        if is_active_v3_module_data_path(&path) {
            active_v3_file_paths.push(path.clone());
        }
        if is_hidden_runtime_snapshot_path(&path) {
            hidden_runtime_file_paths.push(path);
        }
    }
    legacy_file_paths.sort();
    active_v3_file_paths.sort();
    hidden_runtime_file_paths.sort();
    let snapshot_shape = match (
        !legacy_file_paths.is_empty(),
        !active_v3_file_paths.is_empty(),
    ) {
        (false, _) => SnapshotVaultShape::V3ActiveSnapshot,
        (true, false) => SnapshotVaultShape::LegacyBentolifeSnapshot,
        (true, true) => SnapshotVaultShape::MixedOrUnknownSnapshot,
    };
    SnapshotShapeDetails {
        snapshot_shape,
        legacy_file_paths,
        active_v3_file_paths,
        hidden_runtime_file_paths,
    }
}

fn is_legacy_snapshot_path(path: &str) -> bool {
    if path.starts_with("notes/") {
        return true;
    }
    if matches!(
        path,
        "modules/todos.md" | "modules/contacts.md" | "modules/habits.md"
    ) {
        return true;
    }
    for module in ["notes", "todos", "contacts", "habits"] {
        let prefix = format!("modules/{module}/");
        if path.starts_with(&prefix)
            && path.ends_with(".md")
            && !path.starts_with(&format!("{prefix}data/"))
            && !path.ends_with("/INDEX.md")
        {
            return true;
        }
    }
    false
}

fn is_active_v3_module_data_path(path: &str) -> bool {
    ["notes", "todos", "contacts", "habits"]
        .iter()
        .any(|module| path.starts_with(&format!("modules/{module}/data/")) && path.ends_with(".md"))
}

fn is_hidden_runtime_snapshot_path(path: &str) -> bool {
    domain::import_policy::is_runtime_path(path)
        || domain::import_policy::is_reserved_system_markdown(path)
}

fn is_snapshot_support_asset_path(path: &str) -> bool {
    let path = normalize_relative(path);
    let parts = path.split('/').collect::<Vec<_>>();
    parts.len() >= 4
        && parts[0] == "assets"
        && ["notes", "todos", "contacts", "habits"].contains(&parts[1])
        && matches!(
            path.rsplit('.').next().map(|extension| extension.to_ascii_lowercase()),
            Some(extension) if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp")
        )
}

fn read_staged_import_index(vault_path: &Path) -> Result<StagedImportIndex, String> {
    let index_path = vault_path.join(STAGED_IMPORT_INDEX_PATH);
    if index_path.is_file() {
        return read_json::<StagedImportIndex>(&index_path);
    }
    Ok(StagedImportIndex {
        schema_version: 1,
        vault_path: vault_path.to_string_lossy().to_string(),
        updated_at: current_timestamp_label(),
        records: Vec::new(),
        hidden_system_count: 0,
        hidden_system_files: Vec::new(),
        warnings: Vec::new(),
    })
}

fn write_staged_import_index(vault_path: &Path, index: &StagedImportIndex) -> Result<(), String> {
    write_json_atomic(&vault_path.join(STAGED_IMPORT_INDEX_PATH), index)
}

fn refresh_staged_import_index(vault_path: &Path) -> Result<StagedImportIndex, String> {
    let stored = read_staged_import_index(vault_path)?;
    let scan = scan_staged_import_records(vault_path)?;
    let mut records = Vec::new();
    for mut record in scan.records {
        if let Some(existing) = stored
            .records
            .iter()
            .find(|existing| existing.staged_file_path == record.staged_file_path)
        {
            record.accepted = existing.accepted;
            record.ignored = existing.ignored;
            if existing.accepted {
                record.conflict_status = None;
            } else if existing.conflict_status.is_some() {
                record.conflict_status = existing.conflict_status.clone();
            }
        }
        records.push(record);
    }
    records.sort_by(|left, right| left.staged_file_path.cmp(&right.staged_file_path));

    let stored_hidden_count = stored.hidden_system_count;
    let stored_hidden_files = stored.hidden_system_files;
    let mut warnings = stored
        .warnings
        .into_iter()
        .filter(|warning| {
            !warning.starts_with("Hidden system/runtime files omitted from import review:")
        })
        .collect::<Vec<_>>();
    let hidden_system_count = stored_hidden_count.max(scan.hidden_system_files.len() as u32);
    let hidden_system_files = if scan.hidden_system_files.is_empty() {
        stored_hidden_files
    } else {
        scan.hidden_system_files
    };
    if hidden_system_count > 0 {
        warnings.push(format!(
            "Hidden system/runtime files omitted from import review: {}.",
            hidden_system_count
        ));
    }

    let index = StagedImportIndex {
        schema_version: 1,
        vault_path: vault_path.to_string_lossy().to_string(),
        updated_at: current_timestamp_label(),
        records,
        hidden_system_count,
        hidden_system_files,
        warnings,
    };
    write_staged_import_index(vault_path, &index)?;
    Ok(index)
}

fn merge_staged_import_records(
    vault_path: &Path,
    records: Vec<StagedImportRecord>,
) -> Result<StagedImportIndex, String> {
    let mut index = read_staged_import_index(vault_path)?;
    for record in records {
        if let Some(existing) = index
            .records
            .iter_mut()
            .find(|existing| existing.staged_file_path == record.staged_file_path)
        {
            *existing = record;
        } else {
            index.records.push(record);
        }
    }
    index
        .records
        .sort_by(|left, right| left.staged_file_path.cmp(&right.staged_file_path));
    index.updated_at = current_timestamp_label();
    write_staged_import_index(vault_path, &index)?;
    refresh_staged_import_index(vault_path)
}

struct StagedImportScan {
    records: Vec<StagedImportRecord>,
    hidden_system_files: Vec<String>,
}

fn scan_staged_import_records(vault_path: &Path) -> Result<StagedImportScan, String> {
    let staging_root = vault_path.join(IMPORT_STAGING_FOLDER);
    if !staging_root.exists() {
        return Ok(StagedImportScan {
            records: Vec::new(),
            hidden_system_files: Vec::new(),
        });
    }
    let mut records = Vec::new();
    let mut hidden_system_files = Vec::new();
    scan_staged_import_records_inner(
        vault_path,
        &staging_root,
        &mut records,
        &mut hidden_system_files,
    )?;
    records.sort_by(|left, right| left.staged_file_path.cmp(&right.staged_file_path));
    hidden_system_files.sort();
    Ok(StagedImportScan {
        records,
        hidden_system_files,
    })
}

fn scan_staged_import_records_inner(
    vault_path: &Path,
    folder: &Path,
    records: &mut Vec<StagedImportRecord>,
    hidden_system_files: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Unable to scan {}: {error}", folder.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read staged entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            scan_staged_import_records_inner(vault_path, &path, records, hidden_system_files)?;
        } else if path.is_file()
            && path.file_name().and_then(|name| name.to_str()) != Some("import-index.json")
        {
            let relative = vault_relative_path(vault_path, &path)?;
            let source = staged_source_relative_path(&relative);
            let source_kind = staged_source_kind(&relative);
            if domain::import_policy::should_show_in_import_review(&source, source_kind) {
                records.push(staged_record_from_staged_path(
                    vault_path,
                    &relative,
                    Some(source),
                    source_kind,
                )?);
            } else {
                hidden_system_files.push(relative);
            }
        }
    }
    Ok(())
}

fn staged_source_kind(staged_file_path: &str) -> &'static str {
    let path = normalize_relative(staged_file_path);
    if path.starts_with(".bentolifelayout/imports/staged/obsidian/") {
        "obsidian_markdown_folder"
    } else if path.starts_with(".bentolifelayout/imports/staged/bentolife-vault/") {
        "bentolife_vault"
    } else if path.starts_with(".bentolifelayout/imports/staged/snapshots/") {
        "snapshot"
    } else {
        "markdown_folder"
    }
}

fn staged_source_relative_path(staged_file_path: &str) -> String {
    let path = normalize_relative(staged_file_path);
    for prefix in [
        ".bentolifelayout/imports/staged/obsidian/",
        ".bentolifelayout/imports/staged/bentolife-vault/",
        ".bentolifelayout/imports/staged/folders/bentolife-vault/",
        ".bentolifelayout/imports/staged/folders/",
        ".bentolifelayout/imports/staged/folder/",
    ] {
        if let Some(source) = path.strip_prefix(prefix) {
            return source.to_string();
        }
    }
    if let Some(source) = path.strip_prefix(".bentolifelayout/imports/staged/snapshots/") {
        let parts = source.split('/').collect::<Vec<_>>();
        if parts.len() > 1 {
            return parts[1..].join("/");
        }
    }
    path
}

fn staged_record_for_path(
    vault_path: &Path,
    staged_file_path: &str,
) -> Result<StagedImportRecord, String> {
    let normalized = normalize_relative(staged_file_path);
    let index = list_staged_imports(vault_path)?;
    index
        .records
        .into_iter()
        .find(|record| record.staged_file_path == normalized)
        .ok_or_else(|| format!("Staged import {staged_file_path} was not found."))
}

fn ensure_import_record_visible(record: &StagedImportRecord) -> Result<(), String> {
    let source_path = import_record_source_relative_path(record);
    if domain::import_policy::should_show_in_import_review(&source_path, &record.source_kind) {
        Ok(())
    } else {
        Err("System/runtime staged files cannot be accepted into modules.".to_string())
    }
}

fn import_record_source_relative_path(record: &StagedImportRecord) -> String {
    record
        .original_source_path
        .clone()
        .unwrap_or_else(|| staged_source_relative_path(&record.staged_file_path))
}

fn staged_record_from_staged_path(
    vault_path: &Path,
    staged_file_path: &str,
    original_source_path: Option<String>,
    source_kind: &str,
) -> Result<StagedImportRecord, String> {
    let normalized = normalize_relative(staged_file_path);
    let path = resolve_vault_relative_path(vault_path, &normalized)?;
    let mut detected_title = None;
    let mut detected_tags = Vec::new();
    let mut detected_links = Vec::new();
    let mut detected_checklists = 0;
    let mut suggested_module = "notes".to_string();
    let mut conflict_status = None;
    if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
        let markdown = fs::read_to_string(&path)
            .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
        let parsed = parse_frontmatter(&markdown);
        detected_title = Some(markdown_title(&parsed.body, &normalized));
        detected_tags = extract_import_tags(&parsed.body);
        detected_links = extract_import_links(&parsed.body);
        detected_checklists = parsed
            .body
            .lines()
            .filter(|line| {
                line.trim_start().starts_with("- [") || line.trim_start().starts_with("* [")
            })
            .count() as u32;
        let suggestion_source_path = original_source_path.as_deref().unwrap_or(&normalized);
        suggested_module = suggest_module_for_import(
            Some(suggestion_source_path),
            &parsed.body,
            detected_checklists,
        );
        if suggested_module != "notes" && contains_unknown_module_markers(&parsed.body) {
            conflict_status = Some(
                "Structured module suggestion found; review unsupported Markdown before accepting."
                    .to_string(),
            );
        }
    } else {
        conflict_status =
            Some("Assets stay staged and are not accepted directly into modules.".to_string());
    }

    Ok(StagedImportRecord {
        staged_file_path: normalized,
        original_source_path,
        source_kind: source_kind.to_string(),
        detected_title,
        detected_tags,
        detected_links,
        detected_checklists,
        suggested_module,
        accepted: false,
        ignored: false,
        conflict_status,
    })
}

fn extract_import_tags(markdown: &str) -> Vec<String> {
    let mut tags = BTreeSet::new();
    for token in markdown.split_whitespace() {
        let cleaned = token
            .trim_matches(|character: char| !character.is_ascii_alphanumeric() && character != '#')
            .trim_start_matches('#');
        if !cleaned.is_empty() && token.starts_with('#') {
            tags.insert(cleaned.to_string());
        }
    }
    tags.into_iter().collect()
}

fn extract_import_links(markdown: &str) -> Vec<String> {
    let mut links = Vec::new();
    for segment in markdown.split("[[").skip(1) {
        if let Some((target, _)) = segment.split_once("]]") {
            links.push(target.trim().to_string());
        }
    }
    links
}

fn suggest_module_for_import(
    source_path: Option<&str>,
    markdown: &str,
    checklist_count: u32,
) -> String {
    if let Some(module) = module_from_import_source_path(source_path) {
        return module.to_string();
    }

    let lower = markdown.to_lowercase();
    if lower.contains("status:")
        || lower.contains("priority:")
        || lower.contains("due date:")
        || lower.contains("due:")
        || checklist_count > 0
    {
        "todos".to_string()
    } else if lower.contains("email:")
        || lower.contains("phone:")
        || lower.contains("relationship:")
    {
        "contacts".to_string()
    } else if lower.contains("frequency:") || lower.contains("check-in") || lower.contains("streak")
    {
        "habits".to_string()
    } else {
        "notes".to_string()
    }
}

fn module_from_import_source_path(source_path: Option<&str>) -> Option<&'static str> {
    let source = source_path?.replace('\\', "/").to_ascii_lowercase();
    for module in ["notes", "todos", "contacts", "habits"] {
        if source.starts_with(&format!("modules/{module}/data/"))
            || source.contains(&format!("/modules/{module}/data/"))
        {
            return Some(module);
        }
    }
    None
}

fn contains_unknown_module_markers(markdown: &str) -> bool {
    markdown.contains("```") || markdown.contains("|---") || markdown.contains("unknown:")
}

fn normalize_import_target_module(target_module: &str) -> Result<String, String> {
    match target_module.trim().to_ascii_lowercase().as_str() {
        "notes" | "note" => Ok("notes".to_string()),
        "todos" | "todo" => Ok("todos".to_string()),
        "contacts" | "contact" => Ok("contacts".to_string()),
        "habits" | "habit" => Ok("habits".to_string()),
        _ => Err(
            "Imported files can only be accepted into Notes, Todos, Contacts, or Habits."
                .to_string(),
        ),
    }
}

fn accepted_target_relative_path(
    vault_path: &Path,
    record: &StagedImportRecord,
    target_module: &str,
    options: &ImportAcceptanceOptions,
) -> Result<String, String> {
    let file_name = options
        .target_filename
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(slug_from_path)
        .unwrap_or_else(|| {
            record
                .detected_title
                .as_deref()
                .map(slug_from_path)
                .unwrap_or_else(|| slug_from_path(&record.staged_file_path))
        });
    let candidate = format!("modules/{target_module}/data/{file_name}.md");
    ensure_vault_relative_path(&candidate)?;
    let _ = vault_path;
    Ok(candidate)
}

fn unique_accept_target_path(
    vault_path: &Path,
    target_relative_path: &str,
) -> Result<String, String> {
    let mut occupied = existing_vault_paths(vault_path)?;
    Ok(unique_relative_path(target_relative_path, &mut occupied))
}

fn markdown_with_import_context(
    markdown: &str,
    record: &StagedImportRecord,
    options: &ImportAcceptanceOptions,
) -> String {
    let mut body = markdown.trim_end().to_string();
    let mut context = Vec::new();
    if options.preserve_source_path {
        if let Some(source_path) = &record.original_source_path {
            context.push(format!("Source: {source_path}"));
        }
    }
    let mut seen_context = BTreeSet::new();
    for tag in &options.tags {
        push_unique_import_context(&mut context, &mut seen_context, &format!("#{tag}"));
    }
    if let Some(batch_tag) = &options.batch_tag {
        push_unique_import_context(&mut context, &mut seen_context, &format!("#{batch_tag}"));
    }
    if !context.is_empty() {
        body.push_str("\n\n<!-- bentolife:import_context ");
        body.push_str(&context.join(" "));
        body.push_str(" -->");
    }
    body.push('\n');
    body
}

fn restore_dependent_markdown_assets(
    vault_path: &Path,
    record: &StagedImportRecord,
    markdown: &str,
    target_module: &str,
    document_id: &str,
) -> Result<Vec<String>, String> {
    let source_document_path = import_record_source_relative_path(record);
    let expected_prefix = format!("assets/{target_module}/{document_id}/");
    let parsed = domain::markdown_parser::MarkdownParser::parse(markdown);
    let mut restored = Vec::new();
    let mut seen = BTreeSet::new();
    let mut sources = Vec::new();
    collect_markdown_image_sources(&parsed.blocks, &mut sources);

    for source in sources {
        let asset_relative_path =
            match resolve_markdown_asset_reference(&source_document_path, &source) {
                Ok(path) => path,
                Err(_) => continue,
            };
        if !asset_relative_path.starts_with(&expected_prefix)
            || !is_snapshot_support_asset_path(&asset_relative_path)
            || !seen.insert(asset_relative_path.clone())
        {
            continue;
        }
        let Some(staged_asset_relative_path) =
            staged_support_relative_path(record, &asset_relative_path)
        else {
            continue;
        };
        let source_file = resolve_vault_relative_path(vault_path, &staged_asset_relative_path)?;
        if !source_file.is_file() {
            continue;
        }
        let target_file = resolve_vault_relative_path(vault_path, &asset_relative_path)?;
        if target_file.exists() {
            restored.push(asset_relative_path);
            continue;
        }
        copy_file_verified(&source_file, &target_file)?;
        restored.push(asset_relative_path);
    }

    Ok(restored)
}

fn collect_markdown_image_sources(
    blocks: &[domain::markdown_parser::MarkdownBlock],
    sources: &mut Vec<String>,
) {
    for block in blocks {
        match block {
            domain::markdown_parser::MarkdownBlock::Image { source, .. } => {
                sources.push(source.clone());
            }
            domain::markdown_parser::MarkdownBlock::Blockquote { children } => {
                collect_markdown_image_sources(children, sources);
            }
            _ => {}
        }
    }
}

fn staged_support_relative_path(
    record: &StagedImportRecord,
    source_relative_path: &str,
) -> Option<String> {
    let staged_file_path = normalize_relative(&record.staged_file_path);
    if let Some(original_source_path) = record.original_source_path.as_deref() {
        let original_source_path = normalize_relative(original_source_path);
        if let Some(root) = staged_file_path.strip_suffix(&original_source_path) {
            return Some(format!("{root}{source_relative_path}"));
        }
    }

    let parts = staged_file_path.split('/').collect::<Vec<_>>();
    let root_len = if parts.len() >= 5
        && parts[0] == ".bentolifelayout"
        && parts[1] == "imports"
        && parts[2] == "staged"
        && parts[3] == "snapshots"
    {
        5
    } else if parts.len() >= 4
        && parts[0] == ".bentolifelayout"
        && parts[1] == "imports"
        && parts[2] == "staged"
        && matches!(parts[3], "bentolife-vault" | "folders" | "obsidian")
    {
        4
    } else {
        return None;
    };
    Some(format!(
        "{}/{}",
        parts[..root_len].join("/"),
        source_relative_path
    ))
}

fn resolve_markdown_asset_reference(
    document_relative_path: &str,
    source: &str,
) -> Result<String, String> {
    let source = source.trim().replace('\\', "/");
    let lower = source.to_ascii_lowercase();
    if source.is_empty()
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:")
        || lower.starts_with("javascript:")
        || lower.starts_with("file:")
        || Path::new(&source).is_absolute()
    {
        return Err("Markdown asset source must be a safe vault-relative image path.".to_string());
    }

    let document_path = document_relative_path.replace('\\', "/");
    let document_parts = document_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut parts = if source.starts_with("assets/") {
        Vec::new()
    } else {
        document_parts
            .iter()
            .take(document_parts.len().saturating_sub(1))
            .map(|part| (*part).to_string())
            .collect::<Vec<_>>()
    };

    for part in source.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("Markdown asset source must not escape the vault.".to_string());
                }
            }
            _ if part.contains(':') => {
                return Err("Markdown asset source must not contain a URL scheme.".to_string());
            }
            _ => parts.push(part.to_string()),
        }
    }

    if parts.first().map(String::as_str) != Some("assets") {
        return Err("Markdown asset source must resolve under assets/.".to_string());
    }

    Ok(parts.join("/"))
}

fn rollback_copied_files(paths: &[PathBuf]) {
    for path in paths.iter().rev() {
        let _ = fs::remove_file(path);
    }
}

pub fn trash_managed_entity(vault_path: &Path, relative_path: &str) -> Result<TrashResult, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let relative_path = domain::security::validate_user_module_markdown_path(relative_path)?;
    let source_path = resolve_vault_relative_path(&vault_path, &relative_path)?;
    if !source_path.is_file() {
        return Err(format!("Cannot trash missing file {relative_path}."));
    }
    let content = fs::read(&source_path)
        .map_err(|error| format!("Unable to read {}: {error}", source_path.display()))?;
    let hash = content_hash_bytes(&content);
    let trash_relative_path = unique_trash_path(&vault_path, &relative_path)?;
    let trash_path = vault_path.join(&trash_relative_path);
    copy_file_verified(&source_path, &trash_path)?;
    fs::remove_file(&source_path)
        .map_err(|error| format!("Unable to remove {}: {error}", source_path.display()))?;
    let entry = TrashEntry {
        original_relative_path: normalize_relative(&relative_path),
        trash_relative_path,
        trashed_at: current_timestamp_label(),
        content_hash: hash,
    };
    write_json_atomic(
        &vault_path
            .join(TRASH_FOLDER)
            .join(format!("{}.trash.json", slug_from_path(&relative_path))),
        &entry,
    )?;
    let cache = rebuild_cache_from_vault(&vault_path)?;
    Ok(TrashResult {
        action: "trash_managed_entity".to_string(),
        entry,
        cache,
    })
}

pub fn restore_trashed_entity(
    vault_path: &Path,
    trash_relative_path: &str,
    restore_relative_path: &str,
) -> Result<TrashResult, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let trash_relative_path =
        domain::security::validate_lifecycle_file_path(trash_relative_path, TRASH_FOLDER)?;
    let restore_relative_path =
        domain::security::validate_user_module_markdown_path(restore_relative_path)?;
    let trash_path = resolve_vault_relative_path(&vault_path, &trash_relative_path)?;
    if !trash_path.is_file() {
        return Err(format!(
            "Cannot restore missing trash file {trash_relative_path}."
        ));
    }
    let restore_path = resolve_vault_relative_path(&vault_path, &restore_relative_path)?;
    if restore_path.exists() {
        return Err(format!(
            "Refusing to overwrite existing restore path {restore_relative_path}."
        ));
    }
    let content = fs::read(&trash_path)
        .map_err(|error| format!("Unable to read {}: {error}", trash_path.display()))?;
    let entry = TrashEntry {
        original_relative_path: normalize_relative(&restore_relative_path),
        trash_relative_path: normalize_relative(&trash_relative_path),
        trashed_at: current_timestamp_label(),
        content_hash: content_hash_bytes(&content),
    };
    copy_file_verified(&trash_path, &restore_path)?;
    fs::remove_file(&trash_path)
        .map_err(|error| format!("Unable to remove {}: {error}", trash_path.display()))?;
    let cache = rebuild_cache_from_vault(&vault_path)?;
    Ok(TrashResult {
        action: "restore_trashed_entity".to_string(),
        entry,
        cache,
    })
}

pub fn archive_managed_entity(
    vault_path: &Path,
    relative_path: &str,
) -> Result<ArchiveResult, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let relative_path = domain::security::validate_user_module_markdown_path(relative_path)?;
    let source_path = resolve_vault_relative_path(&vault_path, &relative_path)?;
    if !source_path.is_file() {
        return Err(format!("Cannot archive missing file {relative_path}."));
    }
    let content = fs::read(&source_path)
        .map_err(|error| format!("Unable to read {}: {error}", source_path.display()))?;
    let hash = content_hash_bytes(&content);
    let archive_relative_path = unique_archive_path(&vault_path, &relative_path)?;
    let archive_path = vault_path.join(&archive_relative_path);
    copy_file_verified(&source_path, &archive_path)?;
    fs::remove_file(&source_path)
        .map_err(|error| format!("Unable to remove {}: {error}", source_path.display()))?;
    let entry = ArchiveEntry {
        original_relative_path: normalize_relative(&relative_path),
        archive_relative_path,
        archived_at: current_timestamp_label(),
        content_hash: hash,
    };
    write_json_atomic(
        &vault_path
            .join(ARCHIVE_FOLDER)
            .join(format!("{}.archive.json", slug_from_path(&relative_path))),
        &entry,
    )?;
    let cache = rebuild_cache_from_vault(&vault_path)?;
    Ok(ArchiveResult {
        action: "archive_managed_entity".to_string(),
        entry,
        cache,
    })
}

pub fn restore_archived_entity(
    vault_path: &Path,
    archive_relative_path: &str,
    restore_relative_path: &str,
) -> Result<ArchiveResult, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let archive_relative_path =
        domain::security::validate_lifecycle_file_path(archive_relative_path, ARCHIVE_FOLDER)?;
    let restore_relative_path =
        domain::security::validate_user_module_markdown_path(restore_relative_path)?;
    let archive_path = resolve_vault_relative_path(&vault_path, &archive_relative_path)?;
    if !archive_path.is_file() {
        return Err(format!(
            "Cannot restore missing archive file {archive_relative_path}."
        ));
    }
    let restore_path = resolve_vault_relative_path(&vault_path, &restore_relative_path)?;
    if restore_path.exists() {
        return Err(format!(
            "Refusing to overwrite existing restore path {restore_relative_path}."
        ));
    }
    let content = fs::read(&archive_path)
        .map_err(|error| format!("Unable to read {}: {error}", archive_path.display()))?;
    let entry = ArchiveEntry {
        original_relative_path: normalize_relative(&restore_relative_path),
        archive_relative_path: normalize_relative(&archive_relative_path),
        archived_at: current_timestamp_label(),
        content_hash: content_hash_bytes(&content),
    };
    copy_file_verified(&archive_path, &restore_path)?;
    fs::remove_file(&archive_path)
        .map_err(|error| format!("Unable to remove {}: {error}", archive_path.display()))?;
    let cache = rebuild_cache_from_vault(&vault_path)?;
    Ok(ArchiveResult {
        action: "restore_archived_entity".to_string(),
        entry,
        cache,
    })
}

pub fn list_trash_entries(vault_path: &Path) -> Result<Vec<FileLifecycleEntry>, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    collect_lifecycle_entries(
        &vault_path,
        TRASH_FOLDER,
        ".trash.json",
        LifecycleKind::Trash,
    )
}

pub fn list_archive_entries(vault_path: &Path) -> Result<Vec<FileLifecycleEntry>, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    collect_lifecycle_entries(
        &vault_path,
        ARCHIVE_FOLDER,
        ".archive.json",
        LifecycleKind::Archive,
    )
}

pub fn restore_trash_entry(vault_path: &Path, entry_id: &str) -> Result<TrashResult, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let metadata_path =
        resolve_lifecycle_metadata_path(&vault_path, entry_id, TRASH_FOLDER, ".trash.json")?;
    let entry = read_json::<TrashEntry>(&metadata_path)?;
    let restore_relative_path = safe_restore_path(&vault_path, &entry.original_relative_path)?;
    let result = restore_trashed_entity(
        &vault_path,
        &entry.trash_relative_path,
        &restore_relative_path,
    )?;
    remove_file_if_present(&metadata_path)?;
    Ok(result)
}

pub fn restore_archive_entry(vault_path: &Path, entry_id: &str) -> Result<ArchiveResult, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    let metadata_path =
        resolve_lifecycle_metadata_path(&vault_path, entry_id, ARCHIVE_FOLDER, ".archive.json")?;
    let entry = read_json::<ArchiveEntry>(&metadata_path)?;
    let restore_relative_path = safe_restore_path(&vault_path, &entry.original_relative_path)?;
    let result = restore_archived_entity(
        &vault_path,
        &entry.archive_relative_path,
        &restore_relative_path,
    )?;
    remove_file_if_present(&metadata_path)?;
    Ok(result)
}

pub fn delete_trash_entry_permanently(
    vault_path: &Path,
    entry_id: &str,
) -> Result<Vec<FileLifecycleEntry>, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    delete_trash_entry_internal(&vault_path, entry_id)?;
    list_trash_entries(&vault_path)
}

pub fn empty_trash(vault_path: &Path) -> Result<Vec<FileLifecycleEntry>, String> {
    let vault_path = ensure_v3_active_vault(vault_path)?;
    for entry in list_trash_entries(&vault_path)? {
        delete_trash_entry_internal(&vault_path, &entry.id)?;
    }
    list_trash_entries(&vault_path)
}

#[derive(Copy, Clone)]
enum LifecycleKind {
    Trash,
    Archive,
}

fn collect_lifecycle_entries(
    vault_path: &Path,
    folder: &str,
    suffix: &str,
    kind: LifecycleKind,
) -> Result<Vec<FileLifecycleEntry>, String> {
    let metadata_root = vault_path.join(folder);
    if !metadata_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for item in fs::read_dir(&metadata_root)
        .map_err(|error| format!("Unable to read {}: {error}", metadata_root.display()))?
    {
        let path = item
            .map_err(|error| {
                format!(
                    "Unable to read lifecycle entry in {}: {error}",
                    metadata_root.display()
                )
            })?
            .path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.ends_with(suffix) {
            continue;
        }

        match lifecycle_entry_from_metadata(vault_path, &path, kind) {
            Ok(entry) => entries.push(entry),
            Err(_) => continue,
        }
    }

    entries.sort_by(|left, right| {
        right
            .deleted_or_archived_at
            .cmp(&left.deleted_or_archived_at)
            .then_with(|| left.original_path.cmp(&right.original_path))
    });
    Ok(entries)
}

fn lifecycle_entry_from_metadata(
    vault_path: &Path,
    metadata_path: &Path,
    kind: LifecycleKind,
) -> Result<FileLifecycleEntry, String> {
    match kind {
        LifecycleKind::Trash => {
            let entry = read_json::<TrashEntry>(metadata_path)?;
            Ok(file_lifecycle_entry(
                vault_path,
                metadata_path,
                &entry.original_relative_path,
                &entry.trash_relative_path,
                Some(entry.trashed_at),
            ))
        }
        LifecycleKind::Archive => {
            let entry = read_json::<ArchiveEntry>(metadata_path)?;
            Ok(file_lifecycle_entry(
                vault_path,
                metadata_path,
                &entry.original_relative_path,
                &entry.archive_relative_path,
                Some(entry.archived_at),
            ))
        }
    }
}

fn file_lifecycle_entry(
    vault_path: &Path,
    metadata_path: &Path,
    original_path: &str,
    current_path: &str,
    deleted_or_archived_at: Option<String>,
) -> FileLifecycleEntry {
    let current = vault_path.join(current_path);
    let metadata_id = vault_relative_path(vault_path, metadata_path)
        .unwrap_or_else(|_| metadata_path.to_string_lossy().to_string());
    FileLifecycleEntry {
        id: metadata_id,
        original_path: normalize_relative(original_path),
        current_path: normalize_relative(current_path),
        file_name: lifecycle_file_name(original_path),
        module_id: module_id_from_relative_path(original_path),
        deleted_or_archived_at,
        size_bytes: fs::metadata(&current).ok().map(|metadata| metadata.len()),
        can_restore: current.is_file(),
    }
}

fn lifecycle_file_name(relative_path: &str) -> String {
    Path::new(relative_path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| normalize_relative(relative_path))
}

fn module_id_from_relative_path(relative_path: &str) -> Option<String> {
    let normalized = normalize_relative(relative_path);
    let mut parts = normalized.split('/');
    match (parts.next(), parts.next()) {
        (Some("modules"), Some(module_id)) => Some(module_id.to_string()),
        _ => None,
    }
}

fn resolve_lifecycle_metadata_path(
    vault_path: &Path,
    entry_id: &str,
    folder: &str,
    suffix: &str,
) -> Result<PathBuf, String> {
    let relative = domain::security::validate_lifecycle_entry_id(entry_id, folder, suffix)?;
    let metadata_path = resolve_vault_relative_path(vault_path, &relative)?;
    if !metadata_path.is_file() {
        return Err(format!(
            "Lifecycle entry metadata was not found: {relative}."
        ));
    }
    Ok(metadata_path)
}

fn safe_restore_path(vault_path: &Path, original_relative_path: &str) -> Result<String, String> {
    let original_relative_path = normalize_relative(original_relative_path);
    let original_path = resolve_vault_relative_path(vault_path, &original_relative_path)?;
    if !original_path.exists() {
        return Ok(original_relative_path);
    }
    let mut occupied = existing_vault_paths(vault_path)?;
    Ok(unique_relative_path(&original_relative_path, &mut occupied))
}

fn delete_trash_entry_internal(vault_path: &Path, entry_id: &str) -> Result<(), String> {
    let metadata_path =
        resolve_lifecycle_metadata_path(vault_path, entry_id, TRASH_FOLDER, ".trash.json")?;
    let entry = read_json::<TrashEntry>(&metadata_path)?;
    let trash_path = resolve_vault_relative_path(vault_path, &entry.trash_relative_path)?;
    remove_file_if_present(&trash_path)?;
    remove_file_if_present(&metadata_path)
}

fn remove_file_if_present(path: &Path) -> Result<(), String> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("Unable to remove {}: {error}", path.display()))?;
    }
    Ok(())
}

fn collect_external_files(
    source_root: &Path,
    folder: &Path,
    source_kind: &str,
    files: &mut Vec<ExternalSourceFile>,
    ignored_count: &mut u32,
) -> Result<(), String> {
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Unable to scan {}: {error}", folder.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read source entry: {error}"))?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if should_ignore_source_name(file_name) {
            *ignored_count += count_files_under(&path)?;
            continue;
        }
        let relative = vault_relative_path(source_root, &path)?;
        if path != source_root && path.is_dir() && domain::import_policy::is_runtime_path(&relative)
        {
            *ignored_count += count_files_under(&path)?;
            continue;
        }
        if path.is_dir() {
            collect_external_files(source_root, &path, source_kind, files, ignored_count)?;
        } else if path.is_file() {
            if !domain::import_policy::should_stage_for_import_review(&relative, source_kind) {
                *ignored_count += 1;
                continue;
            }
            let file_kind = file_kind_for_path(&path);
            let mut title = None;
            let mut document_id = None;
            let bytes = fs::read(&path)
                .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
            if file_kind == "markdown" {
                let markdown = String::from_utf8_lossy(&bytes);
                let parsed = parse_frontmatter(&markdown);
                title = Some(markdown_title(&parsed.body, &relative));
                document_id = find_identity_comment(&markdown).map(|identity| identity.document_id);
            }
            files.push(ExternalSourceFile {
                source_relative_path: relative,
                target_relative_path: String::new(),
                file_kind,
                document_id,
                title,
                content_hash: content_hash_bytes(&bytes),
                copied: false,
                collision_renamed: false,
                skipped: false,
                reason: None,
            });
        }
    }
    files.sort_by(|left, right| left.source_relative_path.cmp(&right.source_relative_path));
    Ok(())
}

fn push_unique_import_context(context: &mut Vec<String>, seen: &mut BTreeSet<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    let key = trimmed.to_ascii_lowercase();
    if seen.insert(key) {
        context.push(trimmed.to_string());
    }
}

fn should_ignore_source_name(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".obsidian" | ".trash" | "node_modules" | ".DS_Store" | "Thumbs.db"
    )
}

fn count_files_under(path: &Path) -> Result<u32, String> {
    if path.is_file() {
        return Ok(1);
    }
    if !path.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in
        fs::read_dir(path).map_err(|error| format!("Unable to scan {}: {error}", path.display()))?
    {
        let entry =
            entry.map_err(|error| format!("Unable to read hidden source entry: {error}"))?;
        count += count_files_under(&entry.path())?;
    }
    Ok(count)
}

fn classify_source(source_path: &Path) -> String {
    if source_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".bentolifevault")
        || source_path.join(".bentolifelayout").is_dir()
    {
        "bentolife_vault".to_string()
    } else if source_path.join(".obsidian").is_dir()
        || source_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name.eq_ignore_ascii_case(".obsidianvault")
                    || name.to_lowercase().contains("obsidian")
            })
    {
        "obsidian_markdown_folder".to_string()
    } else {
        "markdown_folder".to_string()
    }
}

fn target_root_for_source(scan: &ExternalSourceScan) -> String {
    if scan.source_kind == "obsidian_markdown_folder" {
        format!("{IMPORT_STAGING_FOLDER}/obsidian")
    } else if scan.source_kind == "bentolife_vault" {
        format!("{IMPORT_STAGING_FOLDER}/bentolife-vault")
    } else {
        format!("{IMPORT_STAGING_FOLDER}/folders")
    }
}

// file_kind_for_path, unique_relative_path are in utils.rs

fn existing_vault_paths(vault_path: &Path) -> Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();
    if vault_path.exists() {
        collect_existing_paths(vault_path, vault_path, &mut paths)?;
    }
    Ok(paths)
}

fn collect_existing_paths(
    root: &Path,
    folder: &Path,
    paths: &mut BTreeSet<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Unable to scan {}: {error}", folder.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read vault entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_existing_paths(root, &path, paths)?;
        } else if path.is_file() {
            paths.insert(vault_relative_path(root, &path)?);
        }
    }
    Ok(())
}

fn ensure_vault_folder(vault_path: &Path) -> Result<PathBuf, String> {
    let path = normalize_path(vault_path)?;
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == ".bentolifevault")
    {
        fs::create_dir_all(&path)
            .map_err(|error| format!("Unable to create {}: {error}", path.display()))?;
        ensure_core_layout(&path)?;
        Ok(path)
    } else {
        Err("Target path must be the .bentolifevault folder itself.".to_string())
    }
}

fn ensure_v3_active_vault(vault_path: &Path) -> Result<PathBuf, String> {
    let path = normalize_path(vault_path)?;
    if path.file_name().and_then(|name| name.to_str()) != Some(".bentolifevault") {
        return Err("Target path must be the .bentolifevault folder itself.".to_string());
    }
    reject_older_vault_target(&path)?;
    fs::create_dir_all(&path)
        .map_err(|error| format!("Unable to create {}: {error}", path.display()))?;
    ensure_core_layout(&path)?;
    Ok(path)
}

fn reject_older_vault_target(vault_path: &Path) -> Result<(), String> {
    recovery::reject_older_vault_target(vault_path)
}

fn ensure_core_layout(vault_path: &Path) -> Result<(), String> {
    for folder in [
        ".bentolifelayout",
        DOCUMENTS_FOLDER,
        ".bentolifelayout/layouts",
        ".bentolifelayout/indexes",
        ".bentolifelayout/themes",
        ".bentolifelayout/imports",
        IMPORT_STAGING_FOLDER,
        ACCEPTED_IMPORT_MANIFEST_FOLDER,
        ".bentolifelayout/backups",
        IMPORT_MANIFEST_FOLDER,
        ENTITY_UPGRADE_MANIFEST_FOLDER,
        TRASH_FOLDER,
        ARCHIVE_FOLDER,
        "modules/navigator",
        "modules/notes",
        "modules/notes/data",
        "modules/notes/views",
        "modules/notes/templates",
        "modules/notes/theme/json",
        "modules/notes/theme/css",
        "modules/todos",
        "modules/todos/data",
        "modules/todos/views",
        "modules/todos/templates",
        "modules/todos/theme/json",
        "modules/todos/theme/css",
        "modules/contacts",
        "modules/contacts/data",
        "modules/contacts/views",
        "modules/contacts/templates",
        "modules/contacts/theme/json",
        "modules/contacts/theme/css",
        "modules/habits",
        "modules/habits/data",
        "modules/habits/views",
        "modules/habits/templates",
        "modules/habits/theme/json",
        "modules/habits/theme/css",
        "modules/trash",
        "modules/archive",
    ] {
        fs::create_dir_all(vault_path.join(folder)).map_err(|error| {
            format!(
                "Unable to create {}: {error}",
                vault_path.join(folder).display()
            )
        })?;
    }
    Ok(())
}

// canonicalize_existing and normalize_path are in utils.rs

fn collect_vault_markdown(vault_path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    collect_vault_markdown_inner(vault_path, vault_path, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_vault_markdown_inner(
    vault_path: &Path,
    folder: &Path,
    paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if should_skip_active_scan_folder(vault_path, folder) {
        return Ok(());
    }
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Unable to scan {}: {error}", folder.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read vault entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_vault_markdown_inner(vault_path, &path, paths)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("md")
            && !should_skip_active_scan_markdown_file(vault_path, &path)
        {
            paths.push(path);
        }
    }
    let _ = vault_path;
    Ok(())
}

fn should_skip_active_scan_folder(vault_path: &Path, folder: &Path) -> bool {
    let Ok(relative) = folder.strip_prefix(vault_path) else {
        return false;
    };
    let normalized = relative.to_string_lossy().replace('\\', "/");
    normalized == LAYOUT_FOLDER
        || normalized.starts_with(&format!("{LAYOUT_FOLDER}/"))
        || normalized == "modules/trash"
        || normalized.starts_with("modules/trash/")
        || normalized == "modules/archive"
        || normalized.starts_with("modules/archive/")
}

fn should_skip_active_scan_markdown_file(vault_path: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(vault_path) else {
        return false;
    };
    let normalized = relative.to_string_lossy().replace('\\', "/");
    is_reserved_v3_system_markdown(&normalized)
}

fn is_reserved_v3_system_markdown(relative_path: &str) -> bool {
    let normalized = normalize_relative(relative_path);
    normalized == NAVIGATOR_INDEX_PATH
        || normalized == NAVIGATOR_DOCUMENT_PATH
        || normalized == "modules/trash/INDEX.md"
        || normalized == "modules/archive/INDEX.md"
        || (normalized.starts_with("modules/") && normalized.ends_with("/MODULE.md"))
}

fn read_document_metadata_index(
    vault_path: &Path,
) -> Result<BTreeMap<String, DocumentMetadataRecord>, String> {
    let mut records = BTreeMap::new();
    let documents_path = vault_path.join(DOCUMENTS_FOLDER);
    if !documents_path.exists() {
        return Ok(records);
    }
    for entry in fs::read_dir(&documents_path)
        .map_err(|error| format!("Unable to read {}: {error}", documents_path.display()))?
    {
        let entry =
            entry.map_err(|error| format!("Unable to read document metadata entry: {error}"))?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let value = read_json::<serde_json::Value>(&path)?;
        if let Some(document_id) = value.get("document_id").and_then(|value| value.as_str()) {
            records.insert(
                document_id.to_string(),
                DocumentMetadataRecord {
                    metadata_path: vault_relative_path(vault_path, &path)?,
                },
            );
        }
    }
    Ok(records)
}

struct DocumentMetadataRecord {
    metadata_path: String,
}

fn default_index_snapshot() -> IndexSnapshot {
    IndexSnapshot {
        schema_version: 1,
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

// graph helpers are now in graph.rs

fn navigator_snapshot_from_cache(
    vault_path: &Path,
    cache: CoreCacheSnapshot,
    markdown: String,
    managed_block_warnings: Vec<String>,
) -> Result<NavigatorSnapshot, String> {
    Ok(NavigatorSnapshot {
        schema_version: 1,
        vault_path: vault_path.to_string_lossy().to_string(),
        navigator_path: NAVIGATOR_DOCUMENT_PATH.to_string(),
        index_path: NAVIGATOR_INDEX_PATH.to_string(),
        markdown,
        module_summaries: module_summaries(&cache),
        health_warnings: cache.health_warnings,
        managed_block_warnings,
        backlinks: cache.graph_links,
        search_index_path: SEARCH_INDEX_PATH.to_string(),
        updated_at: current_timestamp_label(),
    })
}

fn module_summaries(cache: &CoreCacheSnapshot) -> Vec<NavigatorModuleSummary> {
    let mut counts = BTreeMap::<String, u32>::new();
    for entity in cache.entities_by_path.values() {
        *counts.entry(entity.entity_type.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(module_id, entity_count)| NavigatorModuleSummary {
            index_path: format!("modules/{module_id}s/INDEX.md"),
            module_id,
            entity_count,
        })
        .collect()
}

fn render_navigator_markdown(existing: &str, cache: &CoreCacheSnapshot) -> (String, Vec<String>) {
    let mut markdown = existing.trim().to_string();
    let mut warnings = Vec::new();
    if markdown.is_empty() {
        markdown = "# Navigator".to_string();
    }
    let update = replace_managed_block(
        &markdown,
        "navigator-module-summary",
        &render_module_summary_block(cache),
    );
    markdown = update.markdown;
    warnings.extend(update.warnings);
    let update = replace_managed_block(
        &markdown,
        "navigator-backlinks",
        &render_backlinks_block(&cache.graph_links),
    );
    markdown = update.markdown;
    warnings.extend(update.warnings);
    let update = replace_managed_block(
        &markdown,
        "navigator-graph-health",
        &render_health_block(&cache.health_warnings),
    );
    warnings.extend(update.warnings);
    (update.markdown, warnings)
}

struct ManagedBlockUpdate {
    markdown: String,
    warnings: Vec<String>,
}

fn replace_managed_block(markdown: &str, name: &str, replacement: &str) -> ManagedBlockUpdate {
    let start_marker = format!("<!-- bentolife:managed-block start name=\"{name}\" -->");
    let end_marker = format!("<!-- bentolife:managed-block end name=\"{name}\" -->");
    let block = format!("{start_marker}\n{}\n{end_marker}", replacement.trim());
    let mut warnings = Vec::new();
    let Some(start) = markdown.find(&start_marker) else {
        return ManagedBlockUpdate {
            markdown: format!("{}\n\n{}\n", markdown.trim_end(), block),
            warnings,
        };
    };
    let Some(end_offset) = markdown[start..].find(&end_marker) else {
        warnings.push(format!(
            "Repaired unclosed managed block '{name}' while rebuilding Navigator."
        ));
        return ManagedBlockUpdate {
            markdown: format!("{}\n{}\n", markdown[..start].trim_end(), block),
            warnings,
        };
    };
    let end = start + end_offset + end_marker.len();
    let suffix = remove_duplicate_managed_blocks(
        &markdown[end..],
        &start_marker,
        &end_marker,
        name,
        &mut warnings,
    );
    ManagedBlockUpdate {
        markdown: format!("{}{}{}", &markdown[..start].trim_end(), block, suffix),
        warnings,
    }
}

fn remove_duplicate_managed_blocks<'a>(
    mut suffix: &'a str,
    start_marker: &str,
    end_marker: &str,
    name: &str,
    warnings: &mut Vec<String>,
) -> &'a str {
    loop {
        let trimmed = suffix.trim_start();
        let removed_whitespace = suffix.len() - trimmed.len();
        if !trimmed.starts_with(start_marker) {
            return suffix;
        }
        let Some(end_offset) = trimmed.find(end_marker) else {
            warnings.push(format!(
                "Removed duplicate unclosed managed block '{name}' while rebuilding Navigator."
            ));
            return "";
        };
        warnings.push(format!(
            "Collapsed duplicate managed block '{name}' while rebuilding Navigator."
        ));
        let next_start = removed_whitespace + end_offset + end_marker.len();
        suffix = &suffix[next_start..];
    }
}

fn render_module_summary_block(cache: &CoreCacheSnapshot) -> String {
    let summaries = module_summaries(cache);
    if summaries.is_empty() {
        return "No graph entities found.".to_string();
    }
    summaries
        .into_iter()
        .map(|summary| {
            format!(
                "- {}: {} entities ({})",
                summary.module_id, summary.entity_count, summary.index_path
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_backlinks_block(links: &[GraphLink]) -> String {
    if links.is_empty() {
        return "No backlinks found.".to_string();
    }
    links
        .iter()
        .map(|link| {
            let target = link
                .resolved_path
                .as_deref()
                .unwrap_or(link.target.as_str());
            format!(
                "- {} -> {} ({}, {})",
                link.source_path, target, link.link_type, link.status
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_health_block(warnings: &[GraphHealthWarning]) -> String {
    if warnings.is_empty() {
        return "Graph health is clean.".to_string();
    }
    warnings
        .iter()
        .map(|warning| format!("- {}: {}", warning.code, warning.message))
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_minimal_document_metadata(
    vault_path: &Path,
    document_id: &str,
    markdown_relative_path: &str,
    document_type: &str,
    markdown_content: &str,
) -> Result<(), String> {
    let now = current_timestamp_label();
    let metadata_path = format!(".bentolifelayout/documents/{document_id}.json");
    let layout_path = format!(".bentolifelayout/layouts/{document_id}.layout.json");
    let metadata = serde_json::json!({
        "schema_version": 1,
        "document_id": document_id,
        "document_type": document_type,
        "vault_relative": true,
        "current_path": normalize_relative(markdown_relative_path),
        "previous_paths": [],
        "layout_path": layout_path,
        "identity": {
            "strategy": "hidden_markdown_uuid_comment",
            "comment": format_identity_comment(document_id)
        },
        "frontmatter_contract": {
            "required_key": "bentolife_metadata",
            "required_value": metadata_path,
            "allowed_app_metadata_in_markdown": ["bentolife_metadata"]
        },
        "content_policy": {
            "content_lives_inside_vault": true,
            "markdown_is_content_source_of_truth": true,
            "layout_is_stored_in_bentolife_folder": true,
            "full_content_is_not_duplicated_in_metadata": true
        },
        "content_hash": content_hash(markdown_content),
        "recovery_status": "managed",
        "created_at": now,
        "updated_at": now
    });
    write_json_atomic(&vault_path.join(metadata_path), &metadata)
}

fn collect_snapshot_entries(vault_path: &Path) -> Result<Vec<SnapshotFileEntry>, String> {
    let mut entries = Vec::new();
    collect_snapshot_entries_inner(vault_path, vault_path, &mut entries)?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(entries)
}

fn collect_snapshot_entries_inner(
    root: &Path,
    folder: &Path,
    entries: &mut Vec<SnapshotFileEntry>,
) -> Result<(), String> {
    for entry in fs::read_dir(folder)
        .map_err(|error| format!("Unable to scan {}: {error}", folder.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read snapshot entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_snapshot_entries_inner(root, &path, entries)?;
        } else if path.is_file() {
            let relative_path = vault_relative_path(root, &path)?;
            if should_skip_export_snapshot_path(&relative_path) {
                continue;
            }
            let bytes = fs::read(&path)
                .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
            entries.push(SnapshotFileEntry {
                relative_path,
                file_kind: file_kind_for_path(&path),
                content_hash: content_hash_bytes(&bytes),
                byte_len: bytes.len() as u64,
            });
        }
    }
    Ok(())
}

fn should_skip_export_snapshot_path(relative_path: &str) -> bool {
    is_legacy_snapshot_path(&normalize_relative(relative_path))
}

fn read_snapshot_manifest(snapshot_path: &Path) -> Result<VaultSnapshotManifest, String> {
    let manifest = read_json::<VaultSnapshotManifest>(
        &normalize_path(snapshot_path)?.join(SNAPSHOT_MANIFEST),
    )?;
    if manifest.schema_version != 1 {
        return Err(format!(
            "Unsupported snapshot schema version {}.",
            manifest.schema_version
        ));
    }
    Ok(manifest)
}

// copy_file_verified is in utils.rs

// Utility functions are now in utils.rs; remaining domain helpers stay here.
// (markdown_title, entity_type_for_path, unique_trash_path, unique_archive_path).

fn markdown_title(markdown_body: &str, markdown_relative_path: &str) -> String {
    markdown_body
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("# ")
                .map(|title| title.trim().to_string())
        })
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| {
            Path::new(markdown_relative_path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Untitled")
                .replace('-', " ")
        })
}

fn entity_type_for_path(path: &str) -> String {
    let normalized = normalize_relative(path);
    if normalized.starts_with("modules/notes/") || normalized.starts_with("notes/") {
        "note".to_string()
    } else if normalized.starts_with("modules/todos/") || normalized == "modules/todos.md" {
        "todos".to_string()
    } else if normalized.starts_with("modules/contacts/") || normalized == "modules/contacts.md" {
        "contact".to_string()
    } else if normalized.starts_with("modules/habits/") || normalized == "modules/habits.md" {
        "habit".to_string()
    } else if normalized.starts_with("modules/navigator/") {
        "navigator".to_string()
    } else {
        "markdown_document".to_string()
    }
}

fn unique_trash_path(vault_path: &Path, relative_path: &str) -> Result<String, String> {
    let mut occupied = existing_vault_paths(vault_path)?;
    let original = normalize_relative(relative_path);
    Ok(unique_relative_path(
        &format!("{TRASH_FOLDER}/files/{original}"),
        &mut occupied,
    ))
}

fn unique_archive_path(vault_path: &Path, relative_path: &str) -> Result<String, String> {
    let mut occupied = existing_vault_paths(vault_path)?;
    let original = normalize_relative(relative_path);
    Ok(unique_relative_path(
        &format!("{ARCHIVE_FOLDER}/files/{original}"),
        &mut occupied,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-core-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path
    }

    #[test]
    fn parses_frontmatter_and_identity() {
        let markdown = "---\nbentolife_metadata: .bentolifelayout/documents/bl_doc_test.json\n---\n\n# Hello\n\n[[Contact:Alex]]\n\n<!-- bentolife:document_id=bl_doc_test -->\n";
        let parsed = parse_frontmatter(markdown);
        let identity = find_identity_comment(markdown).expect("identity");
        let links = extract_graph_links(&parsed.body, "notes/hello.md");

        assert_eq!(
            parsed.metadata_reference.as_deref(),
            Some(".bentolifelayout/documents/bl_doc_test.json")
        );
        assert_eq!(identity.document_id, "bl_doc_test");
        assert_eq!(links[0].link_type, "contact");
        assert_eq!(links[0].target, "Alex");
    }

    #[test]
    fn navigator_rebuild_is_idempotent_and_skips_system_markdown_warnings() {
        let vault = unique_temp_path("navigator-clean").join(".bentolifevault");
        domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault).expect("scaffold");
        domain::notes::NotesService::create_note(
            &vault,
            "Daily",
            Some("# Daily\n\nLink to nothing.\n".to_string()),
        )
        .expect("note");

        let first = rebuild_navigator(&vault).expect("first rebuild");
        let second = rebuild_navigator(&vault).expect("second rebuild");

        assert_eq!(
            first.markdown.matches("navigator-module-summary").count(),
            second.markdown.matches("navigator-module-summary").count()
        );
        assert!(second.managed_block_warnings.is_empty());
        assert!(!second.health_warnings.iter().any(|warning| {
            warning
                .path
                .as_deref()
                .is_some_and(|path| path == NAVIGATOR_INDEX_PATH || path == NAVIGATOR_DOCUMENT_PATH)
        }));
        assert!(!second
            .health_warnings
            .iter()
            .any(|warning| warning.message.contains("BentoLife Graph")));

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn navigator_rebuild_repairs_unclosed_managed_block() {
        let vault = unique_temp_path("navigator-broken").join(".bentolifevault");
        domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault).expect("scaffold");
        fs::create_dir_all(vault.join("modules/navigator")).expect("navigator folder");
        fs::write(
            vault.join(NAVIGATOR_DOCUMENT_PATH),
            "# Navigator\n\n<!-- bentolife:managed-block start name=\"navigator-backlinks\" -->\nold\n",
        )
        .expect("broken navigator");

        let snapshot = rebuild_navigator(&vault).expect("rebuild");

        assert!(snapshot
            .managed_block_warnings
            .iter()
            .any(|warning| warning.contains("unclosed managed block")));
        assert_eq!(
            snapshot
                .markdown
                .matches("managed-block start name=\"navigator-backlinks\"")
                .count(),
            1
        );
        assert_eq!(
            snapshot
                .markdown
                .matches("managed-block end name=\"navigator-backlinks\"")
                .count(),
            1
        );

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn import_copies_source_and_ignores_obsidian_config() {
        let source = unique_temp_path("obsidian-source");
        let vault = unique_temp_path("import-target").join(".bentolifevault");
        fs::create_dir_all(source.join(".obsidian")).expect("obsidian folder");
        fs::write(source.join(".obsidian/config.json"), "{}").expect("config");
        fs::write(source.join("Daily.md"), "# Daily\n\n[[Project]]\n").expect("markdown");

        let manifest = apply_folder_import(&source, &vault).expect("import");

        assert_eq!(manifest.files.iter().filter(|file| file.copied).count(), 1);
        assert!(vault
            .join(".bentolifelayout/imports/staged/obsidian/Daily.md")
            .is_file());
        assert!(source.join("Daily.md").is_file());
        assert!(!vault
            .join(".bentolifelayout/imports/staged/obsidian/.obsidian/config.json")
            .exists());
        let cache = rebuild_cache_from_vault(&vault).expect("cache");
        assert!(cache.entities_by_path.is_empty());
        let staged = list_staged_imports(&vault).expect("staged imports");
        assert_eq!(staged.records.len(), 1);
        assert_eq!(staged.records[0].suggested_module, "notes");

        let _ = fs::remove_dir_all(source);
        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn importing_bentolife_vault_reviews_only_module_data_markdown() {
        let source = unique_temp_path("source-vault").join(".bentolifevault");
        let vault = unique_temp_path("import-target-vault").join(".bentolifevault");
        fs::create_dir_all(source.join(".bentolifelayout/layouts")).expect("layout folder");
        fs::create_dir_all(source.join("modules/notes/data")).expect("notes data");
        fs::create_dir_all(source.join("modules/navigator")).expect("navigator");
        fs::create_dir_all(source.join("schemas/modules")).expect("schemas");
        fs::write(source.join("INDEX.md"), "# System Index\n").expect("root index");
        fs::write(source.join(".bentolifelayout/layouts/daily.json"), "{}").expect("layout");
        fs::write(source.join("modules/notes/INDEX.md"), "# Notes\n").expect("module index");
        fs::write(source.join("modules/notes/MODULE.md"), "# Notes Module\n").expect("module doc");
        fs::write(source.join("modules/notes/data/daily.md"), "# Daily\n").expect("user note");
        fs::write(
            source.join("modules/navigator/NAVIGATOR.md"),
            "# Navigator\n",
        )
        .expect("navigator doc");
        fs::write(source.join("schemas/modules/note.json"), "{}").expect("schema");

        let preview = plan_folder_import(&source, &vault).expect("preview");
        assert_eq!(preview.scan.source_kind, "bentolife_vault");
        assert_eq!(preview.scan.markdown_count, 1);
        assert!(preview.scan.ignored_count >= 5);

        let manifest = apply_folder_import(&source, &vault).expect("import");
        assert_eq!(manifest.files.iter().filter(|file| file.copied).count(), 1);

        let staged = list_staged_imports(&vault).expect("staged");
        assert_eq!(staged.records.len(), 1);
        assert_eq!(
            staged.records[0].original_source_path.as_deref(),
            Some("modules/notes/data/daily.md")
        );
        assert_eq!(staged.records[0].source_kind, "bentolife_vault");
        assert!(staged.hidden_system_count >= 5);
        assert!(!staged.records.iter().any(|record| {
            let source = record.original_source_path.as_deref().unwrap_or("");
            source.contains(".bentolifelayout")
                || source.ends_with("INDEX.md")
                || source.ends_with("MODULE.md")
        }));

        let _ = fs::remove_dir_all(source.parent().expect("source parent"));
        let _ = fs::remove_dir_all(vault.parent().expect("vault parent"));
    }

    #[test]
    fn staged_review_hides_assets_and_refuses_hidden_acceptance() {
        let vault = unique_temp_path("staged-review-filter").join(".bentolifevault");
        fs::create_dir_all(vault.join(".bentolifelayout/imports/staged/folders/assets"))
            .expect("assets folder");
        fs::create_dir_all(vault.join(".bentolifelayout/imports/staged/folders/modules/trash"))
            .expect("trash folder");
        fs::write(
            vault.join(".bentolifelayout/imports/staged/folders/Daily.md"),
            "# Daily\n",
        )
        .expect("daily");
        fs::write(
            vault.join(".bentolifelayout/imports/staged/folders/assets/banner.png"),
            "png",
        )
        .expect("asset");
        fs::write(
            vault.join(".bentolifelayout/imports/staged/folders/modules/trash/INDEX.md"),
            "# Trash\n",
        )
        .expect("trash index");

        let staged = list_staged_imports(&vault).expect("staged");

        assert_eq!(staged.records.len(), 1);
        assert_eq!(
            staged.records[0].staged_file_path,
            ".bentolifelayout/imports/staged/folders/Daily.md"
        );
        assert_eq!(staged.hidden_system_count, 2);
        assert!(accept_import_into_module(
            &vault,
            ".bentolifelayout/imports/staged/folders/modules/trash/INDEX.md",
            "notes",
            ImportAcceptanceOptions {
                target_filename: None,
                tags: Vec::new(),
                preserve_source_path: true,
                batch_tag: None,
            },
        )
        .is_err());

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn import_suggestion_prefers_source_path_then_content_signals() {
        assert_eq!(
            suggest_module_for_import(
                Some("modules/todos/data/task.md"),
                "# Plain\n\nEmail: someone@example.com\n",
                0,
            ),
            "todos"
        );
        assert_eq!(
            suggest_module_for_import(None, "# Task\n\nStatus: Open\nPriority: High\n", 0),
            "todos"
        );
        assert_eq!(
            suggest_module_for_import(
                None,
                "# Contact\n\nEmail: mina@example.com\nRelationship: Client\n",
                0
            ),
            "contacts"
        );
        assert_eq!(
            suggest_module_for_import(None, "# Habit\n\nFrequency: Daily\nStreak: 3\n", 0),
            "habits"
        );
        assert_eq!(
            suggest_module_for_import(None, "# Journal\n\nAmbiguous body.\n", 0),
            "notes"
        );
    }

    #[test]
    fn import_renames_target_collisions() {
        let source = unique_temp_path("collision-source");
        let vault = unique_temp_path("collision-target").join(".bentolifevault");
        fs::create_dir_all(&source).expect("source");
        fs::create_dir_all(vault.join(".bentolifelayout/imports/staged/folders")).expect("vault");
        fs::write(source.join("Daily.md"), "# Daily\n").expect("source file");
        fs::write(
            vault.join(".bentolifelayout/imports/staged/folders/Daily.md"),
            "# Existing\n",
        )
        .expect("existing");

        let manifest = apply_folder_import(&source, &vault).expect("import");

        assert!(manifest.files[0].collision_renamed);
        assert!(vault
            .join(".bentolifelayout/imports/staged/folders/Daily-2.md")
            .is_file());

        let _ = fs::remove_dir_all(source);
        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn accept_staged_import_writes_managed_module_markdown() {
        let source = unique_temp_path("accept-source");
        let vault = unique_temp_path("accept-target").join(".bentolifevault");
        fs::create_dir_all(&source).expect("source");
        fs::write(source.join("Imported.md"), "# Imported\n\nPortable body.\n")
            .expect("source file");
        apply_folder_import(&source, &vault).expect("import");
        let staged = list_staged_imports(&vault).expect("staged");
        let staged_file = staged.records[0].staged_file_path.clone();

        let report = accept_import_into_module(
            &vault,
            &staged_file,
            "notes",
            ImportAcceptanceOptions {
                target_filename: None,
                tags: vec!["imported".to_string()],
                preserve_source_path: true,
                batch_tag: Some("imported".to_string()),
            },
        )
        .expect("accept");

        assert!(report
            .accepted_relative_path
            .starts_with("modules/notes/data/"));
        let accepted =
            fs::read_to_string(vault.join(&report.accepted_relative_path)).expect("accepted");
        assert!(accepted.contains("Portable body."));
        assert!(accepted.contains("bentolife_metadata"));
        assert!(accepted.contains("bentolife:document_id="));
        assert_eq!(accepted.matches("#imported").count(), 1);
        let staged = list_staged_imports(&vault).expect("updated staged");
        assert!(staged.records[0].accepted);
        assert!(vault.join(&report.accepted_manifest_path).is_file());

        let _ = fs::remove_dir_all(source);
        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn accept_staged_import_supports_all_content_modules() {
        let vault = unique_temp_path("accept-all-target").join(".bentolifevault");
        for module in ["notes", "todos", "contacts", "habits"] {
            let source = unique_temp_path(&format!("accept-{module}-source"));
            fs::create_dir_all(&source).expect("source");
            fs::write(
                source.join(format!("{module}.md")),
                format!("# {module}\n\nBody.\n"),
            )
            .expect("source file");
            apply_folder_import(&source, &vault).expect("import");
            let staged = list_staged_imports(&vault).expect("staged");
            let staged_file = staged
                .records
                .iter()
                .find(|record| !record.accepted && !record.ignored)
                .expect("unaccepted record")
                .staged_file_path
                .clone();

            let report = accept_import_into_module(
                &vault,
                &staged_file,
                module,
                ImportAcceptanceOptions {
                    target_filename: None,
                    tags: vec!["imported".to_string()],
                    preserve_source_path: true,
                    batch_tag: None,
                },
            )
            .expect("accept");

            assert!(report
                .accepted_relative_path
                .starts_with(&format!("modules/{module}/data/")));
            let _ = fs::remove_dir_all(source);
        }

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn snapshot_restore_copies_to_new_vault_and_rebuilds_cache() {
        let old_vault = unique_temp_path("old-vault").join(".bentolifevault");
        let snapshot = unique_temp_path("snapshot");
        let new_vault = unique_temp_path("new-vault").join(".bentolifevault");
        fs::create_dir_all(old_vault.join("modules/notes/data")).expect("old vault");
        fs::write(
            old_vault.join("modules/notes/data/daily.md"),
            "# Daily\n\n<!-- bentolife:document_id=bl_doc_daily -->\n",
        )
        .expect("old note");

        let manifest = create_vault_snapshot(&old_vault, &snapshot).expect("snapshot");
        let report = restore_vault_snapshot(&snapshot, &new_vault).expect("restore");

        assert_eq!(manifest.files.len(), 1);
        assert!(new_vault.join("modules/notes/data/daily.md").is_file());
        assert!(report
            .cache
            .entities_by_path
            .contains_key("modules/notes/data/daily.md"));
        assert!(new_vault.join(INDEX_PATH).is_file());

        let _ = fs::remove_dir_all(old_vault.parent().expect("parent"));
        let _ = fs::remove_dir_all(snapshot);
        let _ = fs::remove_dir_all(new_vault.parent().expect("parent"));
    }

    #[test]
    fn snapshot_restore_preserves_pasted_note_image_assets() {
        let source_vault = unique_temp_path("asset-snapshot-source").join(".bentolifevault");
        let snapshot = unique_temp_path("asset-snapshot");
        let target_vault = unique_temp_path("asset-snapshot-target").join(".bentolifevault");
        let note = domain::notes::NotesService::create_note(&source_vault, "Image Note", None)
            .expect("note");
        let asset = domain::markdown_assets::MarkdownAssetService::save_markdown_asset(
            &source_vault,
            "notes",
            &note.document_id,
            Some("paste.png".to_string()),
            "image/png",
            vec![137, 80, 78, 71],
        )
        .expect("asset");
        let updated = domain::notes::NotesService::update_note(
            &source_vault,
            &note.document_id,
            format!("# Image Note\n\n![Pasted image]({})\n", asset.markdown_link),
            Some(note.parsed_entity.content_hash),
            false,
        )
        .expect("update note");

        let manifest = create_vault_snapshot(&source_vault, &snapshot).expect("snapshot");
        assert!(manifest
            .files
            .iter()
            .any(|file| file.relative_path == asset.vault_relative_path));
        let preview = preview_snapshot_restore(&snapshot, &target_vault).expect("preview");
        assert_eq!(preview.snapshot_shape, SnapshotVaultShape::V3ActiveSnapshot);

        restore_vault_snapshot(&snapshot, &target_vault).expect("restore");
        assert!(target_vault.join(&asset.vault_relative_path).is_file());
        let restored_note =
            domain::notes::NotesService::read_note(&target_vault, &updated.document_id)
                .expect("restored note");
        assert!(restored_note.markdown_body.contains(&asset.markdown_link));
        let read = domain::markdown_assets::MarkdownAssetService::read_markdown_asset(
            &target_vault,
            "notes",
            &updated.document_id,
            &asset.markdown_link,
        )
        .expect("read restored asset");
        assert_eq!(read.bytes, vec![137, 80, 78, 71]);

        let _ = fs::remove_dir_all(source_vault.parent().expect("parent"));
        let _ = fs::remove_dir_all(snapshot);
        let _ = fs::remove_dir_all(target_vault.parent().expect("parent"));
    }

    #[test]
    fn snapshot_export_omits_stale_legacy_markdown_paths() {
        let vault = unique_temp_path("snapshot-clean-export").join(".bentolifevault");
        let snapshot = unique_temp_path("snapshot-clean-export-target");
        let restore_target =
            unique_temp_path("snapshot-clean-export-restore").join(".bentolifevault");
        domain::notes::NotesService::create_note(&vault, "Current", None).expect("note");
        fs::create_dir_all(vault.join("notes")).expect("legacy notes");
        fs::create_dir_all(vault.join("modules/notes")).expect("legacy modules notes");
        fs::write(vault.join("notes/Legacy.md"), "# Legacy\n").expect("legacy note");
        fs::write(vault.join("modules/todos.md"), "# Legacy Todos\n").expect("legacy todos");
        fs::write(vault.join("modules/contacts.md"), "# Legacy Contacts\n")
            .expect("legacy contacts");
        fs::write(vault.join("modules/habits.md"), "# Legacy Habits\n").expect("legacy habits");
        fs::write(vault.join("modules/notes/Loose.md"), "# Loose\n").expect("loose note");
        fs::write(vault.join("modules/notes/INDEX.md"), "# Notes\n").expect("module index");

        let manifest = create_vault_snapshot(&vault, &snapshot).expect("snapshot");
        let exported_paths = manifest
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>();

        assert!(exported_paths
            .iter()
            .any(|path| path.starts_with("modules/notes/data/")));
        assert!(exported_paths.contains(&"modules/notes/INDEX.md"));
        for legacy_path in [
            "notes/Legacy.md",
            "modules/todos.md",
            "modules/contacts.md",
            "modules/habits.md",
            "modules/notes/Loose.md",
        ] {
            assert!(!exported_paths.contains(&legacy_path), "{legacy_path}");
        }
        let preview = preview_snapshot_restore(&snapshot, &restore_target).expect("preview");
        assert_eq!(preview.snapshot_shape, SnapshotVaultShape::V3ActiveSnapshot);
        assert_eq!(preview.legacy_file_count, 0);
        assert!(preview.legacy_file_paths.is_empty());
        assert_eq!(preview.active_v3_file_count, 1);
        restore_vault_snapshot(&snapshot, &restore_target).expect("restore");

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
        let _ = fs::remove_dir_all(snapshot);
        let _ = fs::remove_dir_all(restore_target.parent().expect("parent"));
    }

    #[test]
    fn staged_snapshot_accept_restores_dependent_pasted_assets() {
        let snapshot = unique_temp_path("staged-asset-snapshot");
        let target_vault = unique_temp_path("staged-asset-target").join(".bentolifevault");
        domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&target_vault)
            .expect("target scaffold");
        let data = snapshot.join("data");
        fs::create_dir_all(data.join("modules/notes/data")).expect("note data");
        fs::create_dir_all(data.join("assets/notes/bl_doc_asset_note")).expect("asset data");
        fs::write(
            data.join("modules/notes/data/Image.md"),
            "# Image\n\n![Pasted image](../../../assets/notes/bl_doc_asset_note/paste.png)\n\n<!-- bentolife:document_id=bl_doc_asset_note -->\n",
        )
        .expect("markdown");
        fs::write(
            data.join("assets/notes/bl_doc_asset_note/paste.png"),
            vec![137, 80, 78, 71],
        )
        .expect("asset");
        let markdown_bytes =
            fs::read(data.join("modules/notes/data/Image.md")).expect("markdown bytes");
        let asset_bytes =
            fs::read(data.join("assets/notes/bl_doc_asset_note/paste.png")).expect("asset bytes");
        let manifest = VaultSnapshotManifest {
            schema_version: 1,
            source_vault_path: "snapshot".to_string(),
            snapshot_path: snapshot.to_string_lossy().to_string(),
            created_at: current_timestamp_label(),
            source_machine: "test".to_string(),
            files: vec![
                SnapshotFileEntry {
                    relative_path: "modules/notes/data/Image.md".to_string(),
                    file_kind: "markdown".to_string(),
                    content_hash: content_hash_bytes(&markdown_bytes),
                    byte_len: markdown_bytes.len() as u64,
                },
                SnapshotFileEntry {
                    relative_path: "assets/notes/bl_doc_asset_note/paste.png".to_string(),
                    file_kind: "asset".to_string(),
                    content_hash: content_hash_bytes(&asset_bytes),
                    byte_len: asset_bytes.len() as u64,
                },
            ],
            warnings: Vec::new(),
        };
        write_json_atomic(&snapshot.join(SNAPSHOT_MANIFEST), &manifest).expect("manifest");

        let staged = stage_snapshot_for_import(&snapshot, &target_vault).expect("stage");
        assert_eq!(staged.staged_files.len(), 1);
        assert!(staged
            .index
            .hidden_system_files
            .iter()
            .any(|path| path.ends_with("assets/notes/bl_doc_asset_note/paste.png")));
        let report = accept_import_into_module(
            &target_vault,
            &staged.staged_files[0].staged_file_path,
            "notes",
            ImportAcceptanceOptions {
                target_filename: Some("restored-image".to_string()),
                tags: Vec::new(),
                preserve_source_path: true,
                batch_tag: None,
            },
        )
        .expect("accept");

        assert_eq!(
            report.accepted_relative_path,
            "modules/notes/data/restored-image.md"
        );
        assert!(target_vault
            .join("assets/notes/bl_doc_asset_note/paste.png")
            .is_file());
        let read = domain::markdown_assets::MarkdownAssetService::read_markdown_asset(
            &target_vault,
            "notes",
            "bl_doc_asset_note",
            "../../../assets/notes/bl_doc_asset_note/paste.png",
        )
        .expect("read asset");
        assert_eq!(read.bytes, vec![137, 80, 78, 71]);

        let _ = fs::remove_dir_all(snapshot);
        let _ = fs::remove_dir_all(target_vault.parent().expect("parent"));
    }

    #[test]
    fn old_snapshot_restore_preview_blocks_direct_restore_and_stages_current_data_only() {
        let snapshot = unique_temp_path("legacy-snapshot");
        let target_vault = unique_temp_path("legacy-stage-target").join(".bentolifevault");
        let data = snapshot.join("data");
        fs::create_dir_all(data.join("notes")).expect("snapshot data");
        fs::create_dir_all(data.join("modules/notes/data")).expect("current data");
        fs::write(
            data.join("notes/Legacy.md"),
            "# Legacy\n\n- [ ] Review\n- [ ] Accept\n",
        )
        .expect("legacy");
        fs::write(data.join("modules/notes/data/Current.md"), "# Current\n").expect("current");
        let bytes = fs::read(data.join("notes/Legacy.md")).expect("bytes");
        let current_bytes =
            fs::read(data.join("modules/notes/data/Current.md")).expect("current bytes");
        let manifest = VaultSnapshotManifest {
            schema_version: 1,
            source_vault_path: "legacy".to_string(),
            snapshot_path: snapshot.to_string_lossy().to_string(),
            created_at: current_timestamp_label(),
            source_machine: "test".to_string(),
            files: vec![
                SnapshotFileEntry {
                    relative_path: "notes/Legacy.md".to_string(),
                    file_kind: "markdown".to_string(),
                    content_hash: content_hash_bytes(&bytes),
                    byte_len: bytes.len() as u64,
                },
                SnapshotFileEntry {
                    relative_path: "modules/notes/data/Current.md".to_string(),
                    file_kind: "markdown".to_string(),
                    content_hash: content_hash_bytes(&current_bytes),
                    byte_len: current_bytes.len() as u64,
                },
                SnapshotFileEntry {
                    relative_path: ".bentolifelayout/index.json".to_string(),
                    file_kind: "asset".to_string(),
                    content_hash: "metadata".to_string(),
                    byte_len: 2,
                },
            ],
            warnings: Vec::new(),
        };
        write_json_atomic(&snapshot.join(SNAPSHOT_MANIFEST), &manifest).expect("manifest");
        fs::create_dir_all(data.join(".bentolifelayout")).expect("layout data");
        fs::write(data.join(".bentolifelayout/index.json"), "{}").expect("layout index");

        let preview = preview_snapshot_restore(&snapshot, &target_vault).expect("preview");
        assert_eq!(
            preview.snapshot_shape,
            SnapshotVaultShape::MixedOrUnknownSnapshot
        );
        assert!(!preview.direct_restore_allowed);
        assert_eq!(
            preview.legacy_file_paths,
            vec!["notes/Legacy.md".to_string()]
        );
        assert_eq!(
            preview.active_v3_file_paths,
            vec!["modules/notes/data/Current.md".to_string()]
        );
        assert_eq!(
            preview.hidden_runtime_file_paths,
            vec![".bentolifelayout/index.json".to_string()]
        );
        assert!(restore_vault_snapshot(&snapshot, &target_vault).is_err());

        let staged = stage_snapshot_for_import(&snapshot, &target_vault).expect("stage");
        assert_eq!(staged.staged_files.len(), 1);
        assert_eq!(
            staged.staged_files[0].original_source_path.as_deref(),
            Some("modules/notes/data/Current.md")
        );
        assert_eq!(staged.index.hidden_system_count, 2);
        assert!(target_vault
            .join(".bentolifelayout/imports/staged/snapshots")
            .is_dir());
        assert_eq!(staged.index.records[0].suggested_module, "notes");

        let _ = fs::remove_dir_all(snapshot);
        let _ = fs::remove_dir_all(target_vault.parent().expect("parent"));
    }

    #[test]
    fn failed_direct_restore_rolls_back_copied_files() {
        let snapshot = unique_temp_path("broken-v3-snapshot");
        let target_vault = unique_temp_path("broken-v3-target").join(".bentolifevault");
        let data = snapshot.join("data/modules/notes/data");
        fs::create_dir_all(&data).expect("snapshot data");
        fs::write(data.join("Good.md"), "# Good\n").expect("good");
        let good_bytes = fs::read(data.join("Good.md")).expect("bytes");
        let manifest = VaultSnapshotManifest {
            schema_version: 1,
            source_vault_path: "v3".to_string(),
            snapshot_path: snapshot.to_string_lossy().to_string(),
            created_at: current_timestamp_label(),
            source_machine: "test".to_string(),
            files: vec![
                SnapshotFileEntry {
                    relative_path: "modules/notes/data/Good.md".to_string(),
                    file_kind: "markdown".to_string(),
                    content_hash: content_hash_bytes(&good_bytes),
                    byte_len: good_bytes.len() as u64,
                },
                SnapshotFileEntry {
                    relative_path: "modules/notes/data/Missing.md".to_string(),
                    file_kind: "markdown".to_string(),
                    content_hash: "missing".to_string(),
                    byte_len: 1,
                },
            ],
            warnings: Vec::new(),
        };
        write_json_atomic(&snapshot.join(SNAPSHOT_MANIFEST), &manifest).expect("manifest");

        let error = restore_vault_snapshot(&snapshot, &target_vault).expect_err("restore fails");
        assert!(error.contains("Unable to copy"));
        assert!(!target_vault.join("modules/notes/data/Good.md").exists());

        let _ = fs::remove_dir_all(snapshot);
        let _ = fs::remove_dir_all(target_vault.parent().expect("parent"));
    }

    #[test]
    fn snapshot_restore_rejects_manifest_traversal_paths() {
        let snapshot = unique_temp_path("unsafe-snapshot");
        let target_vault = unique_temp_path("unsafe-snapshot-target").join(".bentolifevault");
        let data = snapshot.join("data");
        fs::create_dir_all(&data).expect("snapshot data");
        let manifest = VaultSnapshotManifest {
            schema_version: 1,
            source_vault_path: "v3".to_string(),
            snapshot_path: snapshot.to_string_lossy().to_string(),
            created_at: current_timestamp_label(),
            source_machine: "test".to_string(),
            files: vec![SnapshotFileEntry {
                relative_path: "../outside.md".to_string(),
                file_kind: "markdown".to_string(),
                content_hash: "unsafe".to_string(),
                byte_len: 1,
            }],
            warnings: Vec::new(),
        };
        write_json_atomic(&snapshot.join(SNAPSHOT_MANIFEST), &manifest).expect("manifest");

        assert!(preview_snapshot_restore(&snapshot, &target_vault).is_err());
        assert!(restore_vault_snapshot(&snapshot, &target_vault).is_err());
        assert!(stage_snapshot_for_import(&snapshot, &target_vault).is_err());

        let _ = fs::remove_dir_all(snapshot);
        let _ = fs::remove_dir_all(target_vault.parent().expect("parent"));
    }

    #[test]
    fn trash_and_restore_moves_managed_file() {
        let vault = unique_temp_path("trash").join(".bentolifevault");
        fs::create_dir_all(vault.join("modules/notes/data")).expect("vault");
        fs::write(vault.join("modules/notes/data/daily.md"), "# Daily\n").expect("note");

        let trashed = trash_managed_entity(&vault, "modules/notes/data/daily.md").expect("trash");
        assert!(!vault.join("modules/notes/data/daily.md").exists());
        assert!(vault.join(&trashed.entry.trash_relative_path).is_file());

        let restored = restore_trashed_entity(
            &vault,
            &trashed.entry.trash_relative_path,
            "modules/notes/data/daily.md",
        )
        .expect("restore");
        assert_eq!(restored.action, "restore_trashed_entity");
        assert!(vault.join("modules/notes/data/daily.md").is_file());

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn archive_and_restore_excludes_archived_file_from_cache() {
        let vault = unique_temp_path("archive").join(".bentolifevault");
        fs::create_dir_all(vault.join("modules/notes/data")).expect("vault");
        fs::write(vault.join("modules/notes/data/daily.md"), "# Daily\n").expect("note");

        let archived =
            archive_managed_entity(&vault, "modules/notes/data/daily.md").expect("archive");
        assert!(!vault.join("modules/notes/data/daily.md").exists());
        assert!(vault.join(&archived.entry.archive_relative_path).is_file());
        assert!(archived.cache.entities_by_path.is_empty());

        let restored = restore_archived_entity(
            &vault,
            &archived.entry.archive_relative_path,
            "modules/notes/data/daily.md",
        )
        .expect("restore");
        assert_eq!(restored.action, "restore_archived_entity");
        assert!(vault.join("modules/notes/data/daily.md").is_file());

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn lifecycle_lists_trash_and_archive_entries() {
        let vault = unique_temp_path("lifecycle-list").join(".bentolifevault");
        fs::create_dir_all(vault.join("modules/notes/data")).expect("vault");
        fs::write(vault.join("modules/notes/data/daily.md"), "# Daily\n").expect("note");
        fs::write(vault.join("modules/notes/data/archive.md"), "# Archive\n").expect("note");

        let trashed = trash_managed_entity(&vault, "modules/notes/data/daily.md").expect("trash");
        let archived =
            archive_managed_entity(&vault, "modules/notes/data/archive.md").expect("archive");
        let trash_entries = list_trash_entries(&vault).expect("trash entries");
        let archive_entries = list_archive_entries(&vault).expect("archive entries");

        assert_eq!(trash_entries.len(), 1);
        assert_eq!(
            trash_entries[0].original_path,
            "modules/notes/data/daily.md"
        );
        assert_eq!(
            trash_entries[0].current_path,
            trashed.entry.trash_relative_path
        );
        assert_eq!(trash_entries[0].file_name, "daily.md");
        assert_eq!(trash_entries[0].module_id.as_deref(), Some("notes"));
        assert!(trash_entries[0].can_restore);

        assert_eq!(archive_entries.len(), 1);
        assert_eq!(
            archive_entries[0].original_path,
            "modules/notes/data/archive.md"
        );
        assert_eq!(
            archive_entries[0].current_path,
            archived.entry.archive_relative_path
        );
        assert!(archive_entries[0].can_restore);

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn restore_by_entry_renames_when_original_path_is_occupied() {
        let vault = unique_temp_path("lifecycle-restore").join(".bentolifevault");
        fs::create_dir_all(vault.join("modules/notes/data")).expect("vault");
        fs::write(vault.join("modules/notes/data/daily.md"), "# Daily\n").expect("note");

        trash_managed_entity(&vault, "modules/notes/data/daily.md").expect("trash");
        fs::write(vault.join("modules/notes/data/daily.md"), "# Replacement\n")
            .expect("replacement");
        let entry_id = list_trash_entries(&vault).expect("trash entries")[0]
            .id
            .clone();
        let restored = restore_trash_entry(&vault, &entry_id).expect("restore by entry");

        assert_eq!(
            restored.entry.original_relative_path,
            "modules/notes/data/daily-2.md"
        );
        assert!(vault.join("modules/notes/data/daily.md").is_file());
        assert!(vault.join("modules/notes/data/daily-2.md").is_file());
        assert!(list_trash_entries(&vault)
            .expect("trash entries")
            .is_empty());

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn permanent_delete_and_empty_trash_remove_internal_records() {
        let vault = unique_temp_path("lifecycle-delete").join(".bentolifevault");
        fs::create_dir_all(vault.join("modules/notes/data")).expect("vault");
        fs::write(vault.join("modules/notes/data/one.md"), "# One\n").expect("note");
        fs::write(vault.join("modules/notes/data/two.md"), "# Two\n").expect("note");

        trash_managed_entity(&vault, "modules/notes/data/one.md").expect("trash");
        trash_managed_entity(&vault, "modules/notes/data/two.md").expect("trash");
        let entries = list_trash_entries(&vault).expect("trash entries");
        assert_eq!(entries.len(), 2);

        let remaining = delete_trash_entry_permanently(&vault, &entries[0].id).expect("delete");
        assert_eq!(remaining.len(), 1);
        let emptied = empty_trash(&vault).expect("empty");
        assert!(emptied.is_empty());
        assert!(list_trash_entries(&vault)
            .expect("trash entries")
            .is_empty());

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }

    #[test]
    fn v3_active_operations_reject_older_vault_targets() {
        let vault = unique_temp_path("legacy-target").join(".bentolifevault");
        fs::create_dir_all(vault.join("notes")).expect("legacy vault");
        fs::write(vault.join("notes/daily.md"), "# Daily\n").expect("legacy note");

        let error = rebuild_cache_from_vault(&vault).expect_err("legacy rejected");

        assert!(error.contains("Older BentoLife vault structure detected"));

        let _ = fs::remove_dir_all(vault.parent().expect("parent"));
    }
}
