use super::shared::*;

#[tauri::command]
pub fn load_widget_types(vault_path: String) -> Result<Vec<WidgetTypeDefinition>, String> {
    DashboardWidgetService::available_widget_types(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn load_dashboard_widgets(vault_path: String) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::read_state(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn create_dashboard_widget(
    vault_path: String,
    input: DashboardWidgetCreateRequest,
) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::create_instance(&checked_vault_path(vault_path)?, input)
}

#[tauri::command]
pub fn update_dashboard_widget(
    vault_path: String,
    instance_id: String,
    input: DashboardWidgetUpdateRequest,
) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::update_instance(&checked_vault_path(vault_path)?, &instance_id, input)
}

#[tauri::command]
pub fn remove_dashboard_widget(
    vault_path: String,
    instance_id: String,
) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::remove_instance(&checked_vault_path(vault_path)?, &instance_id)
}

#[tauri::command]
pub fn duplicate_dashboard_widget(
    vault_path: String,
    instance_id: String,
) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::duplicate_instance(&checked_vault_path(vault_path)?, &instance_id)
}

#[tauri::command]
pub fn move_dashboard_widget(
    vault_path: String,
    instance_id: String,
    layout: DashboardWidgetLayout,
) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::move_instance(
        &checked_vault_path(vault_path)?,
        &instance_id,
        layout.column,
        layout.row,
    )
}

#[tauri::command]
pub fn resize_dashboard_widget(
    vault_path: String,
    instance_id: String,
    size: WidgetSizeDefinition,
) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::resize_instance(
        &checked_vault_path(vault_path)?,
        &instance_id,
        size.width,
        size.height,
    )
}

#[tauri::command]
pub fn set_dashboard_widget_collapsed(
    vault_path: String,
    instance_id: String,
    collapsed: bool,
) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::set_collapsed(&checked_vault_path(vault_path)?, &instance_id, collapsed)
}

#[tauri::command]
pub fn reset_dashboard_widgets(vault_path: String) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::reset_state(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn compact_dashboard_widgets(vault_path: String) -> Result<DashboardWidgetState, String> {
    DashboardWidgetService::compact_layout(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn load_workspace_ui_state(vault_path: String) -> Result<WorkspaceState, String> {
    WorkspaceMetadataService::read_workspace_state(&checked_vault_path(vault_path)?)
}

#[tauri::command]
pub fn save_workspace_ui_state(
    vault_path: String,
    state: WorkspaceState,
) -> Result<WorkspaceState, String> {
    let vault_path = checked_vault_path(vault_path)?;
    WorkspaceMetadataService::write_workspace_state(&vault_path, &state)?;
    WorkspaceMetadataService::read_workspace_state(&vault_path)
}
