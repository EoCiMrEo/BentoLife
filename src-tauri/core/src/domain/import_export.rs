use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    dashboard_widgets::{DashboardWidgetService, DashboardWidgetState},
    layout_folder::LayoutFolderService,
    layout_metadata::LayoutMetadata,
    storage::{content_hash, read_json, resolve_vault_relative_path, write_text_atomic},
    theme::ThemeService,
    workspace_metadata::WorkspaceMetadataService,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportValidation {
    pub kind: String,
    pub safe: bool,
    pub message: String,
    pub source_path: String,
    pub normalized_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportResult {
    pub validation: ImportValidation,
    pub stored_relative_path: String,
    pub bytes_copied: usize,
}

pub struct ImportExportService;

impl ImportExportService {
    pub fn service_name() -> &'static str {
        "ImportExportService"
    }

    pub fn validate_layout_import(source_path: &Path) -> ImportValidation {
        validate_import(source_path, "layout")
    }

    pub fn validate_widget_layout_import(
        vault_path: &Path,
        source_path: &Path,
    ) -> ImportValidation {
        validate_widget_layout_import(vault_path, source_path)
    }

    pub fn validate_theme_import(source_path: &Path) -> ImportValidation {
        validate_import(source_path, "theme")
    }

    pub fn import_layout_file(
        vault_path: &Path,
        source_path: &Path,
    ) -> Result<ImportResult, String> {
        let validation = Self::validate_layout_import(source_path);
        if !validation.safe {
            return Err(validation.message);
        }
        copy_validated_import(
            vault_path,
            source_path,
            validation,
            ".bentolifelayout/imports/layouts",
            "json",
        )
    }

    pub fn import_widget_layout_file(
        vault_path: &Path,
        source_path: &Path,
    ) -> Result<ImportResult, String> {
        let validation = Self::validate_widget_layout_import(vault_path, source_path);
        if !validation.safe {
            return Err(validation.message);
        }
        let content = fs::read_to_string(source_path)
            .map_err(|error| format!("Unable to read {}: {error}", source_path.display()))?;
        let state = serde_json::from_str::<DashboardWidgetState>(&content)
            .map_err(|error| format!("Dashboard widget layout JSON is invalid: {error}"))?;
        let result = copy_validated_import(
            vault_path,
            source_path,
            validation,
            ".bentolifelayout/imports/dashboard-widgets",
            "json",
        )?;
        DashboardWidgetService::import_state(vault_path, state)?;
        Ok(result)
    }

    pub fn export_widget_layout_file(
        vault_path: &Path,
        output_path: &Path,
    ) -> Result<ImportResult, String> {
        let source_path = output_path.to_string_lossy().to_string();
        let Some(extension) = output_path
            .extension()
            .and_then(|extension| extension.to_str())
        else {
            return Err("Dashboard widget layout exports must be .json files.".to_string());
        };
        if is_executable_or_unexpected_extension(extension)
            || !extension.eq_ignore_ascii_case("json")
        {
            return Err("Dashboard widget layout exports must be .json files.".to_string());
        }
        let normalized_name = normalized_import_name(output_path, "json").map_err(|_| {
            "Export file name must include at least one ASCII letter or number.".to_string()
        })?;
        let state = DashboardWidgetService::read_state(vault_path)?;
        DashboardWidgetService::validate_import_state(vault_path, &state)?;
        let content = serde_json::to_string_pretty(&state)
            .map_err(|error| format!("Unable to serialize Dashboard widget layout: {error}"))?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Unable to create {}: {error}", parent.display()))?;
        }
        fs::write(output_path, &content)
            .map_err(|error| format!("Unable to write {}: {error}", output_path.display()))?;
        Ok(ImportResult {
            validation: ImportValidation {
                kind: "widget_layout_export".to_string(),
                safe: true,
                message: "Dashboard widget layout export is valid and data-only.".to_string(),
                source_path,
                normalized_name: Some(normalized_name),
            },
            stored_relative_path: output_path.to_string_lossy().to_string(),
            bytes_copied: content.len(),
        })
    }

    pub fn import_theme_file(
        vault_path: &Path,
        source_path: &Path,
    ) -> Result<ImportResult, String> {
        let validation = Self::validate_theme_import(source_path);
        if !validation.safe {
            return Err(validation.message);
        }
        copy_validated_import(
            vault_path,
            source_path,
            validation,
            ".bentolifelayout/themes",
            "css",
        )
    }
}

fn validate_widget_layout_import(vault_path: &Path, source_path: &Path) -> ImportValidation {
    let source_path_label = source_path.to_string_lossy().to_string();
    let Some(extension) = source_path
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return unsafe_validation(
            "widget_layout",
            source_path_label,
            "Dashboard widget layout imports must be .json files.",
        );
    };
    if is_executable_or_unexpected_extension(extension) || !extension.eq_ignore_ascii_case("json") {
        return unsafe_validation(
            "widget_layout",
            source_path_label,
            "Dashboard widget layout imports must be .json files.",
        );
    }
    if !source_path.is_file() {
        return unsafe_validation(
            "widget_layout",
            source_path_label,
            "Import source file was not found.",
        );
    }
    let normalized_name = match normalized_import_name(source_path, "json") {
        Ok(name) => name,
        Err(message) => return unsafe_validation("widget_layout", source_path_label, &message),
    };
    let validation_result = fs::read_to_string(source_path)
        .map_err(|error| format!("Unable to read {}: {error}", source_path.display()))
        .and_then(|content| {
            serde_json::from_str::<DashboardWidgetState>(&content)
                .map_err(|error| format!("Dashboard widget layout JSON is invalid: {error}"))
        })
        .and_then(|state| DashboardWidgetService::validate_import_state(vault_path, &state));

    match validation_result {
        Ok(()) => ImportValidation {
            kind: "widget_layout".to_string(),
            safe: true,
            message: "Dashboard widget layout import is valid and data-only.".to_string(),
            source_path: source_path_label,
            normalized_name: Some(normalized_name),
        },
        Err(message) => unsafe_validation("widget_layout", source_path_label, &message),
    }
}

fn validate_import(source_path: &Path, kind: &str) -> ImportValidation {
    let source_path_label = source_path.to_string_lossy().to_string();
    let expected_extension = if kind == "layout" { "json" } else { "css" };
    let Some(extension) = source_path
        .extension()
        .and_then(|extension| extension.to_str())
    else {
        return unsafe_validation(
            kind,
            source_path_label,
            "Imported files must include a file extension.",
        );
    };

    if is_executable_or_unexpected_extension(extension)
        || !extension.eq_ignore_ascii_case(expected_extension)
    {
        return unsafe_validation(
            kind,
            source_path_label,
            "Imported layouts must be .json files and imported themes must be .css files.",
        );
    }

    if !source_path.is_file() {
        return unsafe_validation(kind, source_path_label, "Import source file was not found.");
    }

    let normalized_name = match normalized_import_name(source_path, expected_extension) {
        Ok(name) => name,
        Err(message) => return unsafe_validation(kind, source_path_label, &message),
    };

    let validation_result = if kind == "layout" {
        read_json::<LayoutMetadata>(source_path).and_then(|metadata| metadata.validate())
    } else {
        fs::read_to_string(source_path)
            .map_err(|error| {
                format!(
                    "Unable to read CSS theme at {}: {error}",
                    source_path.display()
                )
            })
            .and_then(|css| ThemeService::validate_css_theme(&css))
    };

    match validation_result {
        Ok(()) => ImportValidation {
            kind: kind.to_string(),
            safe: true,
            message: format!("{kind} import is valid and data-only."),
            source_path: source_path_label,
            normalized_name: Some(normalized_name),
        },
        Err(message) => unsafe_validation(kind, source_path_label, &message),
    }
}

fn copy_validated_import(
    vault_path: &Path,
    source_path: &Path,
    validation: ImportValidation,
    folder: &str,
    extension: &str,
) -> Result<ImportResult, String> {
    LayoutFolderService::create_or_repair(vault_path)?;
    WorkspaceMetadataService::write_bootstrap_files(vault_path)?;

    let content = fs::read_to_string(source_path)
        .map_err(|error| format!("Unable to read {}: {error}", source_path.display()))?;
    let base_name = validation
        .normalized_name
        .clone()
        .ok_or_else(|| "Validated import is missing a normalized file name.".to_string())?;
    let relative_path =
        unique_import_relative_path(vault_path, folder, &base_name, extension, &content)?;
    let relative_path = super::security::validate_runtime_import_target(&relative_path)?;
    let target_path = resolve_vault_relative_path(vault_path, &relative_path)?;
    write_text_atomic(&target_path, &content)?;

    Ok(ImportResult {
        validation,
        stored_relative_path: relative_path,
        bytes_copied: content.len(),
    })
}

fn unique_import_relative_path(
    vault_path: &Path,
    folder: &str,
    base_name: &str,
    extension: &str,
    content: &str,
) -> Result<String, String> {
    let stem = base_name
        .strip_suffix(&format!(".{extension}"))
        .unwrap_or(base_name)
        .trim_end_matches('.');
    let hash = &content_hash(content)[..8];
    let candidate = format!("{folder}/{stem}-{hash}.{extension}");
    let path = resolve_vault_relative_path(vault_path, &candidate)?;
    if path.exists() {
        return Ok(candidate);
    }
    Ok(candidate)
}

fn normalized_import_name(source_path: &Path, expected_extension: &str) -> Result<String, String> {
    let file_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Import source must have a valid file name.".to_string())?;
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err("Import file names must not contain path traversal segments.".to_string());
    }

    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| "Import source must have a valid file stem.".to_string())?;
    let mut sanitized = String::new();
    let mut last_dash = false;
    for character in stem.trim().to_lowercase().chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            sanitized.push(character);
            last_dash = false;
        } else if !last_dash {
            sanitized.push('-');
            last_dash = true;
        }
    }
    let sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.is_empty() {
        return Err(
            "Import file name must include at least one ASCII letter or number.".to_string(),
        );
    }
    Ok(format!("{sanitized}.{expected_extension}"))
}

fn is_executable_or_unexpected_extension(extension: &str) -> bool {
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "js" | "mjs"
            | "cjs"
            | "ts"
            | "tsx"
            | "jsx"
            | "html"
            | "htm"
            | "svg"
            | "wasm"
            | "exe"
            | "bat"
            | "cmd"
            | "ps1"
            | "vbs"
            | "com"
            | "scr"
            | "dll"
            | "msi"
    )
}

fn unsafe_validation(kind: &str, source_path: String, message: &str) -> ImportValidation {
    ImportValidation {
        kind: kind.to_string(),
        safe: false,
        message: message.to_string(),
        source_path,
        normalized_name: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{layout_metadata::LayoutMetadataService, storage::current_timestamp_label};
    use std::path::PathBuf;

    fn unique_temp_folder(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-import-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path
    }

    #[test]
    fn validates_and_imports_layout_json_inside_vault() {
        let folder = unique_temp_folder("layout");
        let vault_path = folder.join(".bentolifevault");
        let source_path = folder.join("shared-layout.json");
        fs::create_dir_all(&folder).expect("folder");
        let layout = LayoutMetadataService::create_default("bl_doc_import").expect("layout");
        fs::write(
            &source_path,
            serde_json::to_string_pretty(&layout).expect("layout json"),
        )
        .expect("fixture");

        let result =
            ImportExportService::import_layout_file(&vault_path, &source_path).expect("import");

        assert!(result.validation.safe);
        assert!(result
            .stored_relative_path
            .starts_with(".bentolifelayout/imports/layouts/shared-layout-"));
        assert!(vault_path.join(&result.stored_relative_path).is_file());

        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn validates_and_imports_static_css_theme_inside_vault() {
        let folder = unique_temp_folder("theme");
        let vault_path = folder.join(".bentolifevault");
        let source_path = folder.join("calm-theme.css");
        fs::create_dir_all(&folder).expect("folder");
        fs::write(&source_path, ":root { --background: #fff; }\n").expect("fixture");

        let result =
            ImportExportService::import_theme_file(&vault_path, &source_path).expect("import");

        assert!(result.validation.safe);
        assert!(result
            .stored_relative_path
            .starts_with(".bentolifelayout/themes/calm-theme-"));
        assert!(vault_path.join(&result.stored_relative_path).is_file());

        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn exports_dashboard_widget_layout_manifest() {
        let folder = unique_temp_folder("widget-export");
        let vault_path = folder.join(".bentolifevault");
        let output_path = folder.join("dashboard-widgets-export.json");
        fs::create_dir_all(&vault_path).expect("vault");

        let result = ImportExportService::export_widget_layout_file(&vault_path, &output_path)
            .expect("export");
        let validation =
            ImportExportService::validate_widget_layout_import(&vault_path, &output_path);

        assert_eq!(result.validation.kind, "widget_layout_export");
        assert!(result.validation.safe);
        assert!(output_path.is_file());
        assert!(validation.safe);

        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn rejects_unsafe_css_and_executable_file_types() {
        let folder = unique_temp_folder("unsafe");
        fs::create_dir_all(&folder).expect("folder");
        let css_path = folder.join("unsafe.css");
        let js_path = folder.join("theme.js");
        fs::write(&css_path, "@import url('https://example.com/x.css');").expect("css");
        fs::write(&js_path, "alert(1)").expect("js");

        let css_validation = ImportExportService::validate_theme_import(&css_path);
        let js_validation = ImportExportService::validate_theme_import(&js_path);

        assert!(!css_validation.safe);
        assert!(!js_validation.safe);

        let _ = fs::remove_dir_all(folder);
    }

    #[test]
    fn rejects_malformed_layout_json() {
        let folder = unique_temp_folder("bad-layout");
        fs::create_dir_all(&folder).expect("folder");
        let source_path = folder.join("bad-layout.json");
        fs::write(&source_path, "{\"schema_version\": 999}").expect("fixture");

        let validation = ImportExportService::validate_layout_import(&source_path);

        assert!(!validation.safe);

        let _ = fs::remove_dir_all(folder);
    }
}
