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
pub struct NoteSummary {
    pub document_id: String,
    pub title: String,
    pub markdown_relative_path: String,
    pub excerpt: String,
    pub status: ScannedDocumentStatus,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteDocument {
    pub document_id: String,
    pub title: String,
    pub markdown_relative_path: String,
    pub markdown_body: String,
    pub parsed_entity: ParsedEntityContract,
    pub schema_warnings: Vec<String>,
    pub document_metadata: DocumentMetadata,
    pub layout_metadata: Option<LayoutMetadata>,
}

pub struct NotesService;

impl NotesService {
    pub fn list_notes(vault_path: &Path) -> Result<Vec<NoteSummary>, String> {
        let scan = WorkspaceScanner::scan(vault_path)?;
        let mut notes = Vec::new();

        for document in scan.documents {
            let is_note = document.document_type == "note"
                || document
                    .markdown_relative_path
                    .starts_with("modules/notes/data/");
            let Some(document_id) = document.document_id else {
                continue;
            };

            if !is_note {
                continue;
            }

            let metadata = DocumentMetadataService::read(vault_path, &document_id).ok();
            notes.push(NoteSummary {
                document_id,
                title: document.title,
                markdown_relative_path: document.markdown_relative_path,
                excerpt: markdown_excerpt(&document.markdown_body),
                status: document.status,
                updated_at: metadata.map(|metadata| metadata.updated_at),
            });
        }

        notes.sort_by_key(|note| note.title.to_lowercase());
        Ok(notes)
    }

    pub fn read_note(vault_path: &Path, document_id: &str) -> Result<NoteDocument, String> {
        let metadata = DocumentMetadataService::read(vault_path, document_id)?;
        validate_note_content_path(&metadata.current_path)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
        let markdown = std::fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
        let markdown_body = DocumentIdentityService::remove_identity_comments(&parsed.body)
            .trim()
            .to_string();
        let layout_metadata = LayoutMetadataService::read(vault_path, document_id).ok();

        let mut parsed_entity = MarkdownParser::parse(&markdown_body);
        parsed_entity.module_id = Some("notes".to_string());
        parsed_entity.entity_type = Some("note".to_string());
        parsed_entity.path = metadata.current_path.clone();
        parsed_entity.content_hash = metadata.content_hash.clone();
        let schema_warnings = apply_schema_descriptors(
            vault_path,
            "modules/notes/module.schema.json",
            &mut parsed_entity,
        )?;

        Ok(NoteDocument {
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

    pub fn create_note(
        vault_path: &Path,
        title: &str,
        markdown_body: Option<String>,
    ) -> Result<NoteDocument, String> {
        let title = clean_title(title);
        let markdown_relative_path = unique_note_path(vault_path, &title)?;
        validate_note_content_path(&markdown_relative_path)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &markdown_relative_path)?;
        if markdown_path.exists() {
            return Err(format!(
                "A note already exists at {markdown_relative_path}."
            ));
        }

        LayoutFolderService::create_or_repair(vault_path)?;
        WorkspaceMetadataService::write_bootstrap_files(vault_path)?;

        let body = normalize_note_body(&title, markdown_body.as_deref().unwrap_or(""));
        let document_id = generate_document_id(&markdown_relative_path);
        let mut document_metadata = DocumentMetadataService::create_default_with_type(
            &document_id,
            &markdown_relative_path,
            &body,
            "note",
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

        Self::read_note(vault_path, &document_id)
    }

    pub fn update_note(
        vault_path: &Path,
        document_id: &str,
        markdown_body: String,
        expected_content_hash: Option<String>,
        overwrite_conflict: bool,
    ) -> Result<NoteDocument, String> {
        let mut metadata = DocumentMetadataService::read(vault_path, document_id)?;
        metadata.document_type = "note".to_string();
        validate_note_content_path(&metadata.current_path)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
        reject_stale_note_write(
            &markdown_path,
            expected_content_hash
                .as_deref()
                .unwrap_or(&metadata.content_hash),
            overwrite_conflict,
        )?;
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

        Self::read_note(vault_path, document_id)
    }

    pub fn rename_note(
        vault_path: &Path,
        document_id: &str,
        new_title: &str,
    ) -> Result<NoteDocument, String> {
        let mut note = Self::read_note(vault_path, document_id)?;
        let mut metadata = note.document_metadata.clone();
        metadata.document_type = "note".to_string();

        let new_title = clean_title(new_title);
        let new_relative_path = note_path_for_title(&new_title);
        let current_relative_path = metadata.current_path.clone();
        validate_note_content_path(&current_relative_path)?;
        validate_note_content_path(&new_relative_path)?;
        let current_path = resolve_vault_relative_path(vault_path, &current_relative_path)?;
        let new_path = resolve_vault_relative_path(vault_path, &new_relative_path)?;

        if new_relative_path != current_relative_path && new_path.exists() {
            return Err(format!("A note already exists at {new_relative_path}."));
        }

        note.markdown_body = replace_or_insert_h1(&note.markdown_body, &new_title);
        let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
            &note.markdown_body,
            document_id,
            &metadata.frontmatter_contract.required_value,
        );

        if new_relative_path != current_relative_path {
            if let Some(parent) = new_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("Unable to create {}: {error}", parent.display()))?;
            }
            std::fs::rename(&current_path, &new_path).map_err(|error| {
                format!("Unable to rename note to {new_relative_path}: {error}")
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

        Self::read_note(vault_path, document_id)
    }
}

fn reject_stale_note_write(
    markdown_path: &Path,
    expected_hash: &str,
    overwrite_conflict: bool,
) -> Result<(), String> {
    if overwrite_conflict {
        return Ok(());
    }
    let current_markdown = std::fs::read_to_string(markdown_path).map_err(|error| {
        format!(
            "Unable to read {} before writing: {error}",
            markdown_path.display()
        )
    })?;
    let current_hash = content_hash(&current_markdown);
    if current_hash != expected_hash {
        return Err("Note was changed outside BentoLife. Reload latest, save as copy, or choose overwrite before saving.".to_string());
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
        "Untitled Note".to_string()
    } else {
        cleaned.to_string()
    }
}

fn note_path_for_title(title: &str) -> String {
    format!("modules/notes/data/{}.md", slugify(title))
}

fn validate_note_content_path(path: &str) -> Result<(), String> {
    let is_v3_note = path.starts_with("modules/notes/data/");
    if !is_v3_note || !path.ends_with(".md") || path.contains(".bentolifelayout") {
        return Err("V3 notes must live under modules/notes/data/ as Markdown files.".to_string());
    }
    Ok(())
}

fn unique_note_path(vault_path: &Path, title: &str) -> Result<String, String> {
    let base_slug = slugify(title);
    for index in 0..100 {
        let candidate = if index == 0 {
            format!("modules/notes/data/{base_slug}.md")
        } else {
            format!("modules/notes/data/{base_slug}-{index}.md")
        };
        if !resolve_vault_relative_path(vault_path, &candidate)?.exists() {
            return Ok(candidate);
        }
    }
    Err("Unable to find an available note filename.".to_string())
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
        "untitled-note".to_string()
    } else {
        slug
    }
}

fn normalize_note_body(title: &str, markdown_body: &str) -> String {
    let body = markdown_body.trim();
    if body.is_empty() {
        format!("# {title}\n")
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
        .unwrap_or("Untitled Note")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-notes-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    #[test]
    fn creates_reads_updates_and_renames_note_without_changing_identity() {
        let vault_path = unique_temp_vault("crud");
        let created = NotesService::create_note(
            &vault_path,
            "Daily Plan",
            Some("## Morning\n\n- [ ] Tea".to_string()),
        )
        .expect("note created");
        let document_id = created.document_id.clone();

        let updated = NotesService::update_note(
            &vault_path,
            &document_id,
            "# Daily Plan\n\n## Afternoon\n\nParagraph".to_string(),
            Some(created.document_metadata.content_hash.clone()),
            false,
        )
        .expect("note updated");
        assert_eq!(updated.document_id, document_id);

        let renamed = NotesService::rename_note(&vault_path, &document_id, "Renamed Plan")
            .expect("note renamed");
        assert_eq!(renamed.document_id, document_id);
        assert_eq!(
            renamed.markdown_relative_path,
            "modules/notes/data/renamed-plan.md"
        );
        assert!(renamed.markdown_body.contains("# Renamed Plan"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rejects_stale_note_writes_after_external_edit() {
        let vault_path = unique_temp_vault("stale");
        let created = NotesService::create_note(&vault_path, "Daily Plan", None).expect("note");
        std::fs::write(
            vault_path.join(&created.markdown_relative_path),
            "# Daily Plan\n\nExternal edit\n",
        )
        .expect("external edit");

        let result = NotesService::update_note(
            &vault_path,
            &created.document_id,
            "# Daily Plan\n\nBentoLife edit\n".to_string(),
            Some(created.document_metadata.content_hash),
            false,
        );

        assert!(result.is_err());

        let overwritten = NotesService::update_note(
            &vault_path,
            &created.document_id,
            "# Daily Plan\n\nBentoLife edit\n".to_string(),
            None,
            true,
        )
        .expect("overwrite");
        assert!(overwritten.markdown_body.contains("BentoLife edit"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rejects_rename_collisions() {
        let vault_path = unique_temp_vault("collision");
        let first = NotesService::create_note(&vault_path, "First", None).expect("first");
        let _second = NotesService::create_note(&vault_path, "Second", None).expect("second");

        let result = NotesService::rename_note(&vault_path, &first.document_id, "Second");

        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn parses_tables_unknown_blocks_and_matches_schema() {
        let vault_path = unique_temp_vault("schema");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        let created = NotesService::create_note(
            &vault_path,
            "Schema Proof",
            Some(
                "# Schema Proof\n\n| Field | Value |\n| --- | --- |\n| Mood | Clear |\n\n#NoSpace\n"
                    .to_string(),
            ),
        )
        .expect("note");

        assert!(created.parsed_entity.blocks.iter().any(|block| matches!(
            block,
            crate::domain::markdown_parser::MarkdownBlock::Table { .. }
        )));
        assert!(!created.parsed_entity.unknown_blocks.is_empty());
        assert!(created.schema_warnings.is_empty());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
