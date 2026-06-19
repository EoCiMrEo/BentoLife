use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::storage::{
    content_hash, current_timestamp_label, ensure_vault_relative_path, read_json, write_json_atomic,
};

pub const DOCUMENT_METADATA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentIdentityMetadata {
    pub strategy: String,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrontmatterContract {
    pub required_key: String,
    pub required_value: String,
    pub allowed_app_metadata_in_markdown: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentPolicy {
    pub content_lives_inside_vault: bool,
    pub markdown_is_content_source_of_truth: bool,
    pub layout_is_stored_in_bentolife_folder: bool,
    pub full_content_is_not_duplicated_in_metadata: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub schema_version: u32,
    pub document_id: String,
    pub document_type: String,
    pub vault_relative: bool,
    pub current_path: String,
    pub previous_paths: Vec<String>,
    pub layout_path: String,
    pub identity: DocumentIdentityMetadata,
    pub frontmatter_contract: FrontmatterContract,
    pub content_policy: ContentPolicy,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default = "default_recovery_status")]
    pub recovery_status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct DocumentMetadataService;

fn default_recovery_status() -> String {
    "managed".to_string()
}

impl DocumentMetadataService {
    pub const DOCUMENTS_FOLDER: &'static str = ".bentolifelayout/documents";

    pub fn service_name() -> &'static str {
        "DocumentMetadataService"
    }

    pub fn metadata_relative_path(document_id: &str) -> String {
        format!("{}/{}.json", Self::DOCUMENTS_FOLDER, document_id)
    }

    pub fn metadata_path(vault_path: &Path, document_id: &str) -> PathBuf {
        vault_path.join(Self::metadata_relative_path(document_id))
    }

    pub fn create_default(
        document_id: &str,
        markdown_relative_path: &str,
        markdown_content: &str,
    ) -> Result<DocumentMetadata, String> {
        Self::create_default_with_type(
            document_id,
            markdown_relative_path,
            markdown_content,
            "markdown_document",
        )
    }

    pub fn create_default_with_type(
        document_id: &str,
        markdown_relative_path: &str,
        markdown_content: &str,
        document_type: &str,
    ) -> Result<DocumentMetadata, String> {
        Self::validate_document_id(document_id)?;
        ensure_vault_relative_path(markdown_relative_path)?;

        let metadata_path = Self::metadata_relative_path(document_id);
        let layout_path = format!(
            "{}/{}.layout.json",
            super::layout_metadata::LayoutMetadataService::LAYOUTS_FOLDER,
            document_id
        );
        let now = current_timestamp_label();

        let metadata = DocumentMetadata {
            schema_version: DOCUMENT_METADATA_VERSION,
            document_id: document_id.to_string(),
            document_type: document_type.to_string(),
            vault_relative: true,
            current_path: markdown_relative_path.replace('\\', "/"),
            previous_paths: Vec::new(),
            layout_path,
            identity: DocumentIdentityMetadata {
                strategy: "hidden_markdown_uuid_comment".to_string(),
                comment: format!("<!-- bentolife:document_id={document_id} -->"),
            },
            frontmatter_contract: FrontmatterContract {
                required_key: "bentolife_metadata".to_string(),
                required_value: metadata_path,
                allowed_app_metadata_in_markdown: vec!["bentolife_metadata".to_string()],
            },
            content_policy: ContentPolicy {
                content_lives_inside_vault: true,
                markdown_is_content_source_of_truth: true,
                layout_is_stored_in_bentolife_folder: true,
                full_content_is_not_duplicated_in_metadata: true,
            },
            content_hash: content_hash(markdown_content),
            recovery_status: "managed".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn read(vault_path: &Path, document_id: &str) -> Result<DocumentMetadata, String> {
        let metadata =
            read_json::<DocumentMetadata>(&Self::metadata_path(vault_path, document_id))?;
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn write(vault_path: &Path, metadata: &DocumentMetadata) -> Result<(), String> {
        metadata.validate()?;
        write_json_atomic(
            &Self::metadata_path(vault_path, &metadata.document_id),
            metadata,
        )
    }

    pub fn list(vault_path: &Path) -> Result<Vec<DocumentMetadata>, String> {
        let folder = vault_path.join(Self::DOCUMENTS_FOLDER);
        if !folder.exists() {
            return Ok(Vec::new());
        }

        let mut documents = Vec::new();
        for entry in std::fs::read_dir(&folder)
            .map_err(|error| format!("Unable to read {}: {error}", folder.display()))?
        {
            let entry = entry.map_err(|error| format!("Unable to read metadata entry: {error}"))?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            let metadata = read_json::<DocumentMetadata>(&path)?;
            metadata.validate()?;
            documents.push(metadata);
        }

        documents.sort_by(|left, right| left.document_id.cmp(&right.document_id));
        Ok(documents)
    }

    pub fn validate_document_id(document_id: &str) -> Result<(), String> {
        if !document_id.starts_with("bl_doc_") {
            return Err("Document IDs must start with bl_doc_.".to_string());
        }

        if document_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            Ok(())
        } else {
            Err(
                "Document IDs may contain only ASCII letters, numbers, and underscores."
                    .to_string(),
            )
        }
    }
}

impl DocumentMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != DOCUMENT_METADATA_VERSION {
            return Err(format!(
                "Unsupported document metadata version {}.",
                self.schema_version
            ));
        }
        DocumentMetadataService::validate_document_id(&self.document_id)?;
        ensure_vault_relative_path(&self.current_path)?;
        ensure_vault_relative_path(&self.layout_path)?;
        ensure_vault_relative_path(&self.frontmatter_contract.required_value)?;

        if self.frontmatter_contract.required_key != "bentolife_metadata" {
            return Err("Document metadata must use bentolife_metadata frontmatter.".to_string());
        }

        if self.frontmatter_contract.required_value
            != DocumentMetadataService::metadata_relative_path(&self.document_id)
        {
            return Err("Document metadata path must be based on the document ID.".to_string());
        }

        if !self.vault_relative {
            return Err("Document metadata must use vault-relative paths.".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_document_metadata_uses_document_id_filename() {
        let metadata =
            DocumentMetadataService::create_default("bl_doc_test", "notes/test.md", "# Test\n")
                .expect("metadata is valid");

        assert_eq!(
            metadata.frontmatter_contract.required_value,
            ".bentolifelayout/documents/bl_doc_test.json"
        );
        assert_eq!(
            metadata.layout_path,
            ".bentolifelayout/layouts/bl_doc_test.layout.json"
        );
    }

    #[test]
    fn document_metadata_rejects_absolute_paths() {
        let mut metadata =
            DocumentMetadataService::create_default("bl_doc_test", "notes/test.md", "# Test\n")
                .expect("metadata is valid");
        metadata.current_path = "C:/Users/Bento/test.md".to_string();

        assert!(metadata.validate().is_err());
    }
}
