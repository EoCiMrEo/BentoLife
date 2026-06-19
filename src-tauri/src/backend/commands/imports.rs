use super::shared::*;

#[tauri::command]
pub fn validate_layout_import(source_path: String) -> Result<ImportValidation, String> {
    Ok(ImportExportService::validate_layout_import(&PathBuf::from(
        source_path,
    )))
}

#[tauri::command]
pub fn import_layout_file(vault_path: String, source_path: String) -> Result<ImportResult, String> {
    ImportExportService::import_layout_file(
        &checked_vault_path(vault_path)?,
        &PathBuf::from(source_path),
    )
}

#[tauri::command]
pub fn validate_widget_layout_import(
    vault_path: String,
    source_path: String,
) -> Result<ImportValidation, String> {
    Ok(ImportExportService::validate_widget_layout_import(
        &checked_vault_path(vault_path)?,
        &PathBuf::from(source_path),
    ))
}

#[tauri::command]
pub fn import_widget_layout_file(
    vault_path: String,
    source_path: String,
) -> Result<ImportResult, String> {
    ImportExportService::import_widget_layout_file(
        &checked_vault_path(vault_path)?,
        &PathBuf::from(source_path),
    )
}

#[tauri::command]
pub fn export_widget_layout_file(
    vault_path: String,
    output_path: String,
) -> Result<ImportResult, String> {
    ImportExportService::export_widget_layout_file(
        &checked_vault_path(vault_path)?,
        &PathBuf::from(output_path),
    )
}

#[tauri::command]
pub fn validate_theme_import(source_path: String) -> Result<ImportValidation, String> {
    Ok(ImportExportService::validate_theme_import(&PathBuf::from(
        source_path,
    )))
}

#[tauri::command]
pub fn import_theme_file(vault_path: String, source_path: String) -> Result<ImportResult, String> {
    ImportExportService::import_theme_file(
        &checked_vault_path(vault_path)?,
        &PathBuf::from(source_path),
    )
}

#[tauri::command]
pub fn preview_folder_import(
    source_path: String,
    vault_path: String,
) -> Result<FolderImportPreview, String> {
    bentolife_core::plan_folder_import(
        &PathBuf::from(source_path),
        &checked_vault_path(vault_path)?,
    )
}

#[tauri::command]
pub fn import_folder_into_vault(
    source_path: String,
    vault_path: String,
) -> Result<FolderImportManifest, String> {
    apply_folder_import(
        &PathBuf::from(source_path),
        &checked_vault_path(vault_path)?,
    )
}
