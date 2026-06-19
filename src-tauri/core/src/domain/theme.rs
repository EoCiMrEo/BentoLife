use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    layout_folder::LayoutFolderService,
    storage::{current_timestamp_label, read_json, resolve_vault_relative_path, write_json_atomic},
    workspace_metadata::WorkspaceMetadataService,
};

pub type ThemeTokenMap = BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeManifest {
    pub schema_version: u32,
    pub theme_id: String,
    pub scope: String,
    pub module_id: Option<String>,
    pub source_path: Option<String>,
    pub tokens: ThemeTokenMap,
    pub active: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemePreview {
    pub safe: bool,
    pub message: String,
    pub scope: String,
    pub module_id: Option<String>,
    pub source_path: String,
    pub tokens: ThemeTokenMap,
    pub effective_tokens: ThemeTokenMap,
    pub rejected_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveThemeState {
    pub schema_version: u32,
    #[serde(default)]
    pub app_default_tokens: ThemeTokenMap,
    pub workspace_theme: ThemeManifest,
    #[serde(default)]
    pub module_default_tokens: BTreeMap<String, ThemeTokenMap>,
    pub module_themes: BTreeMap<String, ThemeManifest>,
    pub effective_tokens: ThemeTokenMap,
    #[serde(default)]
    pub effective_module_tokens: BTreeMap<String, ThemeTokenMap>,
    pub updated_at: String,
}

pub struct ThemeService;

impl ThemeService {
    pub const DEFAULT_THEME: &'static str = "clean-slate";
    pub const ACTIVE_THEME_PATH: &'static str = ".bentolifelayout/themes/active-theme.json";

    pub fn validate_css_theme(css: &str) -> Result<(), String> {
        let _ = Self::parse_css_tokens(css)?;
        Ok(())
    }

    pub fn read_active_theme(vault_path: &Path) -> Result<ActiveThemeState, String> {
        LayoutFolderService::create_or_repair(vault_path)?;
        WorkspaceMetadataService::write_bootstrap_files(vault_path)?;
        let path = resolve_vault_relative_path(vault_path, Self::ACTIVE_THEME_PATH)?;
        if path.is_file() {
            let state = read_json::<ActiveThemeState>(&path)?;
            let state = state.hydrated()?;
            return Ok(state);
        }
        let state = ActiveThemeState::default().hydrated()?;
        write_json_atomic(&path, &state)?;
        Ok(state)
    }

    pub fn preview_theme_tokens(
        vault_path: &Path,
        scope: &str,
        module_id: Option<&str>,
        source_path: &Path,
    ) -> Result<ThemePreview, String> {
        let scope = normalize_scope(scope)?;
        let module_id = normalize_module_id(&scope, module_id)?;
        let (source_path_label, content) =
            read_theme_source(vault_path, &scope, module_id.as_deref(), source_path)?;
        let tokens = if source_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            parse_json_tokens(&content)?
        } else {
            Self::parse_css_tokens(&content)?
        };

        let mut state = Self::read_active_theme(vault_path)?;
        let effective_tokens =
            preview_effective_tokens(&mut state, &scope, module_id.as_deref(), &tokens)?;

        Ok(ThemePreview {
            safe: true,
            message: "Theme token preview is safe to apply.".to_string(),
            scope,
            module_id,
            source_path: source_path_label,
            tokens,
            effective_tokens,
            rejected_tokens: Vec::new(),
        })
    }

    pub fn apply_theme_tokens(
        vault_path: &Path,
        scope: &str,
        module_id: Option<&str>,
        source_path: &Path,
    ) -> Result<ActiveThemeState, String> {
        let preview = Self::preview_theme_tokens(vault_path, scope, module_id, source_path)?;
        let mut state = Self::read_active_theme(vault_path)?;
        let manifest = ThemeManifest {
            schema_version: 1,
            theme_id: theme_id_for(&preview.scope, preview.module_id.as_deref()),
            scope: preview.scope.clone(),
            module_id: preview.module_id.clone(),
            source_path: Some(preview.source_path),
            tokens: preview.tokens,
            active: true,
            updated_at: current_timestamp_label(),
        };

        if manifest.scope == "workspace" {
            state.workspace_theme = manifest;
        } else if let Some(module_id) = &manifest.module_id {
            state.module_themes.insert(module_id.clone(), manifest);
        }
        state.updated_at = current_timestamp_label();
        state = state.hydrated()?;
        write_json_atomic(
            &resolve_vault_relative_path(vault_path, Self::ACTIVE_THEME_PATH)?,
            &state,
        )?;
        Ok(state)
    }

    pub fn rollback_theme(
        vault_path: &Path,
        scope: &str,
        module_id: Option<&str>,
    ) -> Result<ActiveThemeState, String> {
        let scope = normalize_scope(scope)?;
        let module_id = normalize_module_id(&scope, module_id)?;
        let mut state = Self::read_active_theme(vault_path)?;
        if scope == "workspace" {
            state.workspace_theme = ThemeManifest::clean_slate("workspace", None);
        } else if let Some(module_id) = module_id {
            state.module_themes.remove(&module_id);
        }
        state.updated_at = current_timestamp_label();
        state = state.hydrated()?;
        write_json_atomic(
            &resolve_vault_relative_path(vault_path, Self::ACTIVE_THEME_PATH)?,
            &state,
        )?;
        Ok(state)
    }

    #[cfg(test)]
    pub fn effective_tokens_for_module(
        state: &ActiveThemeState,
        module_id: Option<&str>,
    ) -> ThemeTokenMap {
        effective_tokens(state, module_id)
    }

    pub fn parse_css_tokens(css: &str) -> Result<ThemeTokenMap, String> {
        let normalized = css.to_lowercase();
        let rejected_patterns = [
            "@import",
            "url(",
            "javascript:",
            "expression(",
            "<script",
            "</script",
            "<html",
            "<body",
            "behavior:",
            "-moz-binding",
            "vbscript:",
            "data:text/html",
            "data:application/javascript",
            "data:text/javascript",
        ];

        for pattern in rejected_patterns {
            if normalized.contains(pattern) {
                return Err(format!("CSS theme contains a rejected executable or remote-loading pattern: {pattern}."));
            }
        }

        if css.trim().is_empty() {
            return Err("CSS theme is empty.".to_string());
        }

        let mut token_source = css.trim();
        if token_source.contains('{') || token_source.contains('}') {
            let trimmed = token_source.trim();
            if !trimmed.starts_with(":root") {
                return Err(
                    "CSS token themes may only use a :root custom-property block.".to_string(),
                );
            }
            let open = trimmed
                .find('{')
                .ok_or_else(|| "CSS token theme is missing an opening block.".to_string())?;
            let close = trimmed
                .rfind('}')
                .ok_or_else(|| "CSS token theme is missing a closing block.".to_string())?;
            if close <= open || !trimmed[close + 1..].trim().is_empty() {
                return Err(
                    "CSS token theme may not include selectors after the :root block.".to_string(),
                );
            }
            token_source = &trimmed[open + 1..close];
            if token_source.contains('{') || token_source.contains('}') {
                return Err("CSS token theme may not contain nested selectors.".to_string());
            }
        }

        parse_token_declarations(token_source)
    }
}

impl ThemeManifest {
    fn clean_slate(scope: &str, module_id: Option<String>) -> Self {
        Self {
            schema_version: 1,
            theme_id: theme_id_for(scope, module_id.as_deref()),
            scope: scope.to_string(),
            module_id,
            source_path: None,
            tokens: ThemeTokenMap::new(),
            active: true,
            updated_at: current_timestamp_label(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "Unsupported theme manifest version {}.",
                self.schema_version
            ));
        }
        normalize_scope(&self.scope)?;
        if self.scope == "module" {
            normalize_module_id(&self.scope, self.module_id.as_deref())?;
        }
        validate_tokens(&self.tokens)
    }
}

impl Default for ActiveThemeState {
    fn default() -> Self {
        let now = current_timestamp_label();
        Self {
            schema_version: 1,
            app_default_tokens: app_default_tokens(),
            workspace_theme: ThemeManifest::clean_slate("workspace", None),
            module_default_tokens: module_default_tokens(),
            module_themes: BTreeMap::new(),
            effective_tokens: ThemeTokenMap::new(),
            effective_module_tokens: BTreeMap::new(),
            updated_at: now,
        }
    }
}

impl ActiveThemeState {
    fn hydrated(mut self) -> Result<Self, String> {
        self.app_default_tokens = app_default_tokens();
        self.module_default_tokens = module_default_tokens();
        self.effective_tokens = effective_tokens(&self, None);
        self.effective_module_tokens = effective_module_tokens(&self);
        self.validate()?;
        Ok(self)
    }

    fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err(format!(
                "Unsupported active theme state version {}.",
                self.schema_version
            ));
        }
        validate_tokens(&self.app_default_tokens)?;
        self.workspace_theme.validate()?;
        for tokens in self.module_default_tokens.values() {
            validate_tokens(tokens)?;
        }
        for manifest in self.module_themes.values() {
            manifest.validate()?;
        }
        validate_tokens(&self.effective_tokens)?;
        for tokens in self.effective_module_tokens.values() {
            validate_tokens(tokens)?;
        }
        Ok(())
    }
}

fn normalize_scope(scope: &str) -> Result<String, String> {
    match scope.trim().to_lowercase().as_str() {
        "workspace" => Ok("workspace".to_string()),
        "module" => Ok("module".to_string()),
        _ => Err("Theme scope must be workspace or module.".to_string()),
    }
}

fn normalize_module_id(scope: &str, module_id: Option<&str>) -> Result<Option<String>, String> {
    if scope == "workspace" {
        return Ok(None);
    }
    let module_id = module_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Module theme scope requires a module ID.".to_string())?;
    if !module_id.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || character == '-'
            || character == '_'
    }) {
        return Err(
            "Module ID may only contain lowercase ASCII letters, numbers, dash, or underscore."
                .to_string(),
        );
    }
    Ok(Some(module_id.to_string()))
}

fn read_theme_source(
    vault_path: &Path,
    scope: &str,
    module_id: Option<&str>,
    source_path: &Path,
) -> Result<(String, String), String> {
    let path = if source_path.is_absolute() {
        if scope == "module" {
            let relative = source_path
                .strip_prefix(vault_path)
                .map_err(|_| {
                    "Module theme sources must live inside the selected vault.".to_string()
                })?
                .to_string_lossy()
                .replace('\\', "/");
            validate_module_theme_relative_path(module_id, &relative)?;
        }
        source_path.to_path_buf()
    } else {
        let relative = source_path.to_string_lossy().replace('\\', "/");
        if relative.contains("..") {
            return Err("Theme source paths must not contain traversal segments.".to_string());
        }
        if scope == "module" {
            validate_module_theme_relative_path(module_id, &relative)?;
        }
        resolve_vault_relative_path(vault_path, &relative)?
    };
    if !path.is_file() {
        return Err(format!(
            "Theme source file was not found at {}.",
            path.display()
        ));
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
    Ok((path.to_string_lossy().replace('\\', "/"), content))
}

fn validate_module_theme_relative_path(
    module_id: Option<&str>,
    relative: &str,
) -> Result<(), String> {
    let module_id =
        module_id.ok_or_else(|| "Module theme scope requires a module ID.".to_string())?;
    let json_prefix = format!("modules/{module_id}/theme/json/");
    let css_prefix = format!("modules/{module_id}/theme/css/");
    if relative.starts_with(&json_prefix) || relative.starts_with(&css_prefix) {
        Ok(())
    } else {
        Err(format!(
            "Module themes must live under {json_prefix} or {css_prefix}."
        ))
    }
}

fn parse_json_tokens(content: &str) -> Result<ThemeTokenMap, String> {
    let tokens = serde_json::from_str::<ThemeTokenMap>(content)
        .map_err(|_| "Theme JSON must be an object of token names to string values.".to_string())?;
    validate_tokens(&tokens)?;
    Ok(tokens)
}

fn parse_token_declarations(content: &str) -> Result<ThemeTokenMap, String> {
    let mut tokens = ThemeTokenMap::new();
    for declaration in content.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        let (name, value) = declaration
            .split_once(':')
            .ok_or_else(|| "CSS token declarations must use --token: value syntax.".to_string())?;
        let name = name.trim();
        let value = value.trim();
        if !name.starts_with("--") {
            return Err("CSS token themes may only contain custom properties.".to_string());
        }
        tokens.insert(name.to_string(), value.to_string());
    }
    validate_tokens(&tokens)?;
    if tokens.is_empty() {
        return Err("Theme token file did not contain any allowlisted tokens.".to_string());
    }
    Ok(tokens)
}

fn validate_tokens(tokens: &ThemeTokenMap) -> Result<(), String> {
    for (token, value) in tokens {
        if !allowed_tokens().contains(&token.as_str()) {
            return Err(format!("Theme token {token} is not allowlisted."));
        }
        validate_token_value(value)?;
    }
    Ok(())
}

fn validate_token_value(value: &str) -> Result<(), String> {
    let normalized = value.to_lowercase();
    for pattern in [
        "@",
        "{",
        "}",
        ";",
        "url(",
        "javascript:",
        "expression(",
        "<",
        ">",
        "behavior:",
        "-moz-binding",
    ] {
        if normalized.contains(pattern) {
            return Err(format!(
                "Theme token value contains a rejected pattern: {pattern}."
            ));
        }
    }
    if value.trim().is_empty() || value.len() > 180 {
        return Err(
            "Theme token values must be non-empty and shorter than 180 characters.".to_string(),
        );
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "#.,%()/ -_+".contains(character))
    {
        return Err(
            "Theme token values may only contain static color, size, or shadow characters."
                .to_string(),
        );
    }
    Ok(())
}

fn allowed_tokens() -> &'static [&'static str] {
    &[
        "--background",
        "--foreground",
        "--card",
        "--card-foreground",
        "--popover",
        "--popover-foreground",
        "--primary",
        "--primary-foreground",
        "--secondary",
        "--secondary-foreground",
        "--muted",
        "--muted-foreground",
        "--accent",
        "--accent-foreground",
        "--destructive",
        "--destructive-foreground",
        "--border",
        "--input",
        "--ring",
        "--sage",
        "--sage-foreground",
        "--soft-blue",
        "--soft-blue-foreground",
        "--amber-note",
        "--amber-note-foreground",
        "--shadow-soft",
        "--shadow-lifted",
        "--habit-progress-height",
        "--habit-streak-emphasis",
        "--habit-completed-state",
        "--todo-overdue-state",
        "--todo-priority-emphasis",
        "--todo-completed-state",
        "--contact-relationship-chip",
    ]
}

fn effective_tokens(state: &ActiveThemeState, module_id: Option<&str>) -> ThemeTokenMap {
    let mut tokens = state.app_default_tokens.clone();
    tokens.extend(state.workspace_theme.tokens.clone());
    if let Some(module_id) = module_id {
        if let Some(module_defaults) = state.module_default_tokens.get(module_id) {
            tokens.extend(module_defaults.clone());
        }
        if let Some(module_theme) = state.module_themes.get(module_id) {
            tokens.extend(module_theme.tokens.clone());
        }
    }
    tokens
}

fn effective_module_tokens(state: &ActiveThemeState) -> BTreeMap<String, ThemeTokenMap> {
    let modules = state
        .module_default_tokens
        .keys()
        .chain(state.module_themes.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    modules
        .iter()
        .map(|module_id| (module_id.clone(), effective_tokens(state, Some(module_id))))
        .collect()
}

fn preview_effective_tokens(
    state: &mut ActiveThemeState,
    scope: &str,
    module_id: Option<&str>,
    preview_tokens: &ThemeTokenMap,
) -> Result<ThemeTokenMap, String> {
    state.validate()?;
    let mut tokens = effective_tokens(state, if scope == "module" { module_id } else { None });
    tokens.extend(preview_tokens.clone());
    validate_tokens(&tokens)?;
    Ok(tokens)
}

fn app_default_tokens() -> ThemeTokenMap {
    ThemeTokenMap::from([
        ("--background".to_string(), "#f6f7f4".to_string()),
        ("--foreground".to_string(), "#202521".to_string()),
        ("--card".to_string(), "#ffffff".to_string()),
        ("--card-foreground".to_string(), "#202521".to_string()),
        ("--popover".to_string(), "#ffffff".to_string()),
        ("--popover-foreground".to_string(), "#202521".to_string()),
        ("--primary".to_string(), "#335c4a".to_string()),
        ("--primary-foreground".to_string(), "#f8fbf7".to_string()),
        ("--secondary".to_string(), "#e8eee9".to_string()),
        ("--secondary-foreground".to_string(), "#26342d".to_string()),
        ("--muted".to_string(), "#edf0eb".to_string()),
        ("--muted-foreground".to_string(), "#66716a".to_string()),
        ("--accent".to_string(), "#e5edf4".to_string()),
        ("--accent-foreground".to_string(), "#213346".to_string()),
        ("--destructive".to_string(), "#9f3a3a".to_string()),
        (
            "--destructive-foreground".to_string(),
            "#fff7f7".to_string(),
        ),
        ("--border".to_string(), "#dfe4dc".to_string()),
        ("--input".to_string(), "#d9e0d7".to_string()),
        ("--ring".to_string(), "#6f927f".to_string()),
        ("--sage".to_string(), "#7e9d86".to_string()),
        ("--sage-foreground".to_string(), "#213528".to_string()),
        ("--soft-blue".to_string(), "#7d98ad".to_string()),
        ("--soft-blue-foreground".to_string(), "#203343".to_string()),
        ("--amber-note".to_string(), "#d7b46a".to_string()),
        ("--amber-note-foreground".to_string(), "#403112".to_string()),
        (
            "--shadow-soft".to_string(),
            "0 16px 40px rgb(39 50 42 / 0.08)".to_string(),
        ),
        (
            "--shadow-lifted".to_string(),
            "0 22px 55px rgb(39 50 42 / 0.12)".to_string(),
        ),
    ])
}

fn module_default_tokens() -> BTreeMap<String, ThemeTokenMap> {
    BTreeMap::from([
        (
            "habits".to_string(),
            ThemeTokenMap::from([
                (
                    "--habit-progress-height".to_string(),
                    "0.375rem".to_string(),
                ),
                ("--habit-streak-emphasis".to_string(), "#335c4a".to_string()),
                ("--habit-completed-state".to_string(), "#7e9d86".to_string()),
            ]),
        ),
        (
            "todos".to_string(),
            ThemeTokenMap::from([
                ("--todo-overdue-state".to_string(), "#9f3a3a".to_string()),
                (
                    "--todo-priority-emphasis".to_string(),
                    "#d7b46a".to_string(),
                ),
                ("--todo-completed-state".to_string(), "#7e9d86".to_string()),
            ]),
        ),
        (
            "contacts".to_string(),
            ThemeTokenMap::from([(
                "--contact-relationship-chip".to_string(),
                "#e5edf4".to_string(),
            )]),
        ),
    ])
}

fn theme_id_for(scope: &str, module_id: Option<&str>) -> String {
    match module_id {
        Some(module_id) => format!("{scope}-{module_id}"),
        None => scope.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::storage::current_timestamp_label;
    use std::path::PathBuf;

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-theme-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    #[test]
    fn accepts_static_css_theme_data() {
        assert!(ThemeService::validate_css_theme(
            ":root { --background: #fff; --foreground: rgb(0 0 0); }"
        )
        .is_ok());
    }

    #[test]
    fn rejects_executable_or_remote_css_patterns() {
        assert!(
            ThemeService::validate_css_theme("@import url('https://example.com/theme.css');")
                .is_err()
        );
        assert!(
            ThemeService::validate_css_theme(".x { background: url(javascript:alert(1)); }")
                .is_err()
        );
        assert!(ThemeService::validate_css_theme("<script>alert(1)</script>").is_err());
    }

    #[test]
    fn parses_allowlisted_css_tokens_and_rejects_selectors_or_unknown_tokens() {
        let tokens = ThemeService::parse_css_tokens(
            ":root { --background: #ffffff; --shadow-soft: 0 1px 2px rgb(0 0 0 / 0.1); }",
        )
        .expect("tokens");

        assert_eq!(
            tokens.get("--background").map(String::as_str),
            Some("#ffffff")
        );
        assert!(ThemeService::parse_css_tokens(".card { color: red; }").is_err());
        assert!(ThemeService::parse_css_tokens(":root { --not-real: red; }").is_err());
    }

    #[test]
    fn previews_applies_and_rolls_back_module_theme_tokens() {
        let vault_path = unique_temp_vault("apply");
        let source_path = vault_path.join("modules/notes/theme/css/calm.css");
        std::fs::create_dir_all(source_path.parent().expect("theme parent")).expect("theme folder");
        std::fs::write(&source_path, ":root { --primary: #123456; }").expect("theme");

        let preview = ThemeService::preview_theme_tokens(
            &vault_path,
            "module",
            Some("notes"),
            Path::new("modules/notes/theme/css/calm.css"),
        )
        .expect("preview");
        assert!(preview.safe);

        let state = ThemeService::apply_theme_tokens(
            &vault_path,
            "module",
            Some("notes"),
            Path::new("modules/notes/theme/css/calm.css"),
        )
        .expect("apply");
        assert!(state.module_themes.contains_key("notes"));
        assert_eq!(
            ThemeService::effective_tokens_for_module(&state, Some("notes"))
                .get("--primary")
                .map(String::as_str),
            Some("#123456")
        );

        let rolled_back =
            ThemeService::rollback_theme(&vault_path, "module", Some("notes")).expect("rollback");
        assert!(!rolled_back.module_themes.contains_key("notes"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn resolves_app_workspace_module_default_override_and_preview_precedence() {
        let vault_path = unique_temp_vault("precedence");
        let workspace_path = vault_path.join("themes/clean-slate/workspace.css");
        let module_path = vault_path.join("modules/todos/theme/css/priority.css");
        std::fs::create_dir_all(workspace_path.parent().expect("workspace parent"))
            .expect("workspace folder");
        std::fs::create_dir_all(module_path.parent().expect("module parent"))
            .expect("module folder");
        std::fs::write(&workspace_path, ":root { --primary: #111111; }").expect("workspace");
        std::fs::write(&module_path, ":root { --todo-priority-emphasis: #222222; }")
            .expect("module");

        let workspace =
            ThemeService::apply_theme_tokens(&vault_path, "workspace", None, &workspace_path)
                .expect("workspace apply");
        assert_eq!(
            ThemeService::effective_tokens_for_module(&workspace, Some("todos"))
                .get("--primary")
                .map(String::as_str),
            Some("#111111")
        );
        assert_eq!(
            ThemeService::effective_tokens_for_module(&workspace, Some("todos"))
                .get("--todo-priority-emphasis")
                .map(String::as_str),
            Some("#d7b46a")
        );

        let module = ThemeService::apply_theme_tokens(
            &vault_path,
            "module",
            Some("todos"),
            Path::new("modules/todos/theme/css/priority.css"),
        )
        .expect("module apply");
        assert_eq!(
            ThemeService::effective_tokens_for_module(&module, Some("todos"))
                .get("--todo-priority-emphasis")
                .map(String::as_str),
            Some("#222222")
        );

        std::fs::write(&module_path, ":root { --todo-priority-emphasis: #333333; }")
            .expect("preview");
        let preview = ThemeService::preview_theme_tokens(
            &vault_path,
            "module",
            Some("todos"),
            Path::new("modules/todos/theme/css/priority.css"),
        )
        .expect("preview");
        assert_eq!(
            preview
                .effective_tokens
                .get("--todo-priority-emphasis")
                .map(String::as_str),
            Some("#333333")
        );

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rejects_module_theme_sources_outside_module_theme_folders() {
        let vault_path = unique_temp_vault("module-path");
        let source_path = vault_path.join("modules/todos/unsafe.css");
        std::fs::create_dir_all(source_path.parent().expect("source parent"))
            .expect("source folder");
        std::fs::write(&source_path, ":root { --todo-overdue-state: #111111; }").expect("source");

        let error = ThemeService::preview_theme_tokens(
            &vault_path,
            "module",
            Some("todos"),
            Path::new("modules/todos/unsafe.css"),
        )
        .expect_err("rejected");
        assert!(error.contains("modules/todos/theme/json/"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
