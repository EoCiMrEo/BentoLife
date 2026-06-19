use super::shared::*;

#[tauri::command]
pub fn read_active_theme(vault_path: String) -> Result<ActiveThemeState, String> {
    ThemeService::read_active_theme(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn preview_theme_tokens(
    vault_path: String,
    scope: String,
    module_id: Option<String>,
    source_path: String,
) -> Result<ThemePreview, String> {
    ThemeService::preview_theme_tokens(
        &checked_vault_path(vault_path)?,
        &scope,
        module_id.as_deref(),
        &PathBuf::from(source_path),
    )
}

#[tauri::command]
pub fn apply_theme_tokens(
    vault_path: String,
    scope: String,
    module_id: Option<String>,
    source_path: String,
) -> Result<ActiveThemeState, String> {
    ThemeService::apply_theme_tokens(
        &checked_vault_path(vault_path)?,
        &scope,
        module_id.as_deref(),
        &PathBuf::from(source_path),
    )
}

#[tauri::command]
pub fn rollback_theme(
    vault_path: String,
    scope: String,
    module_id: Option<String>,
) -> Result<ActiveThemeState, String> {
    ThemeService::rollback_theme(
        &checked_vault_path(vault_path)?,
        &scope,
        module_id.as_deref(),
    )
}
