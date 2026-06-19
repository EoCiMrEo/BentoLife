use super::shared::*;

#[tauri::command]
pub fn preview_vault_snapshot(
    source_vault_path: String,
    snapshot_path: String,
) -> Result<VaultSnapshotPreview, String> {
    core_preview_vault_snapshot(
        &checked_vault_path(source_vault_path)?,
        &PathBuf::from(snapshot_path),
    )
}

#[tauri::command]
pub fn create_vault_snapshot(
    source_vault_path: String,
    snapshot_path: String,
) -> Result<VaultSnapshotManifest, String> {
    core_create_vault_snapshot(
        &checked_vault_path(source_vault_path)?,
        &PathBuf::from(snapshot_path),
    )
}

#[tauri::command]
pub fn preview_snapshot_restore(
    snapshot_path: String,
    target_vault_path: String,
) -> Result<SnapshotRestorePreview, String> {
    core_preview_snapshot_restore(
        &PathBuf::from(snapshot_path),
        &checked_vault_path(target_vault_path)?,
    )
}

#[tauri::command]
pub fn restore_vault_snapshot(
    snapshot_path: String,
    target_vault_path: String,
    confirmation_token: String,
) -> Result<SnapshotRestoreReport, String> {
    require_confirmation(restore_snapshot_token(), &confirmation_token)?;
    core_restore_vault_snapshot(
        &PathBuf::from(snapshot_path),
        &checked_vault_path(target_vault_path)?,
    )
}

#[tauri::command]
pub fn stage_snapshot_for_import(
    snapshot_path: String,
    target_vault_path: String,
) -> Result<SnapshotStageReport, String> {
    core_stage_snapshot_for_import(
        &PathBuf::from(snapshot_path),
        &checked_vault_path(target_vault_path)?,
    )
}

#[tauri::command]
pub fn list_staged_imports(vault_path: String) -> Result<StagedImportIndex, String> {
    core_list_staged_imports(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn preview_accept_import(
    vault_path: String,
    staged_file_path: String,
    target_module: String,
    options: ImportAcceptanceOptions,
) -> Result<ImportAcceptPreview, String> {
    let staged_file_path = checked_staged_import_path(&staged_file_path)?;
    let target_module = checked_module_id(&target_module)?;
    core_preview_accept_import(
        &checked_vault_path(vault_path)?,
        &staged_file_path,
        &target_module,
        options,
    )
}

#[tauri::command]
pub fn accept_import_into_module(
    vault_path: String,
    staged_file_path: String,
    target_module: String,
    options: ImportAcceptanceOptions,
) -> Result<ImportAcceptReport, String> {
    let staged_file_path = checked_staged_import_path(&staged_file_path)?;
    let target_module = checked_module_id(&target_module)?;
    core_accept_import_into_module(
        &checked_vault_path(vault_path)?,
        &staged_file_path,
        &target_module,
        options,
    )
}

#[tauri::command]
pub fn ignore_staged_import(
    vault_path: String,
    staged_file_path: String,
) -> Result<IgnoredStagedImportReport, String> {
    let staged_file_path = checked_staged_import_path(&staged_file_path)?;
    core_ignore_staged_import(&checked_vault_path(vault_path)?, &staged_file_path)
}

#[tauri::command]
pub fn bulk_accept_imports(
    vault_path: String,
    selected_files: Vec<String>,
    target_module: String,
    options: ImportAcceptanceOptions,
) -> Result<BulkImportAcceptReport, String> {
    let selected_files = selected_files
        .iter()
        .map(|path| checked_staged_import_path(path))
        .collect::<Result<Vec<_>, _>>()?;
    let target_module = checked_module_id(&target_module)?;
    core_bulk_accept_imports(
        &checked_vault_path(vault_path)?,
        selected_files,
        &target_module,
        options,
    )
}

#[tauri::command]
pub fn rebuild_core_cache(vault_path: String) -> Result<CoreCacheSnapshot, String> {
    rebuild_cache_from_vault(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn preview_entity_upgrade(vault_path: String) -> Result<EntityUpgradePreview, String> {
    core_preview_entity_upgrade(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn apply_entity_upgrade(
    vault_path: String,
    confirmation_token: String,
) -> Result<EntityUpgradeReport, String> {
    require_confirmation(apply_entity_upgrade_token(), &confirmation_token)?;
    core_apply_entity_upgrade(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn read_navigator(vault_path: String) -> Result<NavigatorSnapshot, String> {
    core_read_navigator(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn rebuild_navigator(vault_path: String) -> Result<NavigatorSnapshot, String> {
    core_rebuild_navigator(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn scan_and_rebuild_navigator(vault_path: String) -> Result<NavigatorRebuildReport, String> {
    core_scan_and_rebuild_navigator(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn rebuild_search_index(vault_path: String) -> Result<SearchIndexSnapshot, String> {
    core_rebuild_search_index(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn search_entities(vault_path: String, query: String) -> Result<SearchIndexSnapshot, String> {
    core_search_entities(&checked_vault_path(vault_path)?, &query)
}

#[tauri::command]
pub fn trash_managed_entity(
    vault_path: String,
    relative_path: String,
    confirmation_token: String,
) -> Result<TrashResult, String> {
    let expected = trash_token(&relative_path)?;
    require_confirmation(&expected, &confirmation_token)?;
    let relative_path = checked_user_markdown_path(&relative_path)?;
    core_trash_managed_entity(&checked_vault_path(vault_path)?, &relative_path)
}

#[tauri::command]
pub fn restore_trashed_entity(
    vault_path: String,
    trash_relative_path: String,
    restore_relative_path: String,
) -> Result<TrashResult, String> {
    let trash_relative_path =
        checked_lifecycle_file_path(&trash_relative_path, bentolife_core::TRASH_FOLDER)?;
    let restore_relative_path = checked_user_markdown_path(&restore_relative_path)?;
    core_restore_trashed_entity(
        &checked_vault_path(vault_path)?,
        &trash_relative_path,
        &restore_relative_path,
    )
}

#[tauri::command]
pub fn archive_managed_entity(
    vault_path: String,
    relative_path: String,
    confirmation_token: String,
) -> Result<ArchiveResult, String> {
    let expected = archive_token(&relative_path)?;
    require_confirmation(&expected, &confirmation_token)?;
    let relative_path = checked_user_markdown_path(&relative_path)?;
    core_archive_managed_entity(&checked_vault_path(vault_path)?, &relative_path)
}

#[tauri::command]
pub fn restore_archived_entity(
    vault_path: String,
    archive_relative_path: String,
    restore_relative_path: String,
) -> Result<ArchiveResult, String> {
    let archive_relative_path =
        checked_lifecycle_file_path(&archive_relative_path, bentolife_core::ARCHIVE_FOLDER)?;
    let restore_relative_path = checked_user_markdown_path(&restore_relative_path)?;
    core_restore_archived_entity(
        &checked_vault_path(vault_path)?,
        &archive_relative_path,
        &restore_relative_path,
    )
}

#[tauri::command]
pub fn list_trash_entries(vault_path: String) -> Result<Vec<FileLifecycleEntry>, String> {
    core_list_trash_entries(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn list_archive_entries(vault_path: String) -> Result<Vec<FileLifecycleEntry>, String> {
    core_list_archive_entries(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn restore_trash_entry(vault_path: String, entry_id: String) -> Result<TrashResult, String> {
    core_restore_trash_entry(&checked_vault_path(vault_path)?, &entry_id)
}

#[tauri::command]
pub fn restore_archive_entry(
    vault_path: String,
    entry_id: String,
) -> Result<ArchiveResult, String> {
    core_restore_archive_entry(&checked_vault_path(vault_path)?, &entry_id)
}

#[tauri::command]
pub fn delete_trash_entry_permanently(
    vault_path: String,
    entry_id: String,
    confirmation_token: String,
) -> Result<FileLifecycleMutationReport, String> {
    let expected = delete_trash_token(&entry_id)?;
    require_confirmation(&expected, &confirmation_token)?;
    let entries = core_delete_trash_entry_permanently(&checked_vault_path(vault_path)?, &entry_id)?;
    Ok(FileLifecycleMutationReport {
        action: "delete_trash_entry_permanently".to_string(),
        message: format!("Permanently deleted Trash entry {entry_id}."),
        changed_count: 1,
        entries,
    })
}

#[tauri::command]
pub fn empty_trash(
    vault_path: String,
    confirmation_token: String,
) -> Result<FileLifecycleMutationReport, String> {
    require_confirmation(empty_trash_token(), &confirmation_token)?;
    let before_count =
        core_list_trash_entries(&checked_vault_path(vault_path.clone())?)?.len() as u32;
    let entries = core_empty_trash(&checked_vault_path(vault_path)?)?;
    Ok(FileLifecycleMutationReport {
        action: "empty_trash".to_string(),
        message: "Trash was emptied permanently.".to_string(),
        changed_count: before_count,
        entries,
    })
}
