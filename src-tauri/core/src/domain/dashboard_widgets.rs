use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    layout_folder::LayoutFolderService,
    module_registry::{ModuleDefinition, ModuleRegistryService},
    module_schema::{WidgetSizeDefinition, WidgetTypeDefinition},
    storage::{content_hash, current_timestamp_label, read_json, write_json_atomic},
};

pub const DASHBOARD_WIDGET_STATE_VERSION: u32 = 1;
const DASHBOARD_WIDGET_MAX_COLUMNS: u32 = 7;
const DASHBOARD_WIDGET_MAX_HEIGHT: u32 = 3;
const DASHBOARD_WIDGET_SPARSE_REPAIR_MIN_ROW: u32 = 5;
const DASHBOARD_WIDGET_SPARSE_REPAIR_MARGIN: u32 = 3;
const DASHBOARD_WIDGET_SPARSE_REPAIR_WARNING: &str =
    "Dashboard widget layout was repaired after detecting sparse rows from an older layout bug.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardWidgetState {
    pub schema_version: u32,
    pub instances: Vec<DashboardWidgetInstance>,
    #[serde(default, skip_serializing)]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing)]
    pub recovery_backup_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_layout_operation: Option<DashboardWidgetLayoutOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardWidgetInstance {
    pub instance_id: String,
    pub widget_type: String,
    pub module_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
    pub layout: DashboardWidgetLayout,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default, skip_serializing)]
    pub active: bool,
    #[serde(default, skip_serializing)]
    pub status: String,
    #[serde(default, skip_serializing)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardWidgetLayout {
    pub column: u32,
    pub row: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardWidgetLayoutOperation {
    #[serde(default)]
    pub moved_widget_id: Option<String>,
    #[serde(default)]
    pub resized_widget_id: Option<String>,
    #[serde(default)]
    pub affected_widget_ids: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardWidgetCreateRequest {
    pub widget_type: String,
    pub module_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub config: BTreeMap<String, Value>,
    #[serde(default)]
    pub layout: Option<DashboardWidgetLayout>,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardWidgetUpdateRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub config: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    pub layout: Option<DashboardWidgetLayout>,
    #[serde(default)]
    pub collapsed: Option<bool>,
}

pub struct DashboardWidgetService;

impl DashboardWidgetService {
    pub fn widgets_path(vault_path: &Path) -> PathBuf {
        LayoutFolderService::layout_path(vault_path)
            .join("dashboard")
            .join("widgets.json")
    }

    pub fn default_state() -> DashboardWidgetState {
        DashboardWidgetState {
            schema_version: DASHBOARD_WIDGET_STATE_VERSION,
            instances: Vec::new(),
            warnings: Vec::new(),
            recovery_backup_path: None,
            last_layout_operation: None,
        }
    }

    pub fn available_widget_types(vault_path: &Path) -> Result<Vec<WidgetTypeDefinition>, String> {
        let registry = ModuleRegistryService::load_registry(vault_path)?;
        Ok(registry
            .modules
            .into_iter()
            .filter(|module| module.installed)
            .flat_map(|module| module.widget_types)
            .collect())
    }

    pub fn read_state(vault_path: &Path) -> Result<DashboardWidgetState, String> {
        let mut state = Self::read_persisted_state(vault_path, true)?;
        Self::repair_sparse_layout_if_needed(vault_path, &mut state)?;
        Self::reconcile_state(vault_path, &mut state)?;
        Ok(state)
    }

    pub fn write_state(
        vault_path: &Path,
        mut state: DashboardWidgetState,
    ) -> Result<DashboardWidgetState, String> {
        state.validate_basic()?;
        Self::validate_modules_installed(vault_path, &state)?;
        Self::persist_state(vault_path, &state)?;
        state.warnings.clear();
        Self::reconcile_state(vault_path, &mut state)?;
        Ok(state)
    }

    pub fn validate_import_state(
        vault_path: &Path,
        state: &DashboardWidgetState,
    ) -> Result<(), String> {
        state.validate_basic()?;
        for instance in &state.instances {
            let widget = Self::require_addable_widget_type(
                vault_path,
                &instance.module_id,
                &instance.widget_type,
            )?;
            validate_widget_config(&widget, &instance.config)?;
            ensure_allowed_size(&widget, instance.layout.width, instance.layout.height)?;
        }
        Ok(())
    }

    pub fn import_state(
        vault_path: &Path,
        state: DashboardWidgetState,
    ) -> Result<DashboardWidgetState, String> {
        Self::validate_import_state(vault_path, &state)?;
        Self::write_state(vault_path, state)
    }

    pub fn create_instance(
        vault_path: &Path,
        request: DashboardWidgetCreateRequest,
    ) -> Result<DashboardWidgetState, String> {
        let widget = Self::require_addable_widget_type(
            vault_path,
            &request.module_id,
            &request.widget_type,
        )?;
        let mut state = Self::read_persisted_state(vault_path, false)?;
        let mut config = default_config(&widget);
        for (key, value) in request.config {
            config.insert(key, value);
        }
        validate_widget_config(&widget, &config)?;
        let layout = request.layout.unwrap_or_else(|| {
            find_next_available_layout(
                &state.instances,
                widget.default_size.width,
                widget.default_size.height,
                DASHBOARD_WIDGET_MAX_COLUMNS,
            )
        });
        validate_layout(&layout)?;
        ensure_allowed_size(&widget, layout.width, layout.height)?;

        state.instances.push(DashboardWidgetInstance {
            instance_id: generate_widget_instance_id(&request.widget_type),
            widget_type: request.widget_type,
            module_id: request.module_id,
            title: request.title,
            config,
            layout,
            collapsed: request.collapsed,
            active: false,
            status: String::new(),
            warnings: Vec::new(),
        });
        Self::write_state(vault_path, state)
    }

    pub fn update_instance(
        vault_path: &Path,
        instance_id: &str,
        request: DashboardWidgetUpdateRequest,
    ) -> Result<DashboardWidgetState, String> {
        let mut state = Self::read_persisted_state(vault_path, false)?;
        let instance = find_instance_mut(&mut state, instance_id)?;
        if let Some(title) = request.title {
            instance.title = if title.trim().is_empty() {
                None
            } else {
                Some(title)
            };
        }
        if let Some(config) = request.config {
            if let Some(widget) =
                Self::find_widget_type(vault_path, &instance.module_id, &instance.widget_type)?
            {
                validate_widget_config(&widget, &config)?;
            }
            instance.config = config;
        }
        if let Some(layout) = request.layout {
            validate_layout(&layout)?;
            if let Some(widget) =
                Self::find_widget_type(vault_path, &instance.module_id, &instance.widget_type)?
            {
                ensure_allowed_size(&widget, layout.width, layout.height)?;
            }
            instance.layout = layout;
        }
        if let Some(collapsed) = request.collapsed {
            instance.collapsed = collapsed;
        }
        Self::write_state(vault_path, state)
    }

    pub fn remove_instance(
        vault_path: &Path,
        instance_id: &str,
    ) -> Result<DashboardWidgetState, String> {
        let mut state = Self::read_persisted_state(vault_path, false)?;
        let before = state.instances.len();
        state
            .instances
            .retain(|instance| instance.instance_id != instance_id);
        if state.instances.len() == before {
            return Err(format!(
                "Dashboard widget instance '{}' was not found.",
                instance_id
            ));
        }
        Self::write_state(vault_path, state)
    }

    pub fn duplicate_instance(
        vault_path: &Path,
        instance_id: &str,
    ) -> Result<DashboardWidgetState, String> {
        let mut state = Self::read_persisted_state(vault_path, false)?;
        let source = state
            .instances
            .iter()
            .find(|instance| instance.instance_id == instance_id)
            .cloned()
            .ok_or_else(|| format!("Dashboard widget instance '{}' was not found.", instance_id))?;
        if Self::find_module(vault_path, &source.module_id)?.is_none_or(|module| !module.installed)
        {
            return Err(format!(
                "Dashboard widget instance '{}' references an uninstalled module.",
                instance_id
            ));
        }
        let mut duplicate = source;
        duplicate.instance_id = generate_widget_instance_id(&duplicate.widget_type);
        duplicate.title = duplicate.title.map(|title| format!("{title} Copy"));
        duplicate.layout = find_next_available_layout(
            &state.instances,
            duplicate.layout.width,
            duplicate.layout.height,
            DASHBOARD_WIDGET_MAX_COLUMNS,
        );
        duplicate.active = false;
        duplicate.status.clear();
        duplicate.warnings.clear();
        state.instances.push(duplicate);
        Self::write_state(vault_path, state)
    }

    pub fn compact_layout(vault_path: &Path) -> Result<DashboardWidgetState, String> {
        let mut state = Self::read_persisted_state(vault_path, false)?;
        let placed = compact_widget_instances(&state.instances);
        let affected_widget_ids = placed
            .iter()
            .filter_map(|instance| {
                state
                    .instances
                    .iter()
                    .find(|before| before.instance_id == instance.instance_id)
                    .filter(|before| before.layout != instance.layout)
                    .map(|_| instance.instance_id.clone())
            })
            .collect();
        state.instances = placed;
        Self::write_layout_operation(
            vault_path,
            state,
            DashboardWidgetLayoutOperation {
                moved_widget_id: None,
                resized_widget_id: None,
                affected_widget_ids,
                reason: "compacted".to_string(),
            },
        )
    }

    pub fn move_instance(
        vault_path: &Path,
        instance_id: &str,
        column: u32,
        row: u32,
    ) -> Result<DashboardWidgetState, String> {
        let mut state = Self::read_persisted_state(vault_path, false)?;
        let previous_state = state.clone();
        let instance = find_instance_mut(&mut state, instance_id)?;
        instance.layout.column = column;
        instance.layout.row = row;
        validate_layout(&instance.layout)?;
        validate_no_layout_overlap(&state.instances, Some(instance_id))
            .map_err(|error| layout_blocked_message(&error.affected_widget_ids))?;
        let affected_widget_ids =
            changed_neighbor_ids(&previous_state.instances, &state.instances, &[instance_id]);
        Self::write_layout_operation(
            vault_path,
            state,
            DashboardWidgetLayoutOperation {
                moved_widget_id: Some(instance_id.to_string()),
                resized_widget_id: None,
                affected_widget_ids,
                reason: "moved".to_string(),
            },
        )
    }

    pub fn resize_instance(
        vault_path: &Path,
        instance_id: &str,
        width: u32,
        height: u32,
    ) -> Result<DashboardWidgetState, String> {
        let mut state = Self::read_persisted_state(vault_path, false)?;
        let previous_state = state.clone();
        let instance = find_instance_mut(&mut state, instance_id)?;
        if let Some(widget) =
            Self::find_widget_type(vault_path, &instance.module_id, &instance.widget_type)?
        {
            ensure_allowed_size(&widget, width, height)?;
        }
        instance.layout.width = width;
        instance.layout.height = height;
        validate_layout(&instance.layout)?;
        validate_no_layout_overlap(&state.instances, Some(instance_id))
            .map_err(|error| layout_blocked_message(&error.affected_widget_ids))?;
        let affected_widget_ids =
            changed_neighbor_ids(&previous_state.instances, &state.instances, &[instance_id]);
        Self::write_layout_operation(
            vault_path,
            state,
            DashboardWidgetLayoutOperation {
                moved_widget_id: None,
                resized_widget_id: Some(instance_id.to_string()),
                affected_widget_ids,
                reason: "resized".to_string(),
            },
        )
    }

    pub fn set_collapsed(
        vault_path: &Path,
        instance_id: &str,
        collapsed: bool,
    ) -> Result<DashboardWidgetState, String> {
        let mut state = Self::read_persisted_state(vault_path, false)?;
        let instance = find_instance_mut(&mut state, instance_id)?;
        instance.collapsed = collapsed;
        Self::write_state(vault_path, state)
    }

    pub fn reset_state(vault_path: &Path) -> Result<DashboardWidgetState, String> {
        let backup_path = Self::backup_malformed_state(vault_path)?;
        let mut state = Self::write_state(vault_path, Self::default_state())?;
        if let Some(backup_path) = backup_path {
            state.recovery_backup_path = Some(backup_path.clone());
            state.warnings.push(format!(
                "Malformed Dashboard widget metadata was backed up to {backup_path} before reset."
            ));
        }
        Ok(state)
    }

    fn read_persisted_state(
        vault_path: &Path,
        recover_malformed: bool,
    ) -> Result<DashboardWidgetState, String> {
        let path = Self::widgets_path(vault_path);
        if !path.exists() {
            return Ok(Self::default_state());
        }
        match read_json::<DashboardWidgetState>(&path).and_then(|mut state| {
            let warnings = state.normalize_recoverable_layouts();
            state.validate_basic()?;
            state.warnings.extend(warnings);
            Ok(state)
        }) {
            Ok(state) => Ok(state),
            Err(error) if recover_malformed => {
                let mut state = Self::default_state();
                state.warnings.push(format!(
                    "Dashboard widgets metadata could not be loaded and was not overwritten: {error}"
                ));
                Ok(state)
            }
            Err(error) => Err(error),
        }
    }

    fn persist_state(vault_path: &Path, state: &DashboardWidgetState) -> Result<(), String> {
        write_json_atomic(&Self::widgets_path(vault_path), state)
    }

    fn repair_sparse_layout_if_needed(
        vault_path: &Path,
        state: &mut DashboardWidgetState,
    ) -> Result<(), String> {
        if state.instances.is_empty() {
            return Ok(());
        }

        let max_row = state
            .instances
            .iter()
            .map(|instance| instance.layout.row)
            .max()
            .unwrap_or(1);
        if max_row <= DASHBOARD_WIDGET_SPARSE_REPAIR_MIN_ROW {
            return Ok(());
        }

        let compacted = compact_widget_instances(&state.instances);
        let compacted_max_row = compacted
            .iter()
            .map(|instance| instance.layout.row)
            .max()
            .unwrap_or(1);
        let instance_count = state.instances.len() as u32;
        let suspicious = max_row > compacted_max_row + DASHBOARD_WIDGET_SPARSE_REPAIR_MARGIN
            || max_row > instance_count + DASHBOARD_WIDGET_SPARSE_REPAIR_MARGIN;
        if !suspicious || layouts_match_by_instance(&state.instances, &compacted) {
            return Ok(());
        }

        let backup_path = Self::backup_widgets_file(vault_path, "Dashboard widget metadata")?;
        state.instances = compacted;
        state.validate_basic()?;
        Self::persist_state(vault_path, state)?;
        state.recovery_backup_path = Some(backup_path);
        state
            .warnings
            .push(DASHBOARD_WIDGET_SPARSE_REPAIR_WARNING.to_string());
        Ok(())
    }

    fn write_layout_operation(
        vault_path: &Path,
        mut state: DashboardWidgetState,
        operation: DashboardWidgetLayoutOperation,
    ) -> Result<DashboardWidgetState, String> {
        state.last_layout_operation = None;
        let mut written = Self::write_state(vault_path, state)?;
        written.last_layout_operation = Some(operation);
        Ok(written)
    }

    fn backup_malformed_state(vault_path: &Path) -> Result<Option<String>, String> {
        let path = Self::widgets_path(vault_path);
        if !path.exists() {
            return Ok(None);
        }
        if read_json::<DashboardWidgetState>(&path)
            .and_then(|state| state.validate_basic().map(|_| state))
            .is_ok()
        {
            return Ok(None);
        }

        Ok(Some(Self::backup_widgets_file(
            vault_path,
            "malformed Dashboard widget metadata",
        )?))
    }

    fn backup_widgets_file(vault_path: &Path, label: &str) -> Result<String, String> {
        let path = Self::widgets_path(vault_path);
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Unable to read {label} for backup: {error}"))?;
        let backup_root = LayoutFolderService::layout_path(vault_path)
            .join("backups")
            .join("dashboard-widgets");
        fs::create_dir_all(&backup_root).map_err(|error| {
            format!(
                "Unable to create Dashboard widget metadata backup folder {}: {error}",
                backup_root.display()
            )
        })?;
        let timestamp_label = current_timestamp_label().replace(':', "-");
        let digest = content_hash(&content);
        let backup_path = backup_root.join(format!("widgets-{timestamp_label}-{digest}.json"));
        fs::write(&backup_path, content).map_err(|error| {
            format!(
                "Unable to back up {label} to {}: {error}",
                backup_path.display()
            )
        })?;
        Ok(vault_relative_label(vault_path, &backup_path))
    }

    fn reconcile_state(vault_path: &Path, state: &mut DashboardWidgetState) -> Result<(), String> {
        let registry = ModuleRegistryService::load_registry(vault_path)?;
        for instance in &mut state.instances {
            instance.active = false;
            instance.status.clear();
            instance.warnings.clear();

            let Some(module) = registry
                .modules
                .iter()
                .find(|module| module.id == instance.module_id)
            else {
                instance.status = "unavailable_module".to_string();
                instance.warnings.push(format!(
                    "Widget instance '{}' references unknown module '{}'.",
                    instance.instance_id, instance.module_id
                ));
                continue;
            };
            if !module.installed {
                instance.status = "uninstalled_module".to_string();
                instance.warnings.push(format!(
                    "Widget instance '{}' references uninstalled module '{}'.",
                    instance.instance_id, instance.module_id
                ));
                continue;
            }
            if !module
                .widget_types
                .iter()
                .any(|widget| widget.id == instance.widget_type)
            {
                instance.status = "unavailable_widget_type".to_string();
                instance.warnings.push(format!(
                    "Widget instance '{}' references unavailable widget type '{}'.",
                    instance.instance_id, instance.widget_type
                ));
                continue;
            }
            if !module.enabled {
                instance.status = "inactive_module_disabled".to_string();
                instance.warnings.push(format!(
                    "Widget instance '{}' is inactive because module '{}' is disabled.",
                    instance.instance_id, instance.module_id
                ));
                continue;
            }
            instance.active = true;
            instance.status = "active".to_string();
        }
        state.warnings.extend(
            state
                .instances
                .iter()
                .flat_map(|instance| instance.warnings.iter().cloned()),
        );
        Ok(())
    }

    fn validate_modules_installed(
        vault_path: &Path,
        state: &DashboardWidgetState,
    ) -> Result<(), String> {
        let registry = ModuleRegistryService::load_registry(vault_path)?;
        for instance in &state.instances {
            let module = registry
                .modules
                .iter()
                .find(|module| module.id == instance.module_id)
                .ok_or_else(|| {
                    format!(
                        "Dashboard widget instance '{}' references unknown module '{}'.",
                        instance.instance_id, instance.module_id
                    )
                })?;
            if !module.installed {
                return Err(format!(
                    "Dashboard widget instance '{}' references uninstalled module '{}'.",
                    instance.instance_id, instance.module_id
                ));
            }
        }
        Ok(())
    }

    fn require_addable_widget_type(
        vault_path: &Path,
        module_id: &str,
        widget_type: &str,
    ) -> Result<WidgetTypeDefinition, String> {
        let module = Self::find_module(vault_path, module_id)?
            .ok_or_else(|| format!("Module '{}' is not installed.", module_id))?;
        if !module.installed {
            return Err(format!("Module '{}' is not installed.", module_id));
        }
        if !module.enabled {
            return Err(format!(
                "Module '{}' must be enabled before adding widget '{}'.",
                module_id, widget_type
            ));
        }
        module
            .widget_types
            .into_iter()
            .find(|widget| widget.id == widget_type)
            .ok_or_else(|| format!("Widget type '{}' is not available.", widget_type))
    }

    fn find_module(vault_path: &Path, module_id: &str) -> Result<Option<ModuleDefinition>, String> {
        Ok(ModuleRegistryService::load_registry(vault_path)?
            .modules
            .into_iter()
            .find(|module| module.id == module_id))
    }

    fn find_widget_type(
        vault_path: &Path,
        module_id: &str,
        widget_type: &str,
    ) -> Result<Option<WidgetTypeDefinition>, String> {
        Ok(
            Self::find_module(vault_path, module_id)?.and_then(|module| {
                module
                    .widget_types
                    .into_iter()
                    .find(|widget| widget.id == widget_type)
            }),
        )
    }
}

impl DashboardWidgetState {
    fn normalize_recoverable_layouts(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        for instance in &mut self.instances {
            let original = instance.layout.clone();
            instance.layout.column =
                clamp_layout_value(instance.layout.column, 1, DASHBOARD_WIDGET_MAX_COLUMNS);
            instance.layout.row = instance.layout.row.max(1);
            instance.layout.width =
                clamp_layout_value(instance.layout.width, 1, DASHBOARD_WIDGET_MAX_COLUMNS);
            instance.layout.height =
                clamp_layout_value(instance.layout.height, 1, DASHBOARD_WIDGET_MAX_HEIGHT);
            if instance.layout.column + instance.layout.width - 1 > DASHBOARD_WIDGET_MAX_COLUMNS {
                instance.layout.width = DASHBOARD_WIDGET_MAX_COLUMNS - instance.layout.column + 1;
            }
            if instance.layout != original {
                warnings.push(format!(
                    "Widget instance '{}' had invalid layout values and was clamped to the V5 dashboard grid.",
                    instance.instance_id
                ));
            }
        }
        warnings
    }

    fn validate_basic(&self) -> Result<(), String> {
        if self.schema_version != DASHBOARD_WIDGET_STATE_VERSION {
            return Err(format!(
                "Unsupported Dashboard widget state version {}.",
                self.schema_version
            ));
        }
        let mut ids = BTreeSet::new();
        for instance in &self.instances {
            instance.validate_basic()?;
            if !ids.insert(instance.instance_id.clone()) {
                return Err(format!(
                    "Duplicate Dashboard widget instance '{}'.",
                    instance.instance_id
                ));
            }
        }
        validate_no_layout_overlap(&self.instances, None)
            .map_err(|error| layout_blocked_message(&error.affected_widget_ids))?;
        Ok(())
    }
}

impl DashboardWidgetInstance {
    fn validate_basic(&self) -> Result<(), String> {
        if !safe_instance_id(&self.instance_id) {
            return Err(format!(
                "Dashboard widget instance ID '{}' is not safe.",
                self.instance_id
            ));
        }
        if !safe_widget_ref(&self.widget_type) {
            return Err(format!(
                "Dashboard widget type '{}' is not safe.",
                self.widget_type
            ));
        }
        if !safe_module_id(&self.module_id) {
            return Err(format!(
                "Dashboard widget module ID '{}' is not safe.",
                self.module_id
            ));
        }
        if !self
            .widget_type
            .starts_with(&format!("{}.", self.module_id))
        {
            return Err(format!(
                "Dashboard widget '{}' must belong to module '{}'.",
                self.widget_type, self.module_id
            ));
        }
        validate_layout(&self.layout)?;
        for key in self.config.keys() {
            if !safe_config_key(key) {
                return Err(format!(
                    "Dashboard widget config key '{}' is not safe.",
                    key
                ));
            }
        }
        Ok(())
    }
}

fn find_instance_mut<'a>(
    state: &'a mut DashboardWidgetState,
    instance_id: &str,
) -> Result<&'a mut DashboardWidgetInstance, String> {
    state
        .instances
        .iter_mut()
        .find(|instance| instance.instance_id == instance_id)
        .ok_or_else(|| format!("Dashboard widget instance '{}' was not found.", instance_id))
}

fn default_config(widget: &WidgetTypeDefinition) -> BTreeMap<String, Value> {
    widget
        .config_schema
        .iter()
        .filter_map(|(key, field)| {
            field
                .default_value
                .clone()
                .map(|value| (key.clone(), value))
        })
        .collect()
}

fn validate_widget_config(
    widget: &WidgetTypeDefinition,
    config: &BTreeMap<String, Value>,
) -> Result<(), String> {
    for (key, value) in config {
        if !safe_config_key(key) {
            return Err(format!("Widget config key '{}' is not safe.", key));
        }
        let Some(field) = widget.config_schema.get(key) else {
            return Err(format!(
                "Widget type '{}' does not define config field '{}'.",
                widget.id, key
            ));
        };
        let valid = match field.field_type.as_str() {
            "text" | "date_range" | "date range" => value.is_string(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "enum" => value
                .as_str()
                .is_some_and(|selected| field.options.iter().any(|option| option == selected)),
            "tags" => value
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string)),
            _ => false,
        };
        if !valid {
            return Err(format!(
                "Widget type '{}' config field '{}' does not match type '{}'.",
                widget.id, key, field.field_type
            ));
        }
        if value.as_str().is_some_and(|text| {
            text.trim_start().starts_with("http://") || text.trim_start().starts_with("https://")
        }) {
            return Err(format!(
                "Widget type '{}' config field '{}' may not reference a remote URL.",
                widget.id, key
            ));
        }
    }
    Ok(())
}

fn ensure_allowed_size(
    widget: &WidgetTypeDefinition,
    width: u32,
    height: u32,
) -> Result<(), String> {
    if widget
        .allowed_sizes
        .iter()
        .any(|size: &WidgetSizeDefinition| size.width == width && size.height == height)
    {
        Ok(())
    } else {
        Err(format!(
            "Widget type '{}' does not allow size {}x{}.",
            widget.id, width, height
        ))
    }
}

fn validate_layout(layout: &DashboardWidgetLayout) -> Result<(), String> {
    if layout.column == 0 || layout.row == 0 || layout.width == 0 || layout.height == 0 {
        return Err("Dashboard widget layout values must be greater than zero.".to_string());
    }
    if layout.column > DASHBOARD_WIDGET_MAX_COLUMNS
        || layout.width > DASHBOARD_WIDGET_MAX_COLUMNS
        || layout.column + layout.width - 1 > DASHBOARD_WIDGET_MAX_COLUMNS
    {
        return Err("Dashboard widget layout must fit the 7-column dashboard grid.".to_string());
    }
    if layout.height > DASHBOARD_WIDGET_MAX_HEIGHT {
        return Err("Dashboard widget layout height must be 3 rows or less.".to_string());
    }
    Ok(())
}

fn find_next_available_layout(
    instances: &[DashboardWidgetInstance],
    width: u32,
    height: u32,
    max_columns: u32,
) -> DashboardWidgetLayout {
    let width = clamp_layout_value(width, 1, max_columns);
    let height = clamp_layout_value(height, 1, DASHBOARD_WIDGET_MAX_HEIGHT);
    let max_start_column = max_columns.saturating_sub(width).saturating_add(1).max(1);
    let mut row = 1;
    loop {
        for column in 1..=max_start_column {
            let candidate = DashboardWidgetLayout {
                column,
                row,
                width,
                height,
            };
            if !instances
                .iter()
                .any(|instance| layouts_overlap(&candidate, &instance.layout))
            {
                return candidate;
            }
        }
        row += 1;
    }
}

fn compact_widget_instances(instances: &[DashboardWidgetInstance]) -> Vec<DashboardWidgetInstance> {
    let mut placed: Vec<DashboardWidgetInstance> = Vec::new();
    let mut ordered = instances.to_vec();
    ordered.sort_by_key(|instance| {
        (
            instance.layout.row,
            instance.layout.column,
            instance.instance_id.clone(),
        )
    });
    for instance in ordered {
        let mut compacted = instance;
        compacted.layout = find_next_available_layout(
            &placed,
            compacted.layout.width,
            compacted.layout.height,
            DASHBOARD_WIDGET_MAX_COLUMNS,
        );
        placed.push(compacted);
    }
    placed
}

fn layouts_match_by_instance(
    left: &[DashboardWidgetInstance],
    right: &[DashboardWidgetInstance],
) -> bool {
    left.len() == right.len()
        && left.iter().all(|left_instance| {
            right
                .iter()
                .find(|right_instance| right_instance.instance_id == left_instance.instance_id)
                .is_some_and(|right_instance| right_instance.layout == left_instance.layout)
        })
}

fn layouts_overlap(left: &DashboardWidgetLayout, right: &DashboardWidgetLayout) -> bool {
    let left_end_column = left.column + left.width - 1;
    let right_end_column = right.column + right.width - 1;
    let left_end_row = left.row + left.height - 1;
    let right_end_row = right.row + right.height - 1;

    left.column <= right_end_column
        && left_end_column >= right.column
        && left.row <= right_end_row
        && left_end_row >= right.row
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayoutOverlapError {
    affected_widget_ids: Vec<String>,
}

fn validate_no_layout_overlap(
    instances: &[DashboardWidgetInstance],
    active_instance_id: Option<&str>,
) -> Result<(), LayoutOverlapError> {
    let mut affected_widget_ids = BTreeSet::new();
    for left_index in 0..instances.len() {
        for right_index in (left_index + 1)..instances.len() {
            let left = &instances[left_index];
            let right = &instances[right_index];
            if !layouts_overlap(&left.layout, &right.layout) {
                continue;
            }
            match active_instance_id {
                Some(active_id) if left.instance_id == active_id => {
                    affected_widget_ids.insert(right.instance_id.clone());
                }
                Some(active_id) if right.instance_id == active_id => {
                    affected_widget_ids.insert(left.instance_id.clone());
                }
                Some(_) => {}
                None => {
                    affected_widget_ids.insert(left.instance_id.clone());
                    affected_widget_ids.insert(right.instance_id.clone());
                }
            }
        }
    }
    if affected_widget_ids.is_empty() {
        Ok(())
    } else {
        Err(LayoutOverlapError {
            affected_widget_ids: affected_widget_ids.into_iter().collect(),
        })
    }
}

fn changed_neighbor_ids(
    before: &[DashboardWidgetInstance],
    after: &[DashboardWidgetInstance],
    active_instance_ids: &[&str],
) -> Vec<String> {
    after
        .iter()
        .filter(|after_instance| {
            !active_instance_ids
                .iter()
                .any(|active_id| after_instance.instance_id == *active_id)
        })
        .filter_map(|after_instance| {
            before
                .iter()
                .find(|before_instance| before_instance.instance_id == after_instance.instance_id)
                .filter(|before_instance| before_instance.layout != after_instance.layout)
                .map(|_| after_instance.instance_id.clone())
        })
        .collect()
}

fn layout_blocked_message(_affected_widget_ids: &[String]) -> String {
    "This position overlaps another widget. Try another spot or use Compact layout.".to_string()
}

fn clamp_layout_value(value: u32, min: u32, max: u32) -> u32 {
    value.max(min).min(max)
}

fn safe_instance_id(value: &str) -> bool {
    value.starts_with("bl_widget_")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}

fn safe_module_id(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn safe_widget_ref(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '-'
                || character == '.'
        })
}

fn safe_config_key(value: &str) -> bool {
    !matches!(
        value,
        "path" | "component" | "script" | "stylesheet" | "url" | "html" | "jsx" | "tsx" | "css"
    ) && value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
}

fn generate_widget_instance_id(seed: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sanitized_seed = seed.replace(['.', '-'], "_");
    format!("bl_widget_{sanitized_seed}_{nanos:x}")
}

fn vault_relative_label(vault_path: &Path, path: &Path) -> String {
    path.strip_prefix(vault_path)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{dashboard::DashboardService, storage::current_timestamp_label};

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-widgets-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    fn widget_instance(
        instance_id: &str,
        widget_type: &str,
        module_id: &str,
        layout: DashboardWidgetLayout,
    ) -> DashboardWidgetInstance {
        DashboardWidgetInstance {
            instance_id: instance_id.to_string(),
            widget_type: widget_type.to_string(),
            module_id: module_id.to_string(),
            title: None,
            config: BTreeMap::new(),
            layout,
            collapsed: false,
            active: false,
            status: String::new(),
            warnings: Vec::new(),
        }
    }

    fn write_widget_state_fixture(vault_path: &Path, instances: Vec<DashboardWidgetInstance>) {
        let path = DashboardWidgetService::widgets_path(vault_path);
        std::fs::create_dir_all(path.parent().expect("widgets parent")).expect("widgets parent");
        let state = DashboardWidgetState {
            schema_version: DASHBOARD_WIDGET_STATE_VERSION,
            instances,
            warnings: Vec::new(),
            recovery_backup_path: None,
            last_layout_operation: None,
        };
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&state).expect("fixture json"),
        )
        .expect("fixture");
    }

    fn projected_row(layout: &DashboardWidgetLayout, visible_columns: u32) -> u32 {
        let projected_index = (layout.row - 1) * DASHBOARD_WIDGET_MAX_COLUMNS + (layout.column - 1);
        projected_index / visible_columns + 1
    }

    #[test]
    fn missing_widgets_file_reads_as_empty_state() {
        let vault_path = unique_temp_vault("missing");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert!(state.instances.is_empty());
        assert!(state.warnings.is_empty());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn creates_moves_resizes_collapses_duplicates_and_removes_widget_instances() {
        let vault_path = unique_temp_vault("lifecycle");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");

        let created = DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.upcoming".to_string(),
                module_id: "todos".to_string(),
                title: Some("Tasks This Week".to_string()),
                config: BTreeMap::new(),
                layout: None,
                collapsed: false,
            },
        )
        .expect("create");
        let instance_id = created.instances[0].instance_id.clone();
        assert_eq!(created.instances[0].status, "active");
        assert_eq!(created.instances[0].config["range_days"], Value::from(7));

        let moved =
            DashboardWidgetService::move_instance(&vault_path, &instance_id, 2, 3).expect("move");
        assert_eq!(moved.instances[0].layout.column, 2);
        assert_eq!(moved.instances[0].layout.row, 3);

        let resized = DashboardWidgetService::resize_instance(&vault_path, &instance_id, 2, 2)
            .expect("resize");
        assert_eq!(resized.instances[0].layout.height, 2);

        let collapsed = DashboardWidgetService::set_collapsed(&vault_path, &instance_id, true)
            .expect("collapse");
        assert!(collapsed.instances[0].collapsed);

        let duplicated = DashboardWidgetService::duplicate_instance(&vault_path, &instance_id)
            .expect("duplicate");
        assert_eq!(duplicated.instances.len(), 2);
        assert_eq!(duplicated.instances[1].layout.column, 1);
        assert_eq!(duplicated.instances[1].layout.row, 1);

        let removed =
            DashboardWidgetService::remove_instance(&vault_path, &instance_id).expect("remove");
        assert_eq!(removed.instances.len(), 1);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn places_new_widgets_in_first_available_grid_slot() {
        let vault_path = unique_temp_vault("placement");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");

        for _ in 0..3 {
            DashboardWidgetService::create_instance(
                &vault_path,
                DashboardWidgetCreateRequest {
                    widget_type: "todos.today".to_string(),
                    module_id: "todos".to_string(),
                    title: None,
                    config: BTreeMap::new(),
                    layout: None,
                    collapsed: false,
                },
            )
            .expect("create");
        }

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");
        assert_eq!(state.instances[0].layout.column, 1);
        assert_eq!(state.instances[0].layout.row, 1);
        assert_eq!(state.instances[1].layout.column, 3);
        assert_eq!(state.instances[1].layout.row, 1);
        assert_eq!(state.instances[2].layout.column, 5);
        assert_eq!(state.instances[2].layout.row, 1);
        assert!(!layouts_overlap(
            &state.instances[0].layout,
            &state.instances[1].layout
        ));
        assert!(!layouts_overlap(
            &state.instances[1].layout,
            &state.instances[2].layout
        ));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn resize_rejects_collision_and_preserves_neighbor_layouts() {
        let vault_path = unique_temp_vault("resize-collision");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");

        let first = DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.upcoming".to_string(),
                module_id: "todos".to_string(),
                title: Some("First".to_string()),
                config: BTreeMap::new(),
                layout: Some(DashboardWidgetLayout {
                    column: 1,
                    row: 1,
                    width: 2,
                    height: 1,
                }),
                collapsed: false,
            },
        )
        .expect("first");
        let first_id = first.instances[0].instance_id.clone();

        DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.upcoming".to_string(),
                module_id: "todos".to_string(),
                title: Some("Second".to_string()),
                config: BTreeMap::new(),
                layout: Some(DashboardWidgetLayout {
                    column: 3,
                    row: 1,
                    width: 2,
                    height: 1,
                }),
                collapsed: false,
            },
        )
        .expect("second");
        DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.upcoming".to_string(),
                module_id: "todos".to_string(),
                title: Some("Third".to_string()),
                config: BTreeMap::new(),
                layout: Some(DashboardWidgetLayout {
                    column: 1,
                    row: 2,
                    width: 2,
                    height: 1,
                }),
                collapsed: false,
            },
        )
        .expect("third");

        let before = DashboardWidgetService::read_state(&vault_path).expect("before");
        let error = DashboardWidgetService::resize_instance(&vault_path, &first_id, 2, 2)
            .expect_err("collision rejected");
        assert!(error.contains("overlaps another widget"));
        let after = DashboardWidgetService::read_state(&vault_path).expect("after");
        assert_eq!(after.instances, before.instances);

        let move_error = DashboardWidgetService::move_instance(&vault_path, &first_id, 2, 1)
            .expect_err("move collision rejected");
        assert!(move_error.contains("overlaps another widget"));
        let after_move_error =
            DashboardWidgetService::read_state(&vault_path).expect("after move error");
        assert_eq!(after_move_error.instances, before.instances);

        let moved =
            DashboardWidgetService::move_instance(&vault_path, &first_id, 5, 2).expect("move");
        assert_eq!(moved.instances[0].layout.column, 5);
        assert_eq!(moved.instances[0].layout.row, 2);
        assert_eq!(
            moved
                .last_layout_operation
                .as_ref()
                .expect("operation")
                .moved_widget_id
                .as_deref(),
            Some(first_id.as_str())
        );
        assert!(moved
            .last_layout_operation
            .as_ref()
            .expect("operation")
            .affected_widget_ids
            .is_empty());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn remove_all_then_add_starts_at_top_left() {
        let vault_path = unique_temp_vault("remove-all-placement");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");

        let first = DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.today".to_string(),
                module_id: "todos".to_string(),
                title: None,
                config: BTreeMap::new(),
                layout: Some(DashboardWidgetLayout {
                    column: 1,
                    row: 8,
                    width: 1,
                    height: 1,
                }),
                collapsed: false,
            },
        )
        .expect("create");
        let instance_id = first.instances[0].instance_id.clone();
        DashboardWidgetService::remove_instance(&vault_path, &instance_id).expect("remove");

        let next = DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.today".to_string(),
                module_id: "todos".to_string(),
                title: None,
                config: BTreeMap::new(),
                layout: None,
                collapsed: false,
            },
        )
        .expect("create");

        assert_eq!(next.instances[0].layout.column, 1);
        assert_eq!(next.instances[0].layout.row, 1);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn compact_layout_is_explicit_and_preserves_non_overlapping_order() {
        let vault_path = unique_temp_vault("compact");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");

        let first = DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.today".to_string(),
                module_id: "todos".to_string(),
                title: Some("First".to_string()),
                config: BTreeMap::new(),
                layout: Some(DashboardWidgetLayout {
                    column: 1,
                    row: 4,
                    width: 1,
                    height: 1,
                }),
                collapsed: false,
            },
        )
        .expect("create");
        let first_id = first.instances[0].instance_id.clone();
        DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.upcoming".to_string(),
                module_id: "todos".to_string(),
                title: Some("Second".to_string()),
                config: BTreeMap::new(),
                layout: Some(DashboardWidgetLayout {
                    column: 3,
                    row: 4,
                    width: 2,
                    height: 1,
                }),
                collapsed: false,
            },
        )
        .expect("create");

        let before = DashboardWidgetService::read_state(&vault_path).expect("before");
        assert_eq!(before.instances[0].layout.row, 4);

        let compacted = DashboardWidgetService::compact_layout(&vault_path).expect("compact");
        let first_after = compacted
            .instances
            .iter()
            .find(|instance| instance.instance_id == first_id)
            .expect("first");
        assert_eq!(first_after.layout.row, 1);
        assert_eq!(first_after.layout.column, 1);
        assert!(!layouts_overlap(
            &compacted.instances[0].layout,
            &compacted.instances[1].layout
        ));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn read_state_repairs_sparse_persisted_row_50() {
        let vault_path = unique_temp_vault("sparse-row-50");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        write_widget_state_fixture(
            &vault_path,
            vec![widget_instance(
                "bl_widget_sparse",
                "todos.today",
                "todos",
                DashboardWidgetLayout {
                    column: 1,
                    row: 50,
                    width: 1,
                    height: 1,
                },
            )],
        );

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert_eq!(state.instances[0].layout.row, 1);
        assert_eq!(state.instances[0].layout.column, 1);
        assert!(state
            .warnings
            .iter()
            .any(|warning| warning == DASHBOARD_WIDGET_SPARSE_REPAIR_WARNING));
        assert!(state.recovery_backup_path.is_some());
        let persisted: DashboardWidgetState = serde_json::from_str(
            &std::fs::read_to_string(DashboardWidgetService::widgets_path(&vault_path))
                .expect("persisted"),
        )
        .expect("persisted json");
        assert_eq!(persisted.instances[0].layout.row, 1);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn sparse_repair_backs_up_original_valid_metadata() {
        let vault_path = unique_temp_vault("sparse-backup");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        write_widget_state_fixture(
            &vault_path,
            vec![widget_instance(
                "bl_widget_sparse",
                "todos.today",
                "todos",
                DashboardWidgetLayout {
                    column: 1,
                    row: 50,
                    width: 1,
                    height: 1,
                },
            )],
        );

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        let backup = state.recovery_backup_path.as_deref().expect("backup path");
        assert!(backup.starts_with(".bentolifelayout/backups/dashboard-widgets/"));
        let backup_content = std::fs::read_to_string(vault_path.join(backup)).expect("backup");
        assert!(backup_content.contains("\"row\": 50"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn valid_compact_layout_is_not_repaired_or_rewritten() {
        let vault_path = unique_temp_vault("valid-compact");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        write_widget_state_fixture(
            &vault_path,
            vec![
                widget_instance(
                    "bl_widget_first",
                    "todos.today",
                    "todos",
                    DashboardWidgetLayout {
                        column: 1,
                        row: 1,
                        width: 1,
                        height: 1,
                    },
                ),
                widget_instance(
                    "bl_widget_second",
                    "todos.overdue",
                    "todos",
                    DashboardWidgetLayout {
                        column: 2,
                        row: 1,
                        width: 1,
                        height: 1,
                    },
                ),
            ],
        );
        let path = DashboardWidgetService::widgets_path(&vault_path);
        let before = std::fs::read_to_string(&path).expect("before");

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert!(state.recovery_backup_path.is_none());
        assert!(!state
            .warnings
            .iter()
            .any(|warning| warning == DASHBOARD_WIDGET_SPARSE_REPAIR_WARNING));
        assert_eq!(std::fs::read_to_string(&path).expect("after"), before);
        assert!(!LayoutFolderService::layout_path(&vault_path)
            .join("backups")
            .join("dashboard-widgets")
            .exists());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn sparse_repair_preserves_order_and_prevents_overlap() {
        let vault_path = unique_temp_vault("sparse-order");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        write_widget_state_fixture(
            &vault_path,
            vec![
                widget_instance(
                    "bl_widget_late",
                    "todos.upcoming",
                    "todos",
                    DashboardWidgetLayout {
                        column: 3,
                        row: 20,
                        width: 2,
                        height: 1,
                    },
                ),
                widget_instance(
                    "bl_widget_early",
                    "todos.today",
                    "todos",
                    DashboardWidgetLayout {
                        column: 1,
                        row: 10,
                        width: 1,
                        height: 1,
                    },
                ),
                widget_instance(
                    "bl_widget_last",
                    "todos.overdue",
                    "todos",
                    DashboardWidgetLayout {
                        column: 5,
                        row: 50,
                        width: 1,
                        height: 1,
                    },
                ),
            ],
        );

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert_eq!(state.instances[0].instance_id, "bl_widget_early");
        assert_eq!(state.instances[0].layout.row, 1);
        assert_eq!(state.instances[1].instance_id, "bl_widget_late");
        assert_eq!(state.instances[1].layout.row, 1);
        assert_eq!(state.instances[2].instance_id, "bl_widget_last");
        assert!(validate_no_layout_overlap(&state.instances, None).is_ok());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn repaired_layout_projects_to_bounded_two_column_rows() {
        let vault_path = unique_temp_vault("sparse-projection");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        write_widget_state_fixture(
            &vault_path,
            vec![widget_instance(
                "bl_widget_sparse",
                "todos.today",
                "todos",
                DashboardWidgetLayout {
                    column: 1,
                    row: 50,
                    width: 1,
                    height: 1,
                },
            )],
        );

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");
        let projected_max_row = state
            .instances
            .iter()
            .map(|instance| projected_row(&instance.layout, 2))
            .max()
            .unwrap_or(1);

        assert!(projected_max_row <= state.instances.len() as u32 + 3);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn sparse_repair_keeps_unavailable_widget_metadata_recoverable() {
        let vault_path = unique_temp_vault("sparse-unavailable");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        write_widget_state_fixture(
            &vault_path,
            vec![widget_instance(
                "bl_widget_missing",
                "todos.removed",
                "todos",
                DashboardWidgetLayout {
                    column: 1,
                    row: 50,
                    width: 1,
                    height: 1,
                },
            )],
        );

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert_eq!(state.instances[0].layout.row, 1);
        assert_eq!(state.instances[0].status, "unavailable_widget_type");
        assert!(state
            .warnings
            .iter()
            .any(|warning| warning == DASHBOARD_WIDGET_SPARSE_REPAIR_WARNING));
        assert!(state
            .warnings
            .iter()
            .any(|warning| warning.contains("todos.removed")));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn disabled_module_widgets_are_preserved_as_inactive_metadata() {
        let vault_path = unique_temp_vault("disabled");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        super::super::module_registry::ModuleRegistryService::set_module_enabled(
            &vault_path,
            "habits",
            true,
        )
        .expect("enable habits");
        let created = DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "habits.daily-checkin".to_string(),
                module_id: "habits".to_string(),
                title: None,
                config: BTreeMap::new(),
                layout: None,
                collapsed: false,
            },
        )
        .expect("create");
        assert!(created.instances[0].active);

        super::super::module_registry::ModuleRegistryService::set_module_enabled(
            &vault_path,
            "habits",
            false,
        )
        .expect("disable habits");
        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert_eq!(state.instances.len(), 1);
        assert!(!state.instances[0].active);
        assert_eq!(state.instances[0].status, "inactive_module_disabled");

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn malformed_widgets_file_recovers_for_read_but_blocks_mutation() {
        let vault_path = unique_temp_vault("malformed");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        let path = DashboardWidgetService::widgets_path(&vault_path);
        std::fs::create_dir_all(path.parent().expect("widgets parent")).expect("parent");
        std::fs::write(&path, "{not json").expect("malformed");

        let read = DashboardWidgetService::read_state(&vault_path).expect("read");
        assert!(read.instances.is_empty());
        assert!(!read.warnings.is_empty());

        let create_error = DashboardWidgetService::create_instance(
            &vault_path,
            DashboardWidgetCreateRequest {
                widget_type: "todos.today".to_string(),
                module_id: "todos".to_string(),
                title: None,
                config: BTreeMap::new(),
                layout: None,
                collapsed: false,
            },
        )
        .expect_err("mutation blocked");
        assert!(create_error.contains("Invalid JSON"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn reset_backs_up_malformed_widgets_file_before_replacing_it() {
        let vault_path = unique_temp_vault("malformed-reset");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        let path = DashboardWidgetService::widgets_path(&vault_path);
        std::fs::create_dir_all(path.parent().expect("widgets parent")).expect("parent");
        std::fs::write(&path, "{not json").expect("malformed");

        let reset = DashboardWidgetService::reset_state(&vault_path).expect("reset");

        assert!(reset.instances.is_empty());
        let backup = reset.recovery_backup_path.as_deref().expect("backup path");
        assert!(backup.starts_with(".bentolifelayout/backups/dashboard-widgets/"));
        assert!(reset
            .warnings
            .iter()
            .any(|warning| warning.contains("backed up")));
        assert_eq!(
            std::fs::read_to_string(vault_path.join(backup)).expect("backup"),
            "{not json"
        );
        assert!(DashboardWidgetService::read_state(&vault_path)
            .expect("read")
            .warnings
            .is_empty());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn unavailable_widget_type_is_preserved_as_warning() {
        let vault_path = unique_temp_vault("missing-type");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        let path = DashboardWidgetService::widgets_path(&vault_path);
        std::fs::create_dir_all(path.parent().expect("widgets parent")).expect("parent");
        std::fs::write(
            &path,
            r#"{
              "schema_version": 1,
              "instances": [{
                "instance_id": "bl_widget_missing",
                "widget_type": "todos.removed",
                "module_id": "todos",
                "title": "Removed",
                "config": {},
                "layout": { "column": 1, "row": 1, "width": 1, "height": 1 },
                "collapsed": false
              }]
            }"#,
        )
        .expect("fixture");

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert_eq!(state.instances.len(), 1);
        assert_eq!(state.instances[0].status, "unavailable_widget_type");
        assert!(!state.instances[0].active);
        assert!(state
            .warnings
            .iter()
            .any(|warning| warning.contains("todos.removed")));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn v5_grid_accepts_seven_columns_and_three_row_widgets() {
        let vault_path = unique_temp_vault("v5-grid");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        let path = DashboardWidgetService::widgets_path(&vault_path);
        std::fs::create_dir_all(path.parent().expect("widgets parent")).expect("parent");
        std::fs::write(
            &path,
            r#"{
              "schema_version": 1,
              "instances": [{
                "instance_id": "bl_widget_wide",
                "widget_type": "todos.upcoming",
                "module_id": "todos",
                "title": "Wide",
                "config": {},
                "layout": { "column": 1, "row": 1, "width": 7, "height": 3 },
                "collapsed": false
              }]
            }"#,
        )
        .expect("fixture");

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert_eq!(state.instances[0].layout.width, 7);
        assert_eq!(state.instances[0].layout.height, 3);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn recoverable_manual_layout_values_are_clamped_with_warning() {
        let vault_path = unique_temp_vault("v5-clamp");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        let path = DashboardWidgetService::widgets_path(&vault_path);
        std::fs::create_dir_all(path.parent().expect("widgets parent")).expect("parent");
        std::fs::write(
            &path,
            r#"{
              "schema_version": 1,
              "instances": [{
                "instance_id": "bl_widget_clamp",
                "widget_type": "todos.upcoming",
                "module_id": "todos",
                "title": "Clamp",
                "config": {},
                "layout": { "column": 6, "row": 0, "width": 9, "height": 8 },
                "collapsed": false
              }]
            }"#,
        )
        .expect("fixture");

        let state = DashboardWidgetService::read_state(&vault_path).expect("state");

        assert_eq!(state.instances[0].layout.column, 6);
        assert_eq!(state.instances[0].layout.row, 1);
        assert_eq!(state.instances[0].layout.width, 2);
        assert_eq!(state.instances[0].layout.height, 3);
        assert!(state
            .warnings
            .iter()
            .any(|warning| warning.contains("clamped")));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
