use super::shared::*;

#[tauri::command]
pub fn preview_workspace_recovery(vault_path: String) -> Result<WorkspaceRecoveryPreview, String> {
    RecoveryService::preview_workspace_recovery(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn recover_document_metadata(
    vault_path: String,
    markdown_relative_path: String,
) -> Result<RecoveryResult, String> {
    let markdown_relative_path = checked_recovery_markdown_path(&markdown_relative_path)?;
    RecoveryService::recover_document_metadata(
        &checked_vault_path(vault_path)?,
        &markdown_relative_path,
    )
}

#[tauri::command]
pub fn recover_layout_metadata(
    vault_path: String,
    document_id: String,
) -> Result<RecoveryResult, String> {
    RecoveryService::recover_layout_metadata(&checked_vault_path(vault_path)?, &document_id)
}

#[tauri::command]
pub fn orphan_missing_document_metadata(
    vault_path: String,
    document_id: String,
) -> Result<RecoveryResult, String> {
    RecoveryService::orphan_missing_document_metadata(
        &checked_vault_path(vault_path)?,
        &document_id,
    )
}

#[tauri::command]
pub fn restore_orphaned_document_metadata(
    vault_path: String,
    document_id: String,
    markdown_relative_path: String,
) -> Result<RecoveryResult, String> {
    let markdown_relative_path = checked_recovery_markdown_path(&markdown_relative_path)?;
    RecoveryService::restore_orphaned_document_metadata(
        &checked_vault_path(vault_path)?,
        &document_id,
        &markdown_relative_path,
    )
}

#[tauri::command]
pub fn repair_document_frontmatter_reference(
    vault_path: String,
    document_id: String,
) -> Result<RecoveryResult, String> {
    RecoveryService::repair_document_frontmatter_reference(
        &checked_vault_path(vault_path)?,
        &document_id,
    )
}
