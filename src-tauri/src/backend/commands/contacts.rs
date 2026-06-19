use super::shared::*;

#[tauri::command]
pub fn read_contacts(vault_path: String) -> Result<ContactDocument, String> {
    ContactsService::read_contacts(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn create_contact(
    vault_path: String,
    contact: ContactInput,
) -> Result<ContactDocument, String> {
    ContactsService::create_contact(&checked_vault_path(vault_path)?, contact)
}

#[tauri::command]
pub fn update_contact(
    vault_path: String,
    contact_id: String,
    contact: ContactInput,
) -> Result<ContactDocument, String> {
    ContactsService::update_contact(&checked_vault_path(vault_path)?, &contact_id, contact)
}
