use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::markdown_parser::{MarkdownParser, ParsedEntityContract};
use super::module_schema::apply_schema_descriptors;
use super::{
    document_identity::DocumentIdentityService,
    document_metadata::{DocumentMetadata, DocumentMetadataService},
    layout_folder::LayoutFolderService,
    layout_metadata::{LayoutMetadata, LayoutMetadataService},
    markdown_document::MarkdownDocumentService,
    storage::{
        content_hash, current_timestamp_label, generate_document_id, resolve_vault_relative_path,
        write_text_atomic,
    },
    workspace_metadata::WorkspaceMetadataService,
    workspace_scanner::{ScannedDocumentStatus, WorkspaceScanner},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoSummary {
    pub document_id: String,
    pub title: String,
    pub markdown_relative_path: String,
    pub excerpt: String,
    pub is_completed: bool,
    pub status: ScannedDocumentStatus,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoDocument {
    pub document_id: String,
    pub title: String,
    pub markdown_relative_path: String,
    pub markdown_body: String,
    pub parsed_entity: ParsedEntityContract,
    pub schema_warnings: Vec<String>,
    pub document_metadata: DocumentMetadata,
    pub layout_metadata: Option<LayoutMetadata>,
}

pub struct TodoService;

impl TodoService {
    pub fn list_todos(vault_path: &Path) -> Result<Vec<TodoSummary>, String> {
        let scan = WorkspaceScanner::scan(vault_path)?;
        let mut todos = Vec::new();

        for document in scan.documents {
            let is_todo = is_todo_data_markdown(&document.markdown_relative_path);
            let Some(document_id) = document.document_id else {
                continue;
            };

            if !is_todo {
                continue;
            }

            let metadata = DocumentMetadataService::read(vault_path, &document_id).ok();
            let parsed = MarkdownParser::parse(&document.markdown_body);

            let is_completed = parsed
                .fields
                .get("status")
                .is_some_and(|status| is_completed_status(status));

            todos.push(TodoSummary {
                document_id,
                title: document.title,
                markdown_relative_path: document.markdown_relative_path,
                excerpt: markdown_excerpt(&document.markdown_body),
                is_completed,
                status: document.status,
                updated_at: metadata.map(|metadata| metadata.updated_at),
            });
        }

        todos.sort_by_key(|todo| todo.title.to_lowercase());
        Ok(todos)
    }

    pub fn read_todo(vault_path: &Path, document_id: &str) -> Result<TodoDocument, String> {
        let metadata = DocumentMetadataService::read(vault_path, document_id)?;
        validate_todo_content_path(&metadata.current_path)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
        let markdown = std::fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
        let markdown_body = DocumentIdentityService::remove_identity_comments(&parsed.body)
            .trim()
            .to_string();
        let layout_metadata = LayoutMetadataService::read(vault_path, document_id).ok();

        let mut parsed_entity = MarkdownParser::parse(&markdown_body);
        parsed_entity.module_id = Some("todos".to_string());
        parsed_entity.entity_type = Some("todos".to_string());
        parsed_entity.path = metadata.current_path.clone();
        parsed_entity.content_hash = metadata.content_hash.clone();
        let schema_warnings = apply_schema_descriptors(
            vault_path,
            "modules/todos/module.schema.json",
            &mut parsed_entity,
        )?;

        Ok(TodoDocument {
            document_id: document_id.to_string(),
            title: markdown_title(&markdown_body, &metadata.current_path),
            markdown_relative_path: metadata.current_path.clone(),
            markdown_body,
            parsed_entity,
            schema_warnings,
            document_metadata: metadata,
            layout_metadata,
        })
    }

    pub fn create_todo(
        vault_path: &Path,
        title: &str,
        markdown_body: Option<String>,
    ) -> Result<TodoDocument, String> {
        let title = clean_title(title);
        let markdown_relative_path = unique_todo_path(vault_path, &title)?;
        validate_todo_content_path(&markdown_relative_path)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &markdown_relative_path)?;
        if markdown_path.exists() {
            return Err(format!(
                "A todos already exists at {markdown_relative_path}."
            ));
        }

        LayoutFolderService::create_or_repair(vault_path)?;
        WorkspaceMetadataService::write_bootstrap_files(vault_path)?;

        let body = normalize_todo_body(&title, markdown_body.as_deref().unwrap_or(""));
        let document_id = generate_document_id(&markdown_relative_path);
        let mut document_metadata = DocumentMetadataService::create_default_with_type(
            &document_id,
            &markdown_relative_path,
            &body,
            "todos",
        )?;
        let layout_metadata = LayoutMetadataService::create_default(&document_id)?;
        let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
            &body,
            &document_id,
            &document_metadata.frontmatter_contract.required_value,
        );
        document_metadata.content_hash = content_hash(&managed_markdown);

        write_text_atomic(&markdown_path, &managed_markdown)?;
        DocumentMetadataService::write(vault_path, &document_metadata)?;
        LayoutMetadataService::write(vault_path, &layout_metadata)?;
        rebuild_and_register(vault_path, &document_metadata)?;

        Self::read_todo(vault_path, &document_id)
    }

    pub fn update_todo(
        vault_path: &Path,
        document_id: &str,
        markdown_body: String,
    ) -> Result<TodoDocument, String> {
        let mut metadata = DocumentMetadataService::read(vault_path, document_id)?;
        metadata.document_type = "todos".to_string();
        validate_todo_content_path(&metadata.current_path)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
        reject_stale_todo_write(&markdown_path, &metadata.content_hash)?;
        let body = ensure_title_fallback(&markdown_body, &metadata.current_path);
        let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
            &body,
            document_id,
            &metadata.frontmatter_contract.required_value,
        );

        metadata.content_hash = content_hash(&managed_markdown);
        metadata.updated_at = current_timestamp_label();
        write_text_atomic(&markdown_path, &managed_markdown)?;
        DocumentMetadataService::write(vault_path, &metadata)?;
        rebuild_and_register(vault_path, &metadata)?;

        Self::read_todo(vault_path, document_id)
    }

    pub fn rename_todo(
        vault_path: &Path,
        document_id: &str,
        new_title: &str,
    ) -> Result<TodoDocument, String> {
        let mut todos = Self::read_todo(vault_path, document_id)?;
        let mut metadata = todos.document_metadata.clone();
        metadata.document_type = "todos".to_string();

        let new_title = clean_title(new_title);
        let new_relative_path = todo_path_for_title(&new_title);
        let current_relative_path = metadata.current_path.clone();
        validate_todo_content_path(&current_relative_path)?;
        validate_todo_content_path(&new_relative_path)?;
        let current_path = resolve_vault_relative_path(vault_path, &current_relative_path)?;
        let new_path = resolve_vault_relative_path(vault_path, &new_relative_path)?;

        if new_relative_path != current_relative_path && new_path.exists() {
            return Err(format!("A todos already exists at {new_relative_path}."));
        }

        todos.markdown_body = replace_or_insert_h1(&todos.markdown_body, &new_title);
        let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
            &todos.markdown_body,
            document_id,
            &metadata.frontmatter_contract.required_value,
        );

        if new_relative_path != current_relative_path {
            if let Some(parent) = new_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("Unable to create {}: {error}", parent.display()))?;
            }
            std::fs::rename(&current_path, &new_path).map_err(|error| {
                format!("Unable to rename todos to {new_relative_path}: {error}")
            })?;
            if !metadata.previous_paths.contains(&current_relative_path) {
                metadata.previous_paths.push(current_relative_path);
            }
            metadata.current_path = new_relative_path;
        }

        metadata.content_hash = content_hash(&managed_markdown);
        metadata.updated_at = current_timestamp_label();
        write_text_atomic(&new_path, &managed_markdown)?;
        DocumentMetadataService::write(vault_path, &metadata)?;
        rebuild_and_register(vault_path, &metadata)?;

        Self::read_todo(vault_path, document_id)
    }
}

fn reject_stale_todo_write(markdown_path: &Path, expected_hash: &str) -> Result<(), String> {
    let current_markdown = std::fs::read_to_string(markdown_path).map_err(|error| {
        format!(
            "Unable to read {} before writing: {error}",
            markdown_path.display()
        )
    })?;
    let current_hash = content_hash(&current_markdown);
    if current_hash != expected_hash {
        return Err("Todos was changed outside BentoLife. Rescan or reopen the task before saving so external Markdown edits are preserved.".to_string());
    }
    Ok(())
}

fn rebuild_and_register(vault_path: &Path, metadata: &DocumentMetadata) -> Result<(), String> {
    let documents = DocumentMetadataService::list(vault_path)?;
    let index = WorkspaceMetadataService::rebuild_index_from_documents(&documents)?;
    WorkspaceMetadataService::write_index(vault_path, &index)?;
    WorkspaceMetadataService::register_document(vault_path, metadata)
}

fn clean_title(title: &str) -> String {
    let cleaned = title.trim();
    if cleaned.is_empty() {
        "Untitled task".to_string()
    } else {
        cleaned.to_string()
    }
}

fn todo_path_for_title(title: &str) -> String {
    format!("modules/todos/data/{}.md", slugify(title))
}

fn validate_todo_content_path(path: &str) -> Result<(), String> {
    if !is_todo_data_markdown(path) || path.contains(".bentolifelayout") {
        return Err("V3 todos must live under modules/todos/data/ as Markdown files.".to_string());
    }
    Ok(())
}

fn is_todo_data_markdown(path: &str) -> bool {
    path.starts_with("modules/todos/data/") && path.ends_with(".md")
}

fn unique_todo_path(vault_path: &Path, title: &str) -> Result<String, String> {
    let base_slug = slugify(title);
    for index in 0..100 {
        let candidate = if index == 0 {
            format!("modules/todos/data/{base_slug}.md")
        } else {
            format!("modules/todos/data/{base_slug}-{index}.md")
        };
        if !resolve_vault_relative_path(vault_path, &candidate)?.exists() {
            return Ok(candidate);
        }
    }
    Err("Unable to find an available todos filename.".to_string())
}

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for character in title.trim().to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "untitled-task".to_string()
    } else {
        slug
    }
}

fn normalize_todo_body(title: &str, markdown_body: &str) -> String {
    let body = markdown_body.trim();
    if body.is_empty() {
        format!("# {title}\n\nStatus: Not started\nPriority: Medium\n")
    } else if body.lines().any(|line| line.trim_start().starts_with("# ")) {
        format!("{body}\n")
    } else {
        format!("# {title}\n\n{body}\n")
    }
}

fn ensure_title_fallback(markdown_body: &str, current_path: &str) -> String {
    let body = markdown_body.trim();
    if body.is_empty() {
        format!("# {}\n", fallback_title_from_path(current_path))
    } else {
        format!("{body}\n")
    }
}

fn replace_or_insert_h1(markdown_body: &str, title: &str) -> String {
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in markdown_body.lines() {
        if !replaced && line.trim_start().starts_with("# ") {
            lines.push(format!("# {title}"));
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !replaced {
        lines.insert(0, format!("# {title}"));
    }

    format!("{}\n", lines.join("\n").trim())
}

fn markdown_title(markdown_body: &str, markdown_relative_path: &str) -> String {
    markdown_body
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("# ")
                .map(|title| title.trim().to_string())
        })
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| fallback_title_from_path(markdown_relative_path))
}

fn fallback_title_from_path(path: &str) -> String {
    PathBuf::from(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Untitled task")
        .replace('-', " ")
}

fn markdown_excerpt(markdown_body: &str) -> String {
    markdown_body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("- ["))
        .unwrap_or("No preview yet.")
        .chars()
        .take(140)
        .collect()
}

fn is_completed_status(status: &str) -> bool {
    matches!(status.trim().to_lowercase().as_str(), "done" | "completed")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-todos-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    #[test]
    fn scaffolded_todos_index_is_not_listed_as_a_todo_record() {
        let vault_path = unique_temp_vault("scaffold-index");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");

        let todos = TodoService::list_todos(&vault_path).expect("todos list");

        assert!(todos.is_empty());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn list_todos_only_exposes_data_folder_markdown_records() {
        let vault_path = unique_temp_vault("data-only");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        let created =
            TodoService::create_todo(&vault_path, "Launch Task", None).expect("todos created");

        let todos = TodoService::list_todos(&vault_path).expect("todos list");

        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].document_id, created.document_id);
        assert!(todos[0]
            .markdown_relative_path
            .starts_with("modules/todos/data/"));
        assert!(TodoService::read_todo(&vault_path, &created.document_id).is_ok());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn creates_reads_updates_and_renames_todo_without_changing_identity() {
        let vault_path = unique_temp_vault("crud");
        let created = TodoService::create_todo(
            &vault_path,
            "Daily Plan",
            Some("## Morning\n\n- [ ] Tea".to_string()),
        )
        .expect("todos created");
        let document_id = created.document_id.clone();

        let updated = TodoService::update_todo(
            &vault_path,
            &document_id,
            "# Daily Plan\n\n## Afternoon\n\nParagraph".to_string(),
        )
        .expect("todos updated");
        assert_eq!(updated.document_id, document_id);

        let renamed = TodoService::rename_todo(&vault_path, &document_id, "Renamed Plan")
            .expect("todos renamed");
        assert_eq!(renamed.document_id, document_id);
        assert_eq!(
            renamed.markdown_relative_path,
            "modules/todos/data/renamed-plan.md"
        );
        assert!(renamed.markdown_body.contains("# Renamed Plan"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn parses_schema_fields_and_checklists() {
        let vault_path = unique_temp_vault("parse");
        let markdown = "
# Trip Todos

Status: In Progress
Priority: High
Due date: Tomorrow

## Items

- [ ] Book flight
- [x] Book hotel
";
        let created =
            TodoService::create_todo(&vault_path, "Trip Todos", Some(markdown.to_string()))
                .unwrap();
        let entity = created.parsed_entity;

        assert_eq!(
            entity.fields.get("status").map(|s| s.as_str()),
            Some("In Progress")
        );
        assert_eq!(
            entity.fields.get("priority").map(|s| s.as_str()),
            Some("High")
        );
        assert_eq!(
            entity.fields.get("due date").map(|s| s.as_str()),
            Some("Tomorrow")
        );

        let has_checklist = entity.blocks.iter().any(|b| {
            if let crate::domain::markdown_parser::MarkdownBlock::Checklist { items } = b {
                items.len() == 2 && !items[0].checked && items[1].checked
            } else {
                false
            }
        });
        assert!(has_checklist, "Checklist should be parsed correctly");

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn checklist_items_do_not_complete_parent_todo_summary() {
        let vault_path = unique_temp_vault("completion");
        let first_checked = TodoService::create_todo(
            &vault_path,
            "Checklist Parent",
            Some(
                "# Checklist Parent\n\nStatus: In progress\nPriority: Medium\n\n- [x] First item\n- [ ] Second item\n"
                    .to_string(),
            ),
        )
        .expect("todo created");
        let all_checked = TodoService::create_todo(
            &vault_path,
            "All Checked Parent",
            Some(
                "# All Checked Parent\n\nStatus: In progress\nPriority: Medium\n\n- [x] First item\n- [x] Second item\n"
                    .to_string(),
            ),
        )
        .expect("todo created");
        let done = TodoService::create_todo(
            &vault_path,
            "Done Parent",
            Some("# Done Parent\n\nStatus: Done\nPriority: Medium\n\n- [ ] Item\n".to_string()),
        )
        .expect("todo created");
        let completed = TodoService::create_todo(
            &vault_path,
            "Completed Parent",
            Some(
                "# Completed Parent\n\nStatus: Completed\nPriority: Medium\n\n- [ ] Item\n"
                    .to_string(),
            ),
        )
        .expect("todo created");

        let summaries = TodoService::list_todos(&vault_path).expect("todos list");
        let summary_for = |document_id: &str| {
            summaries
                .iter()
                .find(|summary| summary.document_id == document_id)
                .expect("summary exists")
        };

        assert!(!summary_for(&first_checked.document_id).is_completed);
        assert!(!summary_for(&all_checked.document_id).is_completed);
        assert!(summary_for(&done.document_id).is_completed);
        assert!(summary_for(&completed.document_id).is_completed);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rejects_stale_todo_writes_after_external_edit() {
        let vault_path = unique_temp_vault("stale");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        let created = TodoService::create_todo(&vault_path, "External Task", None).expect("todos");
        let path = vault_path.join(&created.markdown_relative_path);
        let markdown = std::fs::read_to_string(&path).expect("todos markdown");
        std::fs::write(&path, format!("{markdown}\nExternal editor change\n"))
            .expect("external edit");

        let result = TodoService::update_todo(
            &vault_path,
            &created.document_id,
            "# External Task\n\n- [ ] External Task".to_string(),
        );

        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn emits_table_unknown_blocks_and_schema_warnings() {
        let vault_path = unique_temp_vault("schema");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        let created = TodoService::create_todo(
            &vault_path,
            "Table Task",
            Some(
                "# Table Task\n\nMystery: value\n\n| Step | State |\n| --- | --- |\n| Draft | Open |\n\n#NoSpace\n"
                    .to_string(),
            ),
        )
        .expect("todos");

        assert!(created.parsed_entity.blocks.iter().any(|block| matches!(
            block,
            crate::domain::markdown_parser::MarkdownBlock::Table { .. }
        )));
        assert!(!created.parsed_entity.unknown_blocks.is_empty());
        assert!(created
            .schema_warnings
            .iter()
            .any(|warning| warning.contains("mystery")));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
