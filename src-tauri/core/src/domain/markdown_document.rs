use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    document_identity::DocumentIdentityService,
    document_metadata::{DocumentMetadata, DocumentMetadataService},
    layout_folder::LayoutFolderService,
    layout_metadata::{LayoutMetadata, LayoutMetadataService},
    storage::{generate_document_id, resolve_vault_relative_path, write_text_atomic},
    workspace_metadata::WorkspaceMetadataService,
};

pub const FRONTMATTER_REFERENCE_KEY: &str = "bentolife_metadata";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedFrontmatter {
    pub metadata_reference: Option<String>,
    pub body: String,
    pub raw_frontmatter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedMarkdownDocument {
    pub document_id: String,
    pub markdown_relative_path: String,
    pub metadata_path: String,
    pub layout_path: String,
    pub markdown: String,
    pub document_metadata: DocumentMetadata,
    pub layout_metadata: LayoutMetadata,
}

pub struct MarkdownDocumentService;

impl MarkdownDocumentService {
    pub fn service_name() -> &'static str {
        "MarkdownDocumentService"
    }

    pub fn parse_frontmatter(markdown: &str) -> ParsedFrontmatter {
        let Some(after_opening) = markdown.strip_prefix("---") else {
            return ParsedFrontmatter {
                metadata_reference: None,
                body: markdown.to_string(),
                raw_frontmatter: None,
            };
        };

        if !after_opening.starts_with('\n') && !after_opening.starts_with("\r\n") {
            return ParsedFrontmatter {
                metadata_reference: None,
                body: markdown.to_string(),
                raw_frontmatter: None,
            };
        }

        let line_start = if after_opening.starts_with("\r\n") {
            5
        } else {
            4
        };
        let content_after_open = &markdown[line_start..];
        let Some(closing_offset) = content_after_open.find("\n---") else {
            return ParsedFrontmatter {
                metadata_reference: None,
                body: markdown.to_string(),
                raw_frontmatter: None,
            };
        };

        let raw_frontmatter = content_after_open[..closing_offset]
            .trim_matches('\r')
            .to_string();
        let closing_start = line_start + closing_offset;
        let closing_line_end = markdown[closing_start + 1..]
            .find('\n')
            .map(|offset| closing_start + 1 + offset + 1)
            .unwrap_or(markdown.len());
        let body = markdown[closing_line_end..].to_string();
        let metadata_reference = frontmatter_value(&raw_frontmatter, FRONTMATTER_REFERENCE_KEY);

        ParsedFrontmatter {
            metadata_reference,
            body,
            raw_frontmatter: Some(raw_frontmatter),
        }
    }

    pub fn prepare_managed_markdown(
        markdown: &str,
        document_id: &str,
        metadata_path: &str,
    ) -> String {
        let parsed = Self::parse_frontmatter(markdown);
        let frontmatter = upsert_frontmatter_value(
            parsed.raw_frontmatter.as_deref(),
            FRONTMATTER_REFERENCE_KEY,
            metadata_path,
        );
        let content_with_frontmatter =
            format!("---\n{frontmatter}\n---\n\n{}", parsed.body.trim_start());

        DocumentIdentityService::ensure_identity_comment_at_end(
            &content_with_frontmatter,
            document_id,
        )
    }

    pub fn manage_document(
        vault_path: &Path,
        markdown_relative_path: &str,
        markdown: &str,
    ) -> Result<ManagedMarkdownDocument, String> {
        LayoutFolderService::create_or_repair(vault_path)?;
        WorkspaceMetadataService::write_bootstrap_files(vault_path)?;

        let existing_identity = DocumentIdentityService::find_identity_comment(markdown);
        let document_id = existing_identity
            .map(|identity| identity.document_id)
            .unwrap_or_else(|| generate_document_id(markdown_relative_path));
        DocumentMetadataService::validate_document_id(&document_id)?;

        let document_metadata = DocumentMetadataService::create_default(
            &document_id,
            markdown_relative_path,
            markdown,
        )?;
        let layout_metadata = LayoutMetadataService::create_default(&document_id)?;
        let metadata_path = document_metadata
            .frontmatter_contract
            .required_value
            .clone();
        let layout_path = document_metadata.layout_path.clone();
        let managed_markdown =
            Self::prepare_managed_markdown(markdown, &document_id, &metadata_path);

        let markdown_path = resolve_vault_relative_path(vault_path, markdown_relative_path)?;
        write_text_atomic(&markdown_path, &managed_markdown)?;
        DocumentMetadataService::write(vault_path, &document_metadata)?;
        LayoutMetadataService::write(vault_path, &layout_metadata)?;

        let documents = DocumentMetadataService::list(vault_path)?;
        let index = WorkspaceMetadataService::rebuild_index_from_documents(&documents)?;
        WorkspaceMetadataService::write_index(vault_path, &index)?;
        WorkspaceMetadataService::register_document(vault_path, &document_metadata)?;

        Ok(ManagedMarkdownDocument {
            document_id,
            markdown_relative_path: markdown_relative_path.replace('\\', "/"),
            metadata_path,
            layout_path,
            markdown: managed_markdown,
            document_metadata,
            layout_metadata,
        })
    }
}

fn frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
    frontmatter.lines().find_map(|line| {
        let trimmed = line.trim();
        let (candidate_key, value) = trimmed.split_once(':')?;
        if candidate_key.trim() == key {
            Some(value.trim().trim_matches('"').to_string())
        } else {
            None
        }
    })
}

fn upsert_frontmatter_value(frontmatter: Option<&str>, key: &str, value: &str) -> String {
    let mut found = false;
    let mut lines: Vec<String> = frontmatter
        .unwrap_or("")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let Some((candidate_key, _)) = line.split_once(':') else {
                return line.to_string();
            };

            if candidate_key.trim() == key {
                found = true;
                format!("{key}: {value}")
            } else {
                line.to_string()
            }
        })
        .collect();

    if !found {
        lines.insert(0, format!("{key}: {value}"));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_existing_bentolife_frontmatter() {
        let parsed = MarkdownDocumentService::parse_frontmatter(
            "---\nbentolife_metadata: .bentolifelayout/documents/bl_doc_daily.json\n---\n\n# Daily\n",
        );

        assert_eq!(
            parsed.metadata_reference.as_deref(),
            Some(".bentolifelayout/documents/bl_doc_daily.json")
        );
        assert!(parsed.body.contains("# Daily"));
    }

    #[test]
    fn inserts_metadata_reference_and_identity_without_losing_body() {
        let managed = MarkdownDocumentService::prepare_managed_markdown(
            "# Daily\n\n- [ ] Tea\n",
            "bl_doc_daily",
            ".bentolifelayout/documents/bl_doc_daily.json",
        );

        assert!(
            managed.contains("bentolife_metadata: .bentolifelayout/documents/bl_doc_daily.json")
        );
        assert!(managed.contains("- [ ] Tea"));
        assert!(managed.ends_with("<!-- bentolife:document_id=bl_doc_daily -->\n"));
    }

    #[test]
    fn preserves_existing_document_identity() {
        let managed = MarkdownDocumentService::prepare_managed_markdown(
            "# Daily\n\n<!-- bentolife:document_id=bl_doc_daily -->\n",
            "bl_doc_daily",
            ".bentolifelayout/documents/bl_doc_daily.json",
        );

        assert_eq!(
            managed
                .matches("bentolife:document_id=bl_doc_daily")
                .count(),
            1
        );
    }

    #[test]
    fn manages_markdown_and_writes_matching_metadata() {
        let mut vault_path = std::env::temp_dir();
        vault_path.push("bentolife-markdown-contract-test");
        vault_path.push(".bentolifevault");
        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));

        let managed = MarkdownDocumentService::manage_document(
            &vault_path,
            "notes/daily.md",
            "# Daily\n\n- [ ] Tea\n",
        )
        .expect("document is managed");

        assert!(vault_path.join("notes/daily.md").is_file());
        assert!(vault_path.join(&managed.metadata_path).is_file());
        assert!(vault_path.join(&managed.layout_path).is_file());
        assert!(vault_path.join(".bentolifelayout/index.json").is_file());
        assert!(managed.markdown.contains("- [ ] Tea"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
