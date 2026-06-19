pub(super) use std::path::PathBuf;

pub(super) use bentolife_core::{
    accept_import_into_module as core_accept_import_into_module,
    apply_entity_upgrade as core_apply_entity_upgrade, apply_folder_import,
    archive_managed_entity as core_archive_managed_entity,
    bulk_accept_imports as core_bulk_accept_imports,
    create_vault_snapshot as core_create_vault_snapshot,
    delete_trash_entry_permanently as core_delete_trash_entry_permanently,
    empty_trash as core_empty_trash, ignore_staged_import as core_ignore_staged_import,
    list_archive_entries as core_list_archive_entries,
    list_staged_imports as core_list_staged_imports, list_trash_entries as core_list_trash_entries,
    preview_accept_import as core_preview_accept_import,
    preview_entity_upgrade as core_preview_entity_upgrade,
    preview_snapshot_restore as core_preview_snapshot_restore,
    preview_vault_snapshot as core_preview_vault_snapshot, read_navigator as core_read_navigator,
    rebuild_cache_from_vault, rebuild_navigator as core_rebuild_navigator,
    rebuild_search_index as core_rebuild_search_index,
    restore_archive_entry as core_restore_archive_entry,
    restore_archived_entity as core_restore_archived_entity,
    restore_trash_entry as core_restore_trash_entry,
    restore_trashed_entity as core_restore_trashed_entity,
    restore_vault_snapshot as core_restore_vault_snapshot,
    scan_and_rebuild_navigator as core_scan_and_rebuild_navigator,
    search_entities as core_search_entities,
    stage_snapshot_for_import as core_stage_snapshot_for_import,
    trash_managed_entity as core_trash_managed_entity, ArchiveResult, BulkImportAcceptReport,
    CoreCacheSnapshot, EntityUpgradePreview, EntityUpgradeReport, FileLifecycleEntry,
    FileLifecycleMutationReport, FolderImportManifest, FolderImportPreview,
    IgnoredStagedImportReport, ImportAcceptPreview, ImportAcceptReport, ImportAcceptanceOptions,
    NavigatorRebuildReport, NavigatorSnapshot, SearchIndexSnapshot, SnapshotRestorePreview,
    SnapshotRestoreReport, SnapshotStageReport, StagedImportIndex, TrashResult,
    VaultSnapshotManifest, VaultSnapshotPreview,
};

pub(super) use bentolife_core::domain::{
    contacts::{ContactDocument, ContactInput, ContactsService},
    dashboard::{DashboardHubDocument, DashboardService},
    dashboard_widgets::{
        DashboardWidgetCreateRequest, DashboardWidgetLayout, DashboardWidgetService,
        DashboardWidgetState, DashboardWidgetUpdateRequest,
    },
    document_metadata::DocumentMetadataService,
    habits::{HabitDocument, HabitInput, HabitsService},
    import_export::{ImportExportService, ImportResult, ImportValidation},
    layout_metadata::{LayoutMetadata, LayoutMetadataService},
    markdown_assets::{MarkdownAsset, MarkdownAssetRead, MarkdownAssetService},
    markdown_document::{ManagedMarkdownDocument, MarkdownDocumentService},
    module_registry::{ModuleDefinition, ModuleRegistry, ModuleRegistryService, RegistryState},
    module_schema::{WidgetSizeDefinition, WidgetTypeDefinition},
    notes::{NoteDocument, NoteSummary, NotesService},
    recovery::{RecoveryResult, RecoveryService, WorkspaceRecoveryPreview},
    theme::{ActiveThemeState, ThemePreview, ThemeService},
    todo::{TodoDocument, TodoService, TodoSummary},
    vault::{VaultInspection, VaultService},
    workspace_metadata::{WorkspaceIndex, WorkspaceMetadataService, WorkspaceState},
    workspace_scanner::{WorkspaceScanResult, WorkspaceScanner},
};

pub(super) use crate::backend::adapters::platform_paths;

pub(super) fn checked_vault_path(path: String) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    bentolife_core::domain::security::validate_vault_root_path(&path)
}

pub(super) fn checked_user_markdown_path(relative_path: &str) -> Result<String, String> {
    bentolife_core::domain::security::validate_user_module_markdown_path(relative_path)
}

pub(super) fn checked_recovery_markdown_path(relative_path: &str) -> Result<String, String> {
    bentolife_core::domain::security::validate_recovery_markdown_path(relative_path)
}

pub(super) fn checked_staged_import_path(relative_path: &str) -> Result<String, String> {
    bentolife_core::domain::security::validate_staged_import_path(relative_path)
}

pub(super) fn checked_lifecycle_file_path(
    relative_path: &str,
    folder: &str,
) -> Result<String, String> {
    bentolife_core::domain::security::validate_lifecycle_file_path(relative_path, folder)
}

pub(super) fn checked_module_id(module_id: &str) -> Result<String, String> {
    bentolife_core::domain::security::validate_known_module_id(module_id)
}

pub(super) fn require_confirmation(expected: &str, provided: &str) -> Result<(), String> {
    bentolife_core::domain::security::require_confirmation_token(expected, provided)
}

pub(super) fn trash_token(relative_path: &str) -> Result<String, String> {
    bentolife_core::domain::security::trash_confirmation_token(relative_path)
}

pub(super) fn archive_token(relative_path: &str) -> Result<String, String> {
    bentolife_core::domain::security::archive_confirmation_token(relative_path)
}

pub(super) fn delete_trash_token(entry_id: &str) -> Result<String, String> {
    bentolife_core::domain::security::delete_trash_confirmation_token(entry_id)
}

pub(super) fn empty_trash_token() -> &'static str {
    bentolife_core::domain::security::empty_trash_confirmation_token()
}

pub(super) fn restore_snapshot_token() -> &'static str {
    bentolife_core::domain::security::restore_snapshot_confirmation_token()
}

pub(super) fn apply_entity_upgrade_token() -> &'static str {
    bentolife_core::domain::security::apply_entity_upgrade_confirmation_token()
}

pub(super) fn repair_vault_token() -> &'static str {
    bentolife_core::domain::security::repair_vault_confirmation_token()
}
