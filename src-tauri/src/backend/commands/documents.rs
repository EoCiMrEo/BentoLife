use super::shared::*;

#[tauri::command]
pub fn manage_markdown_document(
    vault_path: String,
    markdown_relative_path: String,
    markdown: String,
) -> Result<ManagedMarkdownDocument, String> {
    MarkdownDocumentService::manage_document(
        &checked_vault_path(vault_path)?,
        &checked_user_markdown_path(&markdown_relative_path)?,
        &markdown,
    )
}

#[tauri::command]
pub fn rebuild_workspace_index(vault_path: String) -> Result<WorkspaceIndex, String> {
    let vault_path = checked_vault_path(vault_path)?;
    let documents = DocumentMetadataService::list(&vault_path)?;
    let index = WorkspaceMetadataService::rebuild_index_from_documents(&documents)?;
    WorkspaceMetadataService::write_index(&vault_path, &index)?;
    Ok(index)
}

#[tauri::command]
pub fn scan_workspace(vault_path: String) -> Result<WorkspaceScanResult, String> {
    WorkspaceScanner::scan(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn read_dashboard_hub(vault_path: String) -> Result<DashboardHubDocument, String> {
    DashboardService::read_dashboard_hub(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn pin_dashboard_entity(
    vault_path: String,
    document_id: String,
) -> Result<DashboardHubDocument, String> {
    DashboardService::pin_dashboard_entity(&checked_vault_path(vault_path)?, &document_id)
}

#[tauri::command]
pub fn unpin_dashboard_entity(
    vault_path: String,
    document_id: String,
) -> Result<DashboardHubDocument, String> {
    DashboardService::unpin_dashboard_entity(&checked_vault_path(vault_path)?, &document_id)
}

#[tauri::command]
pub fn load_layout_metadata(
    vault_path: String,
    document_id: String,
) -> Result<LayoutMetadata, String> {
    LayoutMetadataService::read(&checked_vault_path(vault_path)?, &document_id)
}

#[tauri::command]
pub fn save_layout_metadata(
    vault_path: String,
    document_id: String,
    layout_metadata: LayoutMetadata,
) -> Result<LayoutMetadata, String> {
    if layout_metadata.document_id != document_id {
        return Err(
            "Layout metadata document ID does not match the requested document ID.".to_string(),
        );
    }

    let vault_path = checked_vault_path(vault_path)?;
    LayoutMetadataService::write(&vault_path, &layout_metadata)?;
    LayoutMetadataService::read(&vault_path, &document_id)
}
