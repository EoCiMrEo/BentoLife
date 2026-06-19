use super::shared::*;

#[tauri::command]
pub fn list_notes(vault_path: String) -> Result<Vec<NoteSummary>, String> {
    NotesService::list_notes(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn read_note(vault_path: String, document_id: String) -> Result<NoteDocument, String> {
    NotesService::read_note(&checked_vault_path(vault_path)?, &document_id)
}

#[tauri::command]
pub fn create_note(
    vault_path: String,
    title: String,
    markdown_body: Option<String>,
) -> Result<NoteDocument, String> {
    NotesService::create_note(&checked_vault_path(vault_path)?, &title, markdown_body)
}

#[tauri::command]
pub fn update_note(
    vault_path: String,
    document_id: String,
    markdown_body: String,
    expected_content_hash: Option<String>,
    overwrite_conflict: Option<bool>,
) -> Result<NoteDocument, String> {
    NotesService::update_note(
        &checked_vault_path(vault_path)?,
        &document_id,
        markdown_body,
        expected_content_hash,
        overwrite_conflict.unwrap_or(false),
    )
}

#[tauri::command]
pub fn rename_note(
    vault_path: String,
    document_id: String,
    new_title: String,
) -> Result<NoteDocument, String> {
    NotesService::rename_note(&checked_vault_path(vault_path)?, &document_id, &new_title)
}

#[tauri::command]
pub fn save_markdown_asset(
    vault_path: String,
    module_id: String,
    document_id: String,
    file_name: Option<String>,
    mime_type: String,
    bytes: Vec<u8>,
) -> Result<MarkdownAsset, String> {
    MarkdownAssetService::save_markdown_asset(
        &checked_vault_path(vault_path)?,
        &module_id,
        &document_id,
        file_name,
        &mime_type,
        bytes,
    )
}

#[tauri::command]
pub fn read_markdown_asset(
    vault_path: String,
    module_id: String,
    document_id: String,
    source: String,
) -> Result<MarkdownAssetRead, String> {
    MarkdownAssetService::read_markdown_asset(
        &checked_vault_path(vault_path)?,
        &module_id,
        &document_id,
        &source,
    )
}
