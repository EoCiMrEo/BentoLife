use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    document_identity::DocumentIdentityService,
    document_metadata::{DocumentMetadata, DocumentMetadataService},
    layout_folder::LayoutFolderService,
    layout_metadata::LayoutMetadataService,
    markdown_document::MarkdownDocumentService,
    module_registry::ModuleRegistry,
    storage::{content_hash, generate_document_id, resolve_vault_relative_path, write_text_atomic},
    workspace_metadata::WorkspaceMetadataService,
    workspace_scanner::{ScannedDocument, WorkspaceScanner},
};

pub const DASHBOARD_INDEX_PATH: &str = "INDEX.md";

const MODULE_INDEXES: [(&str, &str, &str); 5] = [
    ("navigator", "Navigator", "modules/navigator/INDEX.md"),
    ("notes", "Notes", "modules/notes/INDEX.md"),
    ("todos", "Todos", "modules/todos/INDEX.md"),
    ("contacts", "Contacts", "modules/contacts/INDEX.md"),
    ("habits", "Habits", "modules/habits/INDEX.md"),
];

const CONTENT_MODULES: [(&str, &str, &str); 4] = [
    ("notes", "Notes", "note"),
    ("todos", "Todos", "todos"),
    ("contacts", "Contacts", "contact"),
    ("habits", "Habits", "habit"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardPinnedEntity {
    pub label: String,
    pub target: String,
    pub document_id: String,
    pub title: String,
    pub markdown_relative_path: String,
    pub entity_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardModuleSummary {
    pub module_id: String,
    pub display_name: String,
    pub status: String,
    pub entity_count: usize,
    pub index_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardHubDocument {
    pub document_id: Option<String>,
    pub markdown_relative_path: String,
    pub title: String,
    pub markdown_body: String,
    pub pinned_entities: Vec<DashboardPinnedEntity>,
    pub unresolved_pins: Vec<String>,
    pub module_summaries: Vec<DashboardModuleSummary>,
    pub warnings: Vec<String>,
}

pub struct DashboardService;

impl DashboardService {
    pub fn ensure_v3_vault_scaffold(vault_path: &Path) -> Result<(), String> {
        LayoutFolderService::create_or_repair(vault_path)?;
        WorkspaceMetadataService::write_bootstrap_files(vault_path)?;
        ensure_managed_index(
            vault_path,
            DASHBOARD_INDEX_PATH,
            default_dashboard_markdown(),
            "dashboard",
        )?;

        for (module_id, display_name, index_path) in MODULE_INDEXES {
            ensure_managed_index(
                vault_path,
                index_path,
                &format!(
                    "# {display_name}\n\nThis module dashboard summarizes BentoLife {module_id} entities.\n"
                ),
                module_id,
            )?;
            ensure_v3_module_folders(vault_path, module_id)?;
            ensure_module_contract(vault_path, module_id, display_name)?;
        }

        for (module_id, display_name, entity_type) in CONTENT_MODULES {
            ensure_v3_content_module(vault_path, module_id, display_name, entity_type)?;
        }
        ensure_system_module_index(
            vault_path,
            "modules/trash/INDEX.md",
            "# Trash\n\nDeleted records are stored internally under `.bentolifelayout/trash/` until explicitly restored or permanently removed.\n",
            "trash",
        )?;
        ensure_system_module_index(
            vault_path,
            "modules/archive/INDEX.md",
            "# Archive\n\nArchived records are stored internally under `.bentolifelayout/archive/` until explicitly restored.\n",
            "archive",
        )?;

        Ok(())
    }

    pub fn required_v3_paths(vault_path: &Path) -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();
        for (module_id, _, _) in CONTENT_MODULES {
            let module_root = vault_path.join("modules").join(module_id);
            paths.extend([
                module_root.join("INDEX.md"),
                module_root.join("MODULE.md"),
                module_root.join("module.schema.json"),
                module_root.join("data"),
                module_root.join("views"),
                module_root.join("templates"),
                module_root.join("theme/json"),
                module_root.join("theme/css"),
            ]);
        }
        paths.extend([
            vault_path.join("modules/trash/INDEX.md"),
            vault_path.join("modules/archive/INDEX.md"),
            vault_path.join(".bentolifelayout/trash"),
            vault_path.join(".bentolifelayout/archive"),
            vault_path.join(".bentolifelayout/imports/staged"),
            vault_path.join(".bentolifelayout/backups"),
        ]);
        paths
    }

    pub fn read_dashboard_hub(vault_path: &Path) -> Result<DashboardHubDocument, String> {
        Self::ensure_v3_vault_scaffold(vault_path)?;
        let scan = WorkspaceScanner::scan(vault_path)?;
        let root_path = resolve_vault_relative_path(vault_path, DASHBOARD_INDEX_PATH)?;
        let markdown = std::fs::read_to_string(&root_path)
            .map_err(|error| format!("Unable to read {}: {error}", root_path.display()))?;
        let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
        let document_id = DocumentIdentityService::find_identity_comment(&markdown)
            .map(|identity| identity.document_id);
        let markdown_body = DocumentIdentityService::remove_identity_comments(&parsed.body)
            .trim()
            .to_string();
        let pins = parse_pin_targets(&markdown_body);
        let mut pinned_entities = Vec::new();
        let mut unresolved_pins = Vec::new();

        for pin in pins {
            if let Some(document) = resolve_pin(&pin.target, &scan.documents) {
                if let Some(document_id) = &document.document_id {
                    pinned_entities.push(DashboardPinnedEntity {
                        label: pin.label,
                        target: pin.target,
                        document_id: document_id.clone(),
                        title: document.title.clone(),
                        markdown_relative_path: document.markdown_relative_path.clone(),
                        entity_type: document.document_type.clone(),
                    });
                }
            } else {
                unresolved_pins.push(pin.target);
            }
        }

        Ok(DashboardHubDocument {
            document_id,
            markdown_relative_path: DASHBOARD_INDEX_PATH.to_string(),
            title: markdown_title(&markdown_body, "Dashboard"),
            markdown_body,
            pinned_entities,
            unresolved_pins,
            module_summaries: module_summaries(&scan.documents),
            warnings: scan.issues.into_iter().map(|issue| issue.message).collect(),
        })
    }

    pub fn pin_dashboard_entity(
        vault_path: &Path,
        document_id: &str,
    ) -> Result<DashboardHubDocument, String> {
        Self::set_dashboard_entity_pin(vault_path, document_id, true)
    }

    pub fn unpin_dashboard_entity(
        vault_path: &Path,
        document_id: &str,
    ) -> Result<DashboardHubDocument, String> {
        Self::set_dashboard_entity_pin(vault_path, document_id, false)
    }

    fn set_dashboard_entity_pin(
        vault_path: &Path,
        document_id: &str,
        pinned: bool,
    ) -> Result<DashboardHubDocument, String> {
        Self::ensure_v3_vault_scaffold(vault_path)?;
        let scan = WorkspaceScanner::scan(vault_path)?;
        let document = scan
            .documents
            .iter()
            .find(|document| document.document_id.as_deref() == Some(document_id))
            .ok_or_else(|| "Pinned entity was not found in the workspace index.".to_string())?;
        let root_path = resolve_vault_relative_path(vault_path, DASHBOARD_INDEX_PATH)?;
        let markdown = std::fs::read_to_string(&root_path)
            .map_err(|error| format!("Unable to read {}: {error}", root_path.display()))?;
        let next_markdown = if pinned {
            ensure_dashboard_pin(&markdown, &document.title, &document.markdown_relative_path)
        } else {
            remove_dashboard_pin(
                &markdown,
                document_id,
                &document.title,
                &document.markdown_relative_path,
            )
        };
        if next_markdown != markdown {
            write_text_atomic(&root_path, &next_markdown)?;
        }
        Self::read_dashboard_hub(vault_path)
    }
}

fn ensure_v3_content_module(
    vault_path: &Path,
    module_id: &str,
    display_name: &str,
    entity_type: &str,
) -> Result<(), String> {
    let module_root = resolve_vault_relative_path(vault_path, &format!("modules/{module_id}"))?;
    for folder in ["data", "views", "templates", "theme/json", "theme/css"] {
        let path = module_root.join(folder);
        std::fs::create_dir_all(&path)
            .map_err(|error| format!("Unable to create {}: {error}", path.display()))?;
    }
    let handled_by_contract = module_contract(module_id, display_name).is_some();
    if !handled_by_contract {
        write_if_missing(
            &module_root.join("MODULE.md"),
            &format!(
                "# {display_name}\n\nV3 {display_name} records live under `modules/{module_id}/data/` and remain portable Markdown.\n"
            ),
        )?;
        write_if_missing(
            &module_root.join("module.schema.json"),
            &format!(
                "{{\n  \"schema_version\": 2,\n  \"module_id\": \"{module_id}\",\n  \"kind\": \"{}\",\n  \"entity_type\": \"{entity_type}\",\n  \"data_path\": \"modules/{module_id}/data\",\n  \"index_path\": \"modules/{module_id}/INDEX.md\",\n  \"default_view\": \"list\",\n  \"enabled_by_default\": {},\n  \"fields\": [],\n  \"renderers\": [],\n  \"validation\": {{ \"unknown_fields\": \"warn_and_preserve\", \"unknown_blocks\": \"generic\" }},\n  \"views\": [\"list\"],\n  \"widgets\": [],\n  \"theme\": {{}},\n  \"migration_version\": 1\n}}\n",
                if module_id == "notes" || module_id == "todos" {
                    "starter"
                } else {
                    "optional"
                },
                if module_id == "notes" || module_id == "todos" {
                    "true"
                } else {
                    "false"
                }
            ),
        )?;
    }
    Ok(())
}

fn ensure_system_module_index(
    vault_path: &Path,
    relative_path: &str,
    markdown: &str,
    document_type: &str,
) -> Result<DocumentMetadata, String> {
    ensure_managed_index(vault_path, relative_path, markdown, document_type)
}

fn write_if_missing(path: &Path, content: &str) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    write_text_atomic(path, content)
}

fn ensure_managed_index(
    vault_path: &Path,
    relative_path: &str,
    default_markdown: &str,
    document_type: &str,
) -> Result<DocumentMetadata, String> {
    let markdown_path = resolve_vault_relative_path(vault_path, relative_path)?;
    let existing_markdown = if markdown_path.is_file() {
        std::fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?
    } else {
        default_markdown.to_string()
    };
    let document_id = DocumentIdentityService::find_identity_comment(&existing_markdown)
        .map(|identity| identity.document_id)
        .unwrap_or_else(|| generate_document_id(relative_path));
    let mut metadata =
        DocumentMetadataService::read(vault_path, &document_id).unwrap_or_else(|_| {
            DocumentMetadataService::create_default_with_type(
                &document_id,
                relative_path,
                &existing_markdown,
                document_type,
            )
            .expect("default index metadata is valid")
        });
    metadata.document_type = document_type.to_string();
    metadata.current_path = relative_path.to_string();
    let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
        &existing_markdown,
        &document_id,
        &metadata.frontmatter_contract.required_value,
    );
    metadata.content_hash = content_hash(&managed_markdown);
    write_text_atomic(&markdown_path, &managed_markdown)?;
    DocumentMetadataService::write(vault_path, &metadata)?;
    if LayoutMetadataService::read(vault_path, &document_id).is_err() {
        let layout =
            LayoutMetadataService::generate_from_markdown(&document_id, &managed_markdown)?;
        LayoutMetadataService::write(vault_path, &layout)?;
    }
    let documents = DocumentMetadataService::list(vault_path)?;
    let index = WorkspaceMetadataService::rebuild_index_from_documents(&documents)?;
    WorkspaceMetadataService::write_index(vault_path, &index)?;
    WorkspaceMetadataService::register_document(vault_path, &metadata)?;
    Ok(metadata)
}

fn ensure_v3_module_folders(vault_path: &Path, module_id: &str) -> Result<(), String> {
    for folder in ["data", "views", "templates", "theme/json", "theme/css"] {
        let path =
            resolve_vault_relative_path(vault_path, &format!("modules/{module_id}/{folder}"))?;
        std::fs::create_dir_all(&path)
            .map_err(|error| format!("Unable to create {}: {error}", path.display()))?;
    }
    Ok(())
}

fn ensure_module_contract(
    vault_path: &Path,
    module_id: &str,
    display_name: &str,
) -> Result<(), String> {
    let Some((module_doc, schema)) = module_contract(module_id, display_name) else {
        return Ok(());
    };

    for (relative_path, body) in [
        (format!("modules/{module_id}/MODULE.md"), module_doc),
        (format!("modules/{module_id}/module.schema.json"), schema),
    ] {
        let path = resolve_vault_relative_path(vault_path, &relative_path)?;
        if !path.is_file() {
            write_text_atomic(&path, body)?;
        }
    }

    Ok(())
}

fn module_contract(module_id: &str, display_name: &str) -> Option<(&'static str, &'static str)> {
    match module_id {
        "notes" => Some((NOTES_MODULE_DOC, NOTES_SCHEMA)),
        "todos" => Some((TODOS_MODULE_DOC, TODOS_SCHEMA)),
        "contacts" => Some((CONTACTS_MODULE_DOC, CONTACTS_SCHEMA)),
        "habits" => Some((HABITS_MODULE_DOC, HABITS_SCHEMA)),
        _ => {
            let _ = display_name;
            None
        }
    }
}

const NOTES_MODULE_DOC: &str = r#"# Notes

- Module ID: notes
- Module kind: starter
- Entity type: note
- Data folder: modules/notes/data
- Default views: cards, list, focused entity

Notes are core BentoLife Markdown entities. They support headings, paragraphs, lists, checklists, code blocks, tags, and relationships.

## Safety

This module schema is data only. Templates, views, and theme files cannot register React components, run scripts, load remote code, or inject raw runtime CSS outside the approved token subset.
"#;

const NOTES_SCHEMA: &str = include_str!("../../../../schemas/modules/notes.schema.json");

const TODOS_MODULE_DOC: &str = r#"# Todos

- Module ID: todos
- Module kind: starter
- Entity type: todos
- Data folder: modules/todos/data
- Default views: cards, list, focused entity

Todos are Markdown task entities. App-created todos live in `modules/todos/data/<todos>.md`; checkbox body content, due dates, tags, relationships, unknown fields, and external Markdown edits remain recoverable local content.

## Authoring Example

```md
# Prepare launch notes

- Status: Open
- Priority: High
- Due: 2026-06-15
- Tags: launch, writing
- Relationships: [[Note:Launch]]

## Checklist

- [ ] Draft notes
- [ ] Review with CPO
```

## Safety

This module schema is data only. Templates, views, widget declarations, and theme files cannot register React components, run scripts, load remote code, or inject raw runtime CSS outside the approved token subset.
"#;

const TODOS_SCHEMA: &str = include_str!("../../../../schemas/modules/todos.schema.json");

const CONTACTS_MODULE_DOC: &str = r#"# Contacts

- Module ID: contacts
- Module kind: optional
- Entity type: contact
- Data folder: modules/contacts/data
- Default views: cards, list, focused entity

Contacts are long-form Markdown entities. App-created contacts live in `modules/contacts/data/<contact>.md`, but user-authored meeting notes, relationship context, backlinks, and unsupported sections remain part of the Markdown document.

## Authoring Example

```md
# Mina Park

- Relationship: Collaborator
- Organization: Studio North
- Email: mina@example.com
- Tags: design, friend
- Relationships: [[Project:Launch]]
- Attachments: assets/mina-card.pdf

## Notes

Met during the launch review. Follow up about the visual QA checklist.
```

## Safety

This module schema is data only. Templates, views, and theme files cannot register React components, run scripts, load remote code, or inject raw runtime CSS outside the approved token subset.
"#;

const CONTACTS_SCHEMA: &str = include_str!("../../../../schemas/modules/contacts.schema.json");

const HABITS_MODULE_DOC: &str = r#"# Habits

- Module ID: habits
- Module kind: optional
- Entity type: habit
- Data folder: modules/habits/data
- Default views: cards, list, focused entity

Habits are one-file Markdown entities. App-created habits live in `modules/habits/data/<habit>.md`; check-ins stay inside that habit file under a `## Check-ins` section as local `YYYY-MM-DD` dates.

## Authoring Example

```md
# Morning Walk

- Frequency: daily
- Target: 20 minutes
- Tags: health, energy
- Relationships: [[Contact:Mina Park]]

## Notes

Best after coffee.

## Check-ins

- 2026-06-01
- 2026-06-02
```

## Safety

This module schema is data only. Templates, views, and theme files cannot register React components, run scripts, load remote code, or inject raw runtime CSS outside the approved token subset.
"#;

const HABITS_SCHEMA: &str = include_str!("../../../../schemas/modules/habits.schema.json");

fn default_dashboard_markdown() -> &'static str {
    "# Today\n\n## Pinned Entities\n\nAdd links to important BentoLife entities here.\n\n## Modules\n\nCore module summaries are generated by BentoLife.\n"
}

fn ensure_dashboard_pin(markdown: &str, title: &str, target: &str) -> String {
    if parse_pin_targets(markdown)
        .iter()
        .any(|pin| normalize_target(&pin.target) == normalize_target(target))
    {
        return ensure_trailing_newline(markdown);
    }

    let pin_line = format!("- [{title}]({target})");
    let mut lines: Vec<String> = markdown.lines().map(str::to_string).collect();
    let identity_index = lines
        .iter()
        .position(|line| line.trim_start().starts_with("<!-- bentolife:document_id="));
    if let Some(heading_index) = lines.iter().position(|line| {
        parse_heading(line.trim()).is_some_and(|(_, heading)| {
            let heading = heading.to_lowercase();
            heading == "pinned" || heading == "pinned entities"
        })
    }) {
        let mut insert_index = heading_index + 1;
        while insert_index < lines.len() && lines[insert_index].trim().is_empty() {
            insert_index += 1;
        }
        lines.insert(insert_index, pin_line);
        if insert_index == heading_index + 1 {
            lines.insert(insert_index, String::new());
        }
        return ensure_trailing_newline(&lines.join("\n"));
    }

    let insert_index = identity_index.unwrap_or(lines.len());
    let section = vec![
        String::new(),
        "## Pinned Entities".to_string(),
        String::new(),
        pin_line,
        String::new(),
    ];
    lines.splice(insert_index..insert_index, section);
    ensure_trailing_newline(&lines.join("\n"))
}

fn remove_dashboard_pin(markdown: &str, document_id: &str, title: &str, target: &str) -> String {
    let mut in_pinned_section = false;
    let mut pinned_level = 0usize;
    let mut lines = Vec::new();

    for line in markdown.lines() {
        if let Some((level, heading)) = parse_heading(line.trim()) {
            if in_pinned_section && level <= pinned_level {
                in_pinned_section = false;
            }
            let heading = heading.to_lowercase();
            if heading == "pinned" || heading == "pinned entities" {
                in_pinned_section = true;
                pinned_level = level;
            }
            lines.push(line.to_string());
            continue;
        }
        if in_pinned_section && line_targets_entity(line, document_id, title, target) {
            continue;
        }
        lines.push(line.to_string());
    }

    ensure_trailing_newline(&lines.join("\n"))
}

fn line_targets_entity(line: &str, document_id: &str, title: &str, target: &str) -> bool {
    let normalized_title = normalize_target(title);
    let normalized_target = normalize_target(target);
    let normalized_document_id = normalize_target(document_id);
    let pins = markdown_links(line).into_iter().chain(wiki_links(line));
    pins.into_iter().any(|pin| {
        let normalized_pin_target = normalize_target(&pin.target);
        normalized_pin_target == normalized_target
            || normalized_pin_target == normalized_document_id
            || normalize_target(&strip_entity_prefix(&pin.target)) == normalized_title
    })
}

fn ensure_trailing_newline(markdown: &str) -> String {
    format!("{}\n", markdown.trim_end())
}

struct PinTarget {
    label: String,
    target: String,
}

fn parse_pin_targets(markdown_body: &str) -> Vec<PinTarget> {
    let mut pins = Vec::new();
    let mut in_pinned_section = false;
    let mut pinned_level = 0usize;

    for line in markdown_body.lines() {
        let trimmed = line.trim();
        if let Some((level, heading)) = parse_heading(trimmed) {
            let heading = heading.to_lowercase();
            if in_pinned_section && level <= pinned_level {
                in_pinned_section = false;
            }
            if heading == "pinned" || heading == "pinned entities" {
                in_pinned_section = true;
                pinned_level = level;
            }
            continue;
        }
        if !in_pinned_section {
            continue;
        }
        pins.extend(markdown_links(trimmed));
        pins.extend(wiki_links(trimmed));
    }

    pins
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    line.get(hashes..)
        .and_then(|rest| rest.strip_prefix(' '))
        .map(|heading| (hashes, heading.trim()))
}

fn markdown_links(line: &str) -> Vec<PinTarget> {
    let mut pins = Vec::new();
    let mut rest = line;
    while let Some(label_start) = rest.find('[') {
        let after_label_start = &rest[label_start + 1..];
        let Some(label_end) = after_label_start.find(']') else {
            break;
        };
        let after_label = &after_label_start[label_end + 1..];
        if !after_label.starts_with('(') {
            rest = after_label;
            continue;
        }
        let Some(target_end) = after_label[1..].find(')') else {
            break;
        };
        let label = after_label_start[..label_end].trim();
        let target = after_label[1..target_end + 1].trim();
        if !label.is_empty() && !target.is_empty() {
            pins.push(PinTarget {
                label: label.to_string(),
                target: target.to_string(),
            });
        }
        rest = &after_label[target_end + 2..];
    }
    pins
}

fn wiki_links(line: &str) -> Vec<PinTarget> {
    let mut pins = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("[[") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("]]") else {
            break;
        };
        let target = after_start[..end].trim();
        if !target.is_empty() {
            pins.push(PinTarget {
                label: target.to_string(),
                target: target.to_string(),
            });
        }
        rest = &after_start[end + 2..];
    }
    pins
}

fn resolve_pin<'a>(target: &str, documents: &'a [ScannedDocument]) -> Option<&'a ScannedDocument> {
    let normalized_target = normalize_target(target);
    documents.iter().find(|document| {
        document.document_id.is_some()
            && normalize_target(&document.markdown_relative_path) == normalized_target
            || normalize_target(&document.title) == normalized_target
            || document
                .document_id
                .as_ref()
                .is_some_and(|document_id| normalize_target(document_id) == normalized_target)
            || normalize_target(&strip_entity_prefix(target)) == normalize_target(&document.title)
    })
}

fn strip_entity_prefix(target: &str) -> String {
    target
        .split_once(':')
        .map(|(_, value)| value.trim().to_string())
        .unwrap_or_else(|| target.to_string())
}

fn normalize_target(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("./")
        .replace('\\', "/")
        .to_lowercase()
}

fn module_summaries(documents: &[ScannedDocument]) -> Vec<DashboardModuleSummary> {
    ModuleRegistry::core_modules()
        .into_iter()
        .map(|module| {
            let entity_count = documents
                .iter()
                .filter(|document| {
                    document.document_id.is_some()
                        && document.markdown_relative_path != module.default_path
                        && document.document_type == module.document_type
                })
                .count();
            DashboardModuleSummary {
                module_id: module.id,
                display_name: module.display_name,
                status: module.implementation_status,
                entity_count,
                index_path: module.default_path,
            }
        })
        .collect()
}

fn markdown_title(markdown_body: &str, fallback: &str) -> String {
    markdown_body
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("# ")
                .map(|title| title.trim().to_string())
        })
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        module_schema::ModuleSchema, notes::NotesService, storage::current_timestamp_label,
    };
    use std::path::PathBuf;

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-dashboard-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    fn product_module_schemas() -> [(&'static str, &'static str); 4] {
        [
            ("notes", NOTES_SCHEMA),
            ("todos", TODOS_SCHEMA),
            ("contacts", CONTACTS_SCHEMA),
            ("habits", HABITS_SCHEMA),
        ]
    }

    #[test]
    fn product_module_schema_files_parse_through_runtime_contract() {
        for (module_id, schema_json) in product_module_schemas() {
            let schema: ModuleSchema = serde_json::from_str(schema_json)
                .unwrap_or_else(|error| panic!("{module_id} schema should parse: {error}"));

            assert_eq!(schema.schema_version, 2);
            assert_eq!(schema.module_id, module_id);
            assert!(
                schema.diagnostics().is_empty(),
                "{module_id} schema should not emit diagnostics: {:?}",
                schema.diagnostics()
            );
        }
    }

    #[test]
    fn scaffold_creates_root_module_indexes_and_theme_folders_without_overwriting() {
        let vault_path = unique_temp_vault("scaffold");
        std::fs::create_dir_all(&vault_path).expect("vault");
        std::fs::write(
            vault_path.join(DASHBOARD_INDEX_PATH),
            "# My Hub\n\n## Pinned\n",
        )
        .expect("existing hub");

        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");

        assert!(
            std::fs::read_to_string(vault_path.join(DASHBOARD_INDEX_PATH))
                .expect("hub")
                .contains("# My Hub")
        );
        assert!(vault_path.join("modules/notes/INDEX.md").is_file());
        assert!(vault_path.join("modules/notes/data").is_dir());
        assert!(vault_path.join("modules/notes/MODULE.md").is_file());
        assert!(vault_path
            .join("modules/notes/module.schema.json")
            .is_file());
        assert!(vault_path.join("modules/notes/theme/json").is_dir());
        assert!(vault_path.join("modules/notes/theme/css").is_dir());
        assert!(vault_path.join("modules/todos/data").is_dir());
        assert!(vault_path.join("modules/todos/MODULE.md").is_file());
        for (module_id, expected_schema) in product_module_schemas() {
            let scaffolded_schema = std::fs::read_to_string(
                vault_path.join(format!("modules/{module_id}/module.schema.json")),
            )
            .unwrap_or_else(|error| panic!("{module_id} schema should be scaffolded: {error}"));
            assert_eq!(
                scaffolded_schema, expected_schema,
                "{module_id} scaffold should match schemas/modules source"
            );
        }
        assert!(vault_path.join("modules/trash/INDEX.md").is_file());
        assert!(vault_path.join("modules/archive/INDEX.md").is_file());
        assert!(vault_path.join("modules/contacts/data").is_dir());
        assert!(vault_path.join("modules/contacts/MODULE.md").is_file());
        assert!(vault_path
            .join("modules/contacts/module.schema.json")
            .is_file());
        assert!(vault_path.join("modules/habits/data").is_dir());
        assert!(vault_path.join("modules/habits/MODULE.md").is_file());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn dashboard_resolves_known_pins_and_reports_unknown_pins() {
        let vault_path = unique_temp_vault("pins");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        let note = NotesService::create_note(&vault_path, "Daily Note", None).expect("note");
        std::fs::write(
            vault_path.join(DASHBOARD_INDEX_PATH),
            format!(
                "# Today\n\n## Pinned Entities\n\n- [Daily](modules/notes/data/daily-note.md)\n- [[Missing]]\n<!-- bentolife:document_id=bl_doc_root_index -->\n",
            ),
        )
        .expect("hub");
        let _ = note;

        let hub = DashboardService::read_dashboard_hub(&vault_path).expect("hub");

        assert_eq!(hub.pinned_entities.len(), 1);
        assert_eq!(hub.pinned_entities[0].title, "Daily Note");
        assert_eq!(hub.unresolved_pins, vec!["Missing".to_string()]);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn dashboard_pin_commands_persist_without_mutating_note_body() {
        let vault_path = unique_temp_vault("pin-command");
        DashboardService::ensure_v3_vault_scaffold(&vault_path).expect("scaffold");
        let note = NotesService::create_note(
            &vault_path,
            "Daily Note",
            Some("# Daily Note\n\nBody".to_string()),
        )
        .expect("note");

        let pinned =
            DashboardService::pin_dashboard_entity(&vault_path, &note.document_id).expect("pin");

        assert_eq!(pinned.pinned_entities.len(), 1);
        assert_eq!(pinned.pinned_entities[0].document_id, note.document_id);
        let note_after_pin =
            NotesService::read_note(&vault_path, &note.document_id).expect("note after pin");
        assert_eq!(note_after_pin.markdown_body, "# Daily Note\n\nBody");

        let reread = DashboardService::read_dashboard_hub(&vault_path).expect("reread");
        assert_eq!(reread.pinned_entities.len(), 1);

        let unpinned = DashboardService::unpin_dashboard_entity(&vault_path, &note.document_id)
            .expect("unpin");
        assert!(unpinned.pinned_entities.is_empty());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
