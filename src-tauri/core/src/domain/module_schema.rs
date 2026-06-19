use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::markdown_parser::{ParsedEntityContract, ParsedFieldDescriptor};
use super::storage::resolve_vault_relative_path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleSchema {
    pub schema_version: u32,
    pub module_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    pub entity_type: String,
    #[serde(default)]
    pub data_path: Option<String>,
    #[serde(default)]
    pub index_path: Option<String>,
    #[serde(default)]
    pub default_view: Option<String>,
    #[serde(default)]
    pub enabled_by_default: Option<bool>,
    #[serde(default)]
    pub fields: Vec<ModuleSchemaField>,
    #[serde(default)]
    pub renderers: Vec<String>,
    #[serde(default)]
    pub validation: BTreeMap<String, Value>,
    #[serde(default)]
    pub views: Vec<ModuleSchemaView>,
    #[serde(default)]
    pub widgets: Vec<WidgetTypeDefinition>,
    #[serde(default)]
    pub theme: BTreeMap<String, Value>,
    #[serde(default)]
    pub migration_version: Option<u32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleSchemaField {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default, rename = "type")]
    pub field_type: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub renderer: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default, rename = "default")]
    pub default_value: Option<Value>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub validation: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ModuleSchemaView {
    Id(String),
    Definition(ModuleSchemaViewDefinition),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleSchemaViewDefinition {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub default_sort: Option<String>,
    #[serde(default)]
    pub visible_fields: Vec<String>,
    #[serde(default)]
    pub empty_state: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetTypeDefinition {
    pub id: String,
    pub module_id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(alias = "renderer")]
    pub renderer_id: String,
    pub default_size: WidgetSizeDefinition,
    #[serde(default)]
    pub allowed_sizes: Vec<WidgetSizeDefinition>,
    #[serde(default)]
    pub config_schema: BTreeMap<String, WidgetConfigFieldDefinition>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetSizeDefinition {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WidgetConfigFieldDefinition {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default, rename = "default")]
    pub default_value: Option<Value>,
    #[serde(default)]
    pub min: Option<Value>,
    #[serde(default)]
    pub max: Option<Value>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ModuleSchema {
    pub fn load(vault_path: &Path, relative_path: &str) -> Result<Self, String> {
        if !relative_path.ends_with("module.schema.json") {
            return Err("Module schema path must point at module.schema.json.".to_string());
        }
        let path = resolve_vault_relative_path(vault_path, relative_path)?;
        let content = std::fs::read_to_string(&path)
            .map_err(|error| format!("Unable to read module schema at {relative_path}: {error}"))?;
        let schema = serde_json::from_str::<ModuleSchema>(&content).map_err(|error| {
            format!("Unable to parse module schema at {relative_path}: {error}")
        })?;
        schema.validate()?;
        Ok(schema)
    }

    pub fn allowed_field_names(&self) -> BTreeSet<String> {
        let mut fields = BTreeSet::new();
        for field in &self.fields {
            fields.insert(normalize_field(&field.id));
            for alias in &field.aliases {
                fields.insert(normalize_field(alias));
            }
        }
        fields.insert("title".to_string());
        fields.insert("name".to_string());
        fields.insert("body".to_string());
        fields.insert("tags".to_string());
        fields.insert("relationships".to_string());
        fields.insert("related".to_string());
        fields
    }

    pub fn diagnostics(&self) -> Vec<ModuleSchemaDiagnostic> {
        let mut diagnostics = Vec::new();
        for key in self.extra.keys() {
            diagnostics.push(ModuleSchemaDiagnostic::warning(
                "unknown_schema_key",
                format!(
                    "Unknown module schema key '{}' is ignored and preserved.",
                    key
                ),
            ));
        }
        for field in &self.fields {
            for key in field.extra.keys() {
                diagnostics.push(ModuleSchemaDiagnostic::warning(
                    "unknown_field_schema_key",
                    format!(
                        "Unknown schema key '{}' on field '{}' is ignored and preserved.",
                        key, field.id
                    ),
                ));
            }
            if let Some(renderer) = &field.renderer {
                if !allowed_renderer_id(renderer) {
                    diagnostics.push(ModuleSchemaDiagnostic::warning(
                        "unknown_renderer_id",
                        format!(
                            "Renderer '{}' on field '{}' is not allowlisted and will use the generic fallback.",
                            renderer, field.id
                        ),
                    ));
                }
            }
        }
        for renderer in &self.renderers {
            if !allowed_renderer_id(renderer) {
                diagnostics.push(ModuleSchemaDiagnostic::warning(
                    "unknown_renderer_id",
                    format!(
                        "Renderer '{}' is not allowlisted and will use the generic fallback.",
                        renderer
                    ),
                ));
            }
        }
        for view in &self.views {
            if let Some(id) = view.id() {
                if !allowed_view_id(id) {
                    diagnostics.push(ModuleSchemaDiagnostic::warning(
                        "unknown_view_id",
                        format!("View '{}' is not allowlisted and will be ignored.", id),
                    ));
                }
            }
        }
        for key in self.theme.keys() {
            if !allowed_theme_token(key) {
                diagnostics.push(ModuleSchemaDiagnostic::warning(
                    "unknown_theme_token",
                    format!(
                        "Theme token '{}' is not allowlisted and will be ignored.",
                        key
                    ),
                ));
            }
        }
        for widget in &self.widgets {
            for key in widget.extra.keys() {
                diagnostics.push(ModuleSchemaDiagnostic::warning(
                    "unknown_widget_schema_key",
                    format!(
                        "Unknown schema key '{}' on widget '{}' is ignored and preserved.",
                        key, widget.id
                    ),
                ));
            }
            for (field_id, field) in &widget.config_schema {
                for key in field.extra.keys() {
                    diagnostics.push(ModuleSchemaDiagnostic::warning(
                        "unknown_widget_config_schema_key",
                        format!(
                            "Unknown schema key '{}' on widget '{}' config field '{}' is ignored and preserved.",
                            key, widget.id, field_id
                        ),
                    ));
                }
            }
        }
        diagnostics
    }

    pub fn field_descriptors(&self, entity: &ParsedEntityContract) -> Vec<ParsedFieldDescriptor> {
        let mut descriptors = Vec::new();
        let mut covered = BTreeSet::new();

        for field in &self.fields {
            let candidates = std::iter::once(field.id.as_str())
                .chain(field.aliases.iter().map(String::as_str))
                .collect::<Vec<_>>();
            for candidate in &candidates {
                covered.insert(normalize_field(candidate));
            }
            let raw_value = field_value(entity, &field.id, &field.aliases);
            let value = if raw_value.trim().is_empty() {
                field_default_string(field).unwrap_or_default()
            } else {
                raw_value
            };
            let mut warnings = Vec::new();
            if is_enum_field(field)
                && !value.trim().is_empty()
                && !enum_option_matches(&field.options, &value)
            {
                warnings.push(format!(
                    "Value '{}' is outside the approved options for field '{}' and is preserved.",
                    value, field.id
                ));
            }
            descriptors.push(ParsedFieldDescriptor {
                id: field.id.clone(),
                label: field
                    .label
                    .clone()
                    .unwrap_or_else(|| field_label(&field.id)),
                field_type: field
                    .field_type
                    .clone()
                    .unwrap_or_else(|| "text".to_string()),
                renderer_id: renderer_id(field.renderer.as_deref(), field.field_type.as_deref()),
                value,
                editable: false,
                aliases: field.aliases.clone(),
                options: field.options.clone(),
                default_value: field_default_string(field),
                warnings,
            });
        }

        for (key, value) in &entity.fields {
            if covered.contains(&normalize_field(key)) {
                continue;
            }
            descriptors.push(ParsedFieldDescriptor {
                id: key.clone(),
                label: field_label(key),
                field_type: "unknown".to_string(),
                renderer_id: "generic".to_string(),
                value: value.clone(),
                editable: false,
                aliases: Vec::new(),
                options: Vec::new(),
                default_value: None,
                warnings: vec![format!(
                    "Unknown field '{}' is preserved as fallback content.",
                    key
                )],
            });
        }

        descriptors
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version == 0 {
            return Err("Module schema version must be greater than zero.".to_string());
        }
        if !safe_identifier(&self.module_id) || !safe_identifier(&self.entity_type) {
            return Err("Module schema IDs must be lowercase safe identifiers.".to_string());
        }
        if let Some(kind) = &self.kind {
            if !matches!(kind.as_str(), "system" | "starter" | "optional") {
                return Err(format!("Invalid module schema kind '{}'.", kind));
            }
        }
        for (label, path) in [
            ("data_path", self.data_path.as_deref()),
            ("index_path", self.index_path.as_deref()),
        ] {
            if let Some(path) = path {
                validate_vault_relative_path(label, path)?;
            }
        }
        if let Some(default_view) = &self.default_view {
            if !allowed_view_id(default_view) {
                return Err(format!(
                    "Default view '{}' is not allowlisted.",
                    default_view
                ));
            }
        }
        for field in &self.fields {
            if !safe_field_name(&field.id) {
                return Err(format!("Module schema field '{}' is not safe.", field.id));
            }
            for alias in &field.aliases {
                if !safe_field_name(alias) {
                    return Err(format!("Module schema alias '{}' is not safe.", alias));
                }
            }
            if let Some(source) = &field.source {
                if !safe_source(source) {
                    return Err(format!(
                        "Module schema field '{}' uses unsupported source '{}'.",
                        field.id, source
                    ));
                }
            }
            if is_enum_field(field) {
                if field.options.is_empty() {
                    return Err(format!(
                        "Enum module schema field '{}' must declare options.",
                        field.id
                    ));
                }
                if let Some(default_value) = field_default_string(field) {
                    if !enum_option_matches(&field.options, &default_value) {
                        return Err(format!(
                            "Default value '{}' for enum field '{}' must match an option.",
                            default_value, field.id
                        ));
                    }
                }
            }
        }
        for view in &self.views {
            if let Some(id) = view.id() {
                if !allowed_view_id(id) {
                    return Err(format!("Module schema view '{}' is not allowlisted.", id));
                }
            }
        }
        for widget in &self.widgets {
            widget.validate(&self.module_id)?;
        }
        Ok(())
    }
}

impl WidgetTypeDefinition {
    fn validate(&self, module_id: &str) -> Result<(), String> {
        if !safe_widget_id(&self.id) {
            return Err(format!("Widget type ID '{}' is not safe.", self.id));
        }
        if self.module_id != module_id {
            return Err(format!(
                "Widget type '{}' must be owned by module '{}'.",
                self.id, module_id
            ));
        }
        let expected_prefix = format!("{module_id}.");
        if !self.id.starts_with(&expected_prefix) {
            return Err(format!(
                "Widget type '{}' must be prefixed with '{}'.",
                self.id, expected_prefix
            ));
        }
        if self.label.trim().is_empty() {
            return Err(format!("Widget type '{}' must include a label.", self.id));
        }
        if !allowed_widget_renderer_id(&self.renderer_id) {
            return Err(format!(
                "Widget type '{}' uses unsupported renderer '{}'.",
                self.id, self.renderer_id
            ));
        }
        self.default_size.validate(&self.id)?;
        if self.allowed_sizes.is_empty() {
            return Err(format!(
                "Widget type '{}' must declare at least one allowed size.",
                self.id
            ));
        }
        if !self
            .allowed_sizes
            .iter()
            .any(|size| size == &self.default_size)
        {
            return Err(format!(
                "Widget type '{}' default size must be included in allowed sizes.",
                self.id
            ));
        }
        for size in &self.allowed_sizes {
            size.validate(&self.id)?;
        }
        reject_runtime_keys(&self.extra, &format!("widget type '{}'", self.id))?;
        for (field_id, field) in &self.config_schema {
            if !safe_field_name(field_id) {
                return Err(format!(
                    "Widget type '{}' config field '{}' is not safe.",
                    self.id, field_id
                ));
            }
            field.validate(&self.id, field_id)?;
        }
        Ok(())
    }
}

impl WidgetSizeDefinition {
    fn validate(&self, widget_id: &str) -> Result<(), String> {
        if self.width == 0 || self.height == 0 || self.width > 7 || self.height > 3 {
            return Err(format!(
                "Widget type '{}' size must fit the 1..7 by 1..3 dashboard grid.",
                widget_id
            ));
        }
        Ok(())
    }
}

impl WidgetConfigFieldDefinition {
    fn validate(&self, widget_id: &str, field_id: &str) -> Result<(), String> {
        if !allowed_widget_config_type(&self.field_type) {
            return Err(format!(
                "Widget type '{}' config field '{}' uses unsupported type '{}'.",
                widget_id, field_id, self.field_type
            ));
        }
        if self.field_type == "enum" && self.options.is_empty() {
            return Err(format!(
                "Widget type '{}' enum config field '{}' must declare options.",
                widget_id, field_id
            ));
        }
        if let Some(default_value) = &self.default_value {
            validate_config_default(widget_id, field_id, &self.field_type, default_value)?;
        }
        for (label, value) in [("min", &self.min), ("max", &self.max)] {
            if let Some(value) = value {
                if !value.is_number() {
                    return Err(format!(
                        "Widget type '{}' config field '{}' {} must be numeric.",
                        widget_id, field_id, label
                    ));
                }
            }
        }
        if let (Some(min), Some(max)) = (&self.min, &self.max) {
            if min.as_f64().unwrap_or_default() > max.as_f64().unwrap_or_default() {
                return Err(format!(
                    "Widget type '{}' config field '{}' min must not exceed max.",
                    widget_id, field_id
                ));
            }
        }
        reject_runtime_keys(
            &self.extra,
            &format!("widget type '{}' config field '{}'", widget_id, field_id),
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleSchemaDiagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
}

impl ModuleSchemaDiagnostic {
    fn warning(code: &str, message: String) -> Self {
        Self {
            severity: "warning".to_string(),
            code: code.to_string(),
            message,
        }
    }
}

impl ModuleSchemaView {
    fn id(&self) -> Option<&str> {
        match self {
            ModuleSchemaView::Id(id) => Some(id.as_str()),
            ModuleSchemaView::Definition(view) => Some(view.id.as_str()),
        }
    }
}

pub fn normalize_field(field: &str) -> String {
    field.trim().to_lowercase().replace('_', " ")
}

pub fn apply_schema_descriptors(
    vault_path: &Path,
    schema_path: &str,
    entity: &mut ParsedEntityContract,
) -> Result<Vec<String>, String> {
    let schema = match ModuleSchema::load(vault_path, schema_path) {
        Ok(schema) => schema,
        Err(error) => return Ok(vec![format!("Module schema could not be loaded: {error}")]),
    };
    entity.field_descriptors = schema.field_descriptors(entity);

    let mut warnings = Vec::new();
    if entity.module_id.as_deref() != Some(schema.module_id.as_str()) {
        warnings.push(format!(
            "Parsed module {:?} does not match schema module {}.",
            entity.module_id, schema.module_id
        ));
    }
    if entity.entity_type.as_deref() != Some(schema.entity_type.as_str()) {
        warnings.push(format!(
            "Parsed entity type {:?} does not match schema entity type {}.",
            entity.entity_type, schema.entity_type
        ));
    }
    let allowed = schema.allowed_field_names();
    for field in &schema.fields {
        if field.required {
            let value = field_value(entity, &field.id, &field.aliases);
            if value.trim().is_empty()
                && !matches!(normalize_field(&field.id).as_str(), "title" | "name")
            {
                warnings.push(format!(
                    "Required {} field '{}' is missing and the original Markdown is preserved.",
                    schema.entity_type, field.id
                ));
            }
        }
        if is_enum_field(field) {
            let value = field_value(entity, &field.id, &field.aliases);
            if !value.trim().is_empty() && !enum_option_matches(&field.options, &value) {
                warnings.push(format!(
                    "{} field '{}' value '{}' is outside the approved options and the original Markdown is preserved.",
                    schema.entity_type, field.id, value
                ));
            }
        }
    }
    for field in entity.fields.keys() {
        if !allowed.contains(&normalize_field(field)) {
            warnings.push(format!(
                "Unknown {} field '{}' is preserved as fallback content.",
                schema.entity_type, field
            ));
        }
    }
    Ok(warnings)
}

fn field_value(entity: &ParsedEntityContract, field_id: &str, aliases: &[String]) -> String {
    let normalized = normalize_field(field_id);
    if normalized == "tags" {
        return entity.tags.join(", ");
    }
    if normalized == "relationships" || normalized == "related" {
        return entity.relationships.join(", ");
    }
    if normalized == "body" || normalized == "notes" {
        return String::new();
    }

    std::iter::once(field_id)
        .chain(aliases.iter().map(String::as_str))
        .find_map(|candidate| {
            entity
                .fields
                .get(candidate)
                .or_else(|| {
                    entity
                        .fields
                        .iter()
                        .find(|(key, _)| normalize_field(key) == normalize_field(candidate))
                        .map(|(_, value)| value)
                })
                .cloned()
        })
        .unwrap_or_default()
}

fn is_enum_field(field: &ModuleSchemaField) -> bool {
    field
        .field_type
        .as_deref()
        .is_some_and(|field_type| field_type.eq_ignore_ascii_case("enum"))
}

fn field_default_string(field: &ModuleSchemaField) -> Option<String> {
    match field.default_value.as_ref()? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn enum_option_matches(options: &[String], value: &str) -> bool {
    let normalized_value = normalize_enum_value(value);
    options
        .iter()
        .any(|option| normalize_enum_value(option) == normalized_value)
}

fn normalize_enum_value(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn renderer_id(renderer: Option<&str>, field_type: Option<&str>) -> String {
    let resolved = match renderer.or(field_type).unwrap_or("text") {
        "relation" => "relationships".to_string(),
        "enum" => "status".to_string(),
        "attachment" | "progress" => "generic".to_string(),
        value => value.to_string(),
    };
    if allowed_renderer_id(&resolved) {
        resolved
    } else {
        "generic".to_string()
    }
}

fn field_label(field: &str) -> String {
    let mut label = String::new();
    for word in field.replace(['_', '-'], " ").split_whitespace() {
        if !label.is_empty() {
            label.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            label.push(first.to_ascii_uppercase());
            label.push_str(chars.as_str());
        }
    }
    if label.is_empty() {
        "Field".to_string()
    } else {
        label
    }
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn safe_field_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || character == '_'
                || character == '-'
                || character == ' '
        })
}

fn validate_vault_relative_path(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.contains("..")
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains(':')
    {
        return Err(format!(
            "Module schema {} '{}' is not vault-relative and safe.",
            label, value
        ));
    }
    Ok(())
}

fn safe_source(value: &str) -> bool {
    matches!(
        value,
        "h1" | "heading" | "field" | "field_or_wikilink" | "body" | "section" | "derived"
    ) || value.starts_with("section:")
}

fn allowed_renderer_id(value: &str) -> bool {
    matches!(
        value,
        "generic"
            | "text"
            | "textarea"
            | "status"
            | "date"
            | "tags"
            | "relation"
            | "relationships"
            | "checklist"
            | "code"
            | "image"
            | "table"
            | "markdown"
            | "link"
            | "list"
            | "number"
            | "attachment"
            | "progress"
    )
}

fn allowed_view_id(value: &str) -> bool {
    matches!(
        value,
        "cards" | "list" | "table" | "focused_entity" | "system"
    )
}

fn allowed_theme_token(value: &str) -> bool {
    matches!(
        value,
        "--background"
            | "--foreground"
            | "--card"
            | "--card-foreground"
            | "--popover"
            | "--popover-foreground"
            | "--primary"
            | "--primary-foreground"
            | "--secondary"
            | "--secondary-foreground"
            | "--muted"
            | "--muted-foreground"
            | "--accent"
            | "--accent-foreground"
            | "--destructive"
            | "--destructive-foreground"
            | "--border"
            | "--input"
            | "--ring"
            | "--sage"
            | "--sage-foreground"
            | "--soft-blue"
            | "--soft-blue-foreground"
            | "--amber-note"
            | "--amber-note-foreground"
            | "--shadow-soft"
            | "--shadow-lifted"
            | "--habit-progress-height"
            | "--habit-streak-emphasis"
            | "--habit-completed-state"
            | "--todo-overdue-state"
            | "--todo-priority-emphasis"
            | "--todo-completed-state"
            | "--contact-relationship-chip"
    )
}

fn safe_widget_id(value: &str) -> bool {
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

fn allowed_widget_renderer_id(value: &str) -> bool {
    matches!(
        value,
        "recent_notes"
            | "pinned_notes"
            | "notes_by_tag"
            | "todo_list"
            | "habit_checkin"
            | "progress"
            | "recent_contacts"
            | "generic_widget"
    )
}

fn allowed_widget_config_type(value: &str) -> bool {
    matches!(
        value,
        "text" | "number" | "boolean" | "enum" | "tags" | "date_range" | "date range"
    )
}

fn validate_config_default(
    widget_id: &str,
    field_id: &str,
    field_type: &str,
    value: &Value,
) -> Result<(), String> {
    let valid = match field_type {
        "text" | "date_range" | "date range" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "enum" => value.is_string(),
        "tags" => value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_string)),
        _ => false,
    };
    if !valid {
        return Err(format!(
            "Widget type '{}' config field '{}' default does not match type '{}'.",
            widget_id, field_id, field_type
        ));
    }
    if let Some(default_text) = value.as_str() {
        if looks_like_remote_url(default_text) {
            return Err(format!(
                "Widget type '{}' config field '{}' default may not reference a remote URL.",
                widget_id, field_id
            ));
        }
    }
    Ok(())
}

fn reject_runtime_keys(extra: &BTreeMap<String, Value>, context: &str) -> Result<(), String> {
    for key in extra.keys() {
        if matches!(
            key.as_str(),
            "path"
                | "component"
                | "script"
                | "stylesheet"
                | "url"
                | "html"
                | "jsx"
                | "tsx"
                | "css"
                | "css_selector"
        ) {
            return Err(format!(
                "Module schema {} may not reference arbitrary '{}'.",
                context, key
            ));
        }
    }
    Ok(())
}

fn looks_like_remote_url(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value.starts_with("http://") || value.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v4_schema_sections_and_preserves_unknown_keys_as_warnings() {
        let schema = serde_json::from_str::<ModuleSchema>(
            r##"{
              "schema_version": 2,
              "module_id": "todos",
              "display_name": "Todos",
              "kind": "starter",
              "entity_type": "todos",
              "data_path": "modules/todos/data",
              "index_path": "modules/todos/INDEX.md",
              "default_view": "cards",
              "enabled_by_default": true,
              "fields": [
                { "id": "title", "type": "text", "required": true, "source": "h1", "unknown": true }
              ],
              "renderers": ["text", "mystery"],
              "validation": { "unknown_fields": "warn_and_preserve" },
              "views": [{ "id": "cards", "label": "Cards" }],
              "widgets": [],
              "theme": { "--primary": "#123456", "--not-real": "red" },
              "migration_version": 1,
              "future": true
            }"##,
        )
        .expect("schema");

        schema.validate().expect("valid schema");
        let diagnostics = schema.diagnostics();

        assert!(diagnostics
            .iter()
            .any(|warning| warning.code == "unknown_schema_key"));
        assert!(diagnostics
            .iter()
            .any(|warning| warning.code == "unknown_field_schema_key"));
        assert!(diagnostics
            .iter()
            .any(|warning| warning.message.contains("mystery")));
        assert!(diagnostics
            .iter()
            .any(|warning| warning.message.contains("--not-real")));
    }

    #[test]
    fn rejects_invalid_kind_paths_and_view_ids() {
        let invalid_kind = ModuleSchema {
            schema_version: 2,
            module_id: "todos".to_string(),
            display_name: None,
            kind: Some("marketplace".to_string()),
            entity_type: "todos".to_string(),
            data_path: None,
            index_path: None,
            default_view: None,
            enabled_by_default: None,
            fields: Vec::new(),
            renderers: Vec::new(),
            validation: BTreeMap::new(),
            views: Vec::new(),
            widgets: Vec::new(),
            theme: BTreeMap::new(),
            migration_version: None,
            extra: BTreeMap::new(),
        };
        assert!(invalid_kind.validate().is_err());

        let invalid_path = ModuleSchema {
            kind: Some("starter".to_string()),
            data_path: Some("../outside".to_string()),
            ..invalid_kind_with_safe_kind()
        };
        assert!(invalid_path.validate().is_err());

        let invalid_view = ModuleSchema {
            views: vec![ModuleSchemaView::Id("custom_component".to_string())],
            ..invalid_kind_with_safe_kind()
        };
        assert!(invalid_view.validate().is_err());
    }

    #[test]
    fn reads_v1_schema_without_requiring_v4_sections() {
        let schema = serde_json::from_str::<ModuleSchema>(
            r#"{
              "schema_version": 1,
              "module_id": "notes",
              "entity_type": "note",
              "fields": [
                { "id": "title", "type": "text", "aliases": ["name"] }
              ]
            }"#,
        )
        .expect("schema");

        schema.validate().expect("v1 read-compatible");

        assert_eq!(schema.schema_version, 1);
        assert!(schema.widgets.is_empty());
        assert!(schema.theme.is_empty());
        assert!(schema.views.is_empty());
    }

    fn invalid_kind_with_safe_kind() -> ModuleSchema {
        ModuleSchema {
            schema_version: 2,
            module_id: "todos".to_string(),
            display_name: None,
            kind: Some("starter".to_string()),
            entity_type: "todos".to_string(),
            data_path: None,
            index_path: None,
            default_view: None,
            enabled_by_default: None,
            fields: Vec::new(),
            renderers: Vec::new(),
            validation: BTreeMap::new(),
            views: Vec::new(),
            widgets: Vec::new(),
            theme: BTreeMap::new(),
            migration_version: None,
            extra: BTreeMap::new(),
        }
    }
}
