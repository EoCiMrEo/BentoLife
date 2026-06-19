use super::shared::*;

#[tauri::command]
pub fn list_todos(vault_path: String) -> Result<Vec<TodoSummary>, String> {
    TodoService::list_todos(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn read_todo(vault_path: String, document_id: String) -> Result<TodoDocument, String> {
    TodoService::read_todo(&checked_vault_path(vault_path)?, &document_id)
}

#[tauri::command]
pub fn create_todo(
    vault_path: String,
    title: String,
    markdown_body: Option<String>,
) -> Result<TodoDocument, String> {
    TodoService::create_todo(&checked_vault_path(vault_path)?, &title, markdown_body)
}

#[tauri::command]
pub fn update_todo(
    vault_path: String,
    document_id: String,
    markdown_body: String,
) -> Result<TodoDocument, String> {
    TodoService::update_todo(
        &checked_vault_path(vault_path)?,
        &document_id,
        markdown_body,
    )
}

#[tauri::command]
pub fn rename_todo(
    vault_path: String,
    document_id: String,
    new_title: String,
) -> Result<TodoDocument, String> {
    TodoService::rename_todo(&checked_vault_path(vault_path)?, &document_id, &new_title)
}
