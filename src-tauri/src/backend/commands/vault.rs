use super::shared::*;

#[tauri::command]
pub fn get_default_vault_path() -> Result<String, String> {
    Ok(platform_paths::default_vault_path()?
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub fn inspect_vault(path: String) -> Result<VaultInspection, String> {
    Ok(VaultService::inspect(path))
}

#[tauri::command]
pub fn create_default_vault() -> Result<VaultInspection, String> {
    let path = platform_paths::default_vault_path()?;
    VaultService::create_vault(path)
}

#[tauri::command]
pub fn create_vault_at(path: String) -> Result<VaultInspection, String> {
    VaultService::create_vault(checked_vault_path(path)?)
}

#[tauri::command]
pub fn repair_vault_structure(
    path: String,
    confirmation_token: String,
) -> Result<VaultInspection, String> {
    require_confirmation(repair_vault_token(), &confirmation_token)?;
    VaultService::repair_vault_structure(checked_vault_path(path)?)
}
