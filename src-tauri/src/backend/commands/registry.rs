use super::shared::*;

#[tauri::command]
pub fn list_core_modules() -> Result<Vec<ModuleDefinition>, String> {
    Ok(ModuleRegistry::core_modules())
}

#[tauri::command]
pub fn load_module_registry(vault_path: String) -> Result<RegistryState, String> {
    ModuleRegistryService::load_registry(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn set_module_enabled(
    vault_path: String,
    module_id: String,
    enabled: bool,
) -> Result<RegistryState, String> {
    let module_id = checked_module_id(&module_id)?;
    ModuleRegistryService::set_module_enabled(&checked_vault_path(vault_path)?, &module_id, enabled)
}
