use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{
    document_identity::DocumentIdentityService,
    document_metadata::{DocumentMetadata, DocumentMetadataService},
    layout_folder::LayoutFolderService,
    layout_metadata::{LayoutMetadata, LayoutMetadataService},
    markdown_document::MarkdownDocumentService,
    storage::{
        content_hash, current_timestamp_label, resolve_vault_relative_path, write_json_atomic,
        write_text_atomic,
    },
    vault::{VaultService, VaultState},
    workspace_metadata::WorkspaceMetadataService,
    workspace_scanner::{WorkspaceScanResult, WorkspaceScanner},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryIssue {
    pub code: String,
    pub message: String,
    pub document_id: Option<String>,
    pub markdown_relative_path: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceRecoveryPreview {
    pub vault_path: String,
    pub issues: Vec<RecoveryIssue>,
    pub scan: Option<WorkspaceScanResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecoveryResult {
    pub action: String,
    pub document_id: Option<String>,
    pub markdown_relative_path: Option<String>,
    pub changed_paths: Vec<String>,
    pub message: String,
    pub scan: Option<WorkspaceScanResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrphanManifest {
    pub document_id: String,
    pub original_metadata_path: String,
    pub original_layout_path: String,
    pub last_known_markdown_path: String,
    pub reason: String,
    pub orphaned_at: String,
}

pub struct RecoveryService;

impl RecoveryService {
    pub fn service_name() -> &'static str {
        "RecoveryService"
    }

    pub fn preview_workspace_recovery(
        vault_path: &Path,
    ) -> Result<WorkspaceRecoveryPreview, String> {
        let inspection = VaultService::inspect(vault_path);
        let mut issues = Vec::new();

        if inspection.state == VaultState::LayoutMissing {
            issues.push(RecoveryIssue {
                code: "layout_folder_missing".to_string(),
                message:
                    "The .bentolifelayout folder is missing; Markdown content may still be safe."
                        .to_string(),
                document_id: None,
                markdown_relative_path: None,
                action: Some("repair_vault_structure".to_string()),
            });
        } else if inspection.state == VaultState::ScaffoldIncomplete {
            issues.push(RecoveryIssue {
                code: "layout_scaffold_incomplete".to_string(),
                message: "Required BentoLife metadata folders or bootstrap files are missing."
                    .to_string(),
                document_id: None,
                markdown_relative_path: None,
                action: Some("repair_vault_structure".to_string()),
            });
        } else if inspection.state != VaultState::Ready {
            issues.push(RecoveryIssue {
                code: "vault_not_ready".to_string(),
                message: inspection.message.clone(),
                document_id: None,
                markdown_relative_path: None,
                action: None,
            });
        }

        let scan = if inspection.state == VaultState::Ready {
            let scan = WorkspaceScanner::scan(vault_path)?;
            issues.extend(
                scan.issues
                    .iter()
                    .filter(|issue| issue.classification == "recovery_issue")
                    .map(|issue| RecoveryIssue {
                        code: issue.code.clone(),
                        message: issue.message.clone(),
                        document_id: issue.document_id.clone(),
                        markdown_relative_path: issue.markdown_relative_path.clone(),
                        action: action_for_issue(&issue.code),
                    }),
            );
            Some(scan)
        } else {
            None
        };

        Ok(WorkspaceRecoveryPreview {
            vault_path: vault_path.to_string_lossy().to_string(),
            issues,
            scan,
        })
    }

    pub fn recover_document_metadata(
        vault_path: &Path,
        markdown_relative_path: &str,
    ) -> Result<RecoveryResult, String> {
        LayoutFolderService::create_or_repair(vault_path)?;
        WorkspaceMetadataService::write_bootstrap_files(vault_path)?;

        let markdown_path = resolve_vault_relative_path(vault_path, markdown_relative_path)?;
        let markdown = fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let identity =
            DocumentIdentityService::find_identity_comment(&markdown).ok_or_else(|| {
                "Cannot recover document metadata without a BentoLife document ID comment."
                    .to_string()
            })?;
        DocumentMetadataService::validate_document_id(&identity.document_id)?;

        let metadata_path =
            DocumentMetadataService::metadata_path(vault_path, &identity.document_id);
        if metadata_path.exists() {
            return Err(format!(
                "Document metadata already exists for {}.",
                identity.document_id
            ));
        }

        let metadata = DocumentMetadataService::create_default_with_type(
            &identity.document_id,
            markdown_relative_path,
            &markdown,
            document_type_for_path(markdown_relative_path),
        )?;
        DocumentMetadataService::write(vault_path, &metadata)?;
        let scan = WorkspaceScanner::scan(vault_path)?;

        Ok(RecoveryResult {
            action: "recover_document_metadata".to_string(),
            document_id: Some(identity.document_id),
            markdown_relative_path: Some(markdown_relative_path.replace('\\', "/")),
            changed_paths: vec![metadata.frontmatter_contract.required_value],
            message: "Document metadata was rebuilt from the Markdown identity comment."
                .to_string(),
            scan: Some(scan),
        })
    }

    pub fn recover_layout_metadata(
        vault_path: &Path,
        document_id: &str,
    ) -> Result<RecoveryResult, String> {
        let metadata = DocumentMetadataService::read(vault_path, document_id)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
        let markdown = fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let layout = LayoutMetadataService::generate_from_markdown(document_id, &markdown)?;
        LayoutMetadataService::write(vault_path, &layout)?;
        let scan = WorkspaceScanner::scan(vault_path)?;

        Ok(RecoveryResult {
            action: "recover_layout_metadata".to_string(),
            document_id: Some(document_id.to_string()),
            markdown_relative_path: Some(metadata.current_path),
            changed_paths: vec![LayoutMetadataService::layout_relative_path(document_id)],
            message: "Layout metadata was regenerated from Markdown headings.".to_string(),
            scan: Some(scan),
        })
    }

    pub fn orphan_missing_document_metadata(
        vault_path: &Path,
        document_id: &str,
    ) -> Result<RecoveryResult, String> {
        let mut metadata = DocumentMetadataService::read(vault_path, document_id)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
        if markdown_path.exists() {
            return Err(
                "Refusing to orphan metadata while the Markdown file still exists.".to_string(),
            );
        }

        let original_metadata_path = metadata.frontmatter_contract.required_value.clone();
        let original_layout_path = metadata.layout_path.clone();
        let last_known_markdown_path = metadata.current_path.clone();
        metadata.recovery_status = "orphaned".to_string();
        metadata.updated_at = current_timestamp_label();

        let orphan_document_path = orphan_document_path(vault_path, document_id);
        let orphan_layout_path = orphan_layout_path(vault_path, document_id);
        let orphan_manifest_path = orphan_manifest_path(vault_path, document_id);
        ensure_orphan_folders(vault_path)?;

        if orphan_document_path.exists() || orphan_manifest_path.exists() {
            return Err(format!("Orphan metadata already exists for {document_id}."));
        }

        write_json_atomic(&orphan_document_path, &metadata)?;
        let active_document_path = DocumentMetadataService::metadata_path(vault_path, document_id);
        fs::remove_file(&active_document_path).map_err(|error| {
            format!(
                "Unable to remove {}: {error}",
                active_document_path.display()
            )
        })?;

        let active_layout_path = LayoutMetadataService::layout_path(vault_path, document_id);
        let mut changed_paths = vec![
            orphan_relative_path(vault_path, &orphan_document_path)?,
            orphan_relative_path(vault_path, &orphan_manifest_path)?,
        ];
        if active_layout_path.exists() {
            fs::rename(&active_layout_path, &orphan_layout_path).map_err(|error| {
                format!(
                    "Unable to move layout metadata from {} to {}: {error}",
                    active_layout_path.display(),
                    orphan_layout_path.display()
                )
            })?;
            changed_paths.push(orphan_relative_path(vault_path, &orphan_layout_path)?);
        }

        let manifest = OrphanManifest {
            document_id: document_id.to_string(),
            original_metadata_path,
            original_layout_path,
            last_known_markdown_path: last_known_markdown_path.clone(),
            reason: "markdown_missing".to_string(),
            orphaned_at: current_timestamp_label(),
        };
        write_json_atomic(&orphan_manifest_path, &manifest)?;

        let scan = WorkspaceScanner::scan(vault_path)?;
        Ok(RecoveryResult {
            action: "orphan_missing_document_metadata".to_string(),
            document_id: Some(document_id.to_string()),
            markdown_relative_path: Some(last_known_markdown_path),
            changed_paths,
            message: "Missing-file metadata was moved into orphan storage and preserved for explicit restore or cleanup.".to_string(),
            scan: Some(scan),
        })
    }

    pub fn restore_orphaned_document_metadata(
        vault_path: &Path,
        document_id: &str,
        markdown_relative_path: &str,
    ) -> Result<RecoveryResult, String> {
        let orphan_manifest = read_orphan_manifest(vault_path, document_id)?;
        let markdown_path = resolve_vault_relative_path(vault_path, markdown_relative_path)?;
        let markdown = fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let identity =
            DocumentIdentityService::find_identity_comment(&markdown).ok_or_else(|| {
                "Cannot restore orphaned metadata without a BentoLife document ID comment."
                    .to_string()
            })?;
        if identity.document_id != document_id {
            return Err(format!(
                "Markdown identity {} does not match orphaned document {}.",
                identity.document_id, document_id
            ));
        }

        let orphan_document_path = orphan_document_path(vault_path, document_id);
        let mut metadata = super::storage::read_json::<DocumentMetadata>(&orphan_document_path)?;
        metadata.current_path = markdown_relative_path.replace('\\', "/");
        metadata.recovery_status = "managed".to_string();
        metadata.content_hash = content_hash(&markdown);
        metadata.updated_at = current_timestamp_label();
        DocumentMetadataService::write(vault_path, &metadata)?;

        let orphan_layout_path = orphan_layout_path(vault_path, document_id);
        if orphan_layout_path.exists() {
            let layout = super::storage::read_json::<LayoutMetadata>(&orphan_layout_path)?;
            LayoutMetadataService::write(vault_path, &layout)?;
            fs::remove_file(&orphan_layout_path).map_err(|error| {
                format!("Unable to remove {}: {error}", orphan_layout_path.display())
            })?;
        }

        fs::remove_file(&orphan_document_path).map_err(|error| {
            format!(
                "Unable to remove {}: {error}",
                orphan_document_path.display()
            )
        })?;
        let manifest_path = orphan_manifest_path(vault_path, document_id);
        fs::remove_file(&manifest_path)
            .map_err(|error| format!("Unable to remove {}: {error}", manifest_path.display()))?;

        let scan = WorkspaceScanner::scan(vault_path)?;
        Ok(RecoveryResult {
            action: "restore_orphaned_document_metadata".to_string(),
            document_id: Some(document_id.to_string()),
            markdown_relative_path: Some(markdown_relative_path.replace('\\', "/")),
            changed_paths: vec![
                orphan_manifest.original_metadata_path,
                orphan_manifest.original_layout_path,
            ],
            message: "Orphaned metadata was restored after matching the Markdown identity comment."
                .to_string(),
            scan: Some(scan),
        })
    }

    pub fn repair_document_frontmatter_reference(
        vault_path: &Path,
        document_id: &str,
    ) -> Result<RecoveryResult, String> {
        let mut metadata = DocumentMetadataService::read(vault_path, document_id)?;
        let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
        let markdown = fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
        let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
            &markdown,
            document_id,
            &metadata.frontmatter_contract.required_value,
        );
        metadata.content_hash = content_hash(&managed_markdown);
        metadata.updated_at = current_timestamp_label();

        write_text_atomic(&markdown_path, &managed_markdown)?;
        DocumentMetadataService::write(vault_path, &metadata)?;
        let scan = WorkspaceScanner::scan(vault_path)?;
        let markdown_relative_path = metadata.current_path.clone();

        Ok(RecoveryResult {
            action: "repair_document_frontmatter_reference".to_string(),
            document_id: Some(document_id.to_string()),
            markdown_relative_path: Some(markdown_relative_path.clone()),
            changed_paths: vec![markdown_relative_path],
            message: "Markdown frontmatter now points to the document metadata path for this document ID.".to_string(),
            scan: Some(scan),
        })
    }
}

fn action_for_issue(code: &str) -> Option<String> {
    match code {
        "metadata_missing" => Some("recover_document_metadata".to_string()),
        "layout_missing" => Some("recover_layout_metadata".to_string()),
        "markdown_missing" => Some("orphan_missing_document_metadata".to_string()),
        "metadata_path_mismatch" => Some("repair_document_frontmatter_reference".to_string()),
        _ => None,
    }
}

fn document_type_for_path(path: &str) -> &'static str {
    let normalized_path = path.replace('\\', "/");
    if normalized_path.starts_with("modules/notes/data/") {
        "note"
    } else if normalized_path.starts_with("modules/todos/data/") {
        "todos"
    } else if normalized_path.starts_with("modules/contacts/data/") {
        "contact"
    } else if normalized_path.starts_with("modules/habits/data/") {
        "habit"
    } else {
        "markdown_document"
    }
}

fn ensure_orphan_folders(vault_path: &Path) -> Result<(), String> {
    for path in [
        vault_path.join(".bentolifelayout/orphans/documents"),
        vault_path.join(".bentolifelayout/orphans/layouts"),
    ] {
        fs::create_dir_all(&path)
            .map_err(|error| format!("Unable to create {}: {error}", path.display()))?;
    }
    Ok(())
}

fn orphan_document_path(vault_path: &Path, document_id: &str) -> PathBuf {
    vault_path
        .join(".bentolifelayout/orphans/documents")
        .join(format!("{document_id}.json"))
}

fn orphan_layout_path(vault_path: &Path, document_id: &str) -> PathBuf {
    vault_path
        .join(".bentolifelayout/orphans/layouts")
        .join(format!("{document_id}.layout.json"))
}

fn orphan_manifest_path(vault_path: &Path, document_id: &str) -> PathBuf {
    vault_path
        .join(".bentolifelayout/orphans")
        .join(format!("{document_id}.orphan.json"))
}

fn read_orphan_manifest(vault_path: &Path, document_id: &str) -> Result<OrphanManifest, String> {
    super::storage::read_json::<OrphanManifest>(&orphan_manifest_path(vault_path, document_id))
}

fn orphan_relative_path(vault_path: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(vault_path)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|error| format!("Unable to make {} vault-relative: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        markdown_document::MarkdownDocumentService, notes::NotesService, vault::VaultService,
    };

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-recovery-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    #[test]
    fn recovers_missing_document_metadata_from_uuid_without_rewriting_markdown() {
        let vault_path = unique_temp_vault("metadata");
        let managed = MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/notes/data/daily.md",
            "# Daily\n",
        )
        .expect("managed");
        let markdown_before =
            fs::read_to_string(vault_path.join("modules/notes/data/daily.md")).expect("markdown");
        fs::remove_file(DocumentMetadataService::metadata_path(
            &vault_path,
            &managed.document_id,
        ))
        .expect("remove metadata");

        let result =
            RecoveryService::recover_document_metadata(&vault_path, "modules/notes/data/daily.md")
                .expect("metadata recovered");
        let markdown_after =
            fs::read_to_string(vault_path.join("modules/notes/data/daily.md")).expect("markdown");

        assert_eq!(
            result.document_id.as_deref(),
            Some(managed.document_id.as_str())
        );
        assert_eq!(markdown_before, markdown_after);
        assert!(
            DocumentMetadataService::metadata_path(&vault_path, &managed.document_id).is_file()
        );

        let _ = fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn recovers_missing_todo_metadata_with_todo_document_type() {
        let vault_path = unique_temp_vault("todos-metadata");
        let managed = MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/todos/data/tea.md",
            "# Todos\n\n## Inbox\n\n- [ ] Tea\n",
        )
        .expect("managed");
        fs::remove_file(DocumentMetadataService::metadata_path(
            &vault_path,
            &managed.document_id,
        ))
        .expect("remove metadata");

        RecoveryService::recover_document_metadata(&vault_path, "modules/todos/data/tea.md")
            .expect("metadata recovered");
        let metadata =
            DocumentMetadataService::read(&vault_path, &managed.document_id).expect("metadata");

        assert_eq!(metadata.document_type, "todos");

        let _ = fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn recovers_missing_layout_metadata_from_headings() {
        let vault_path = unique_temp_vault("layout");
        let managed = MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/notes/data/daily.md",
            "# Daily\n\n## Today\n\n- [ ] Tea\n",
        )
        .expect("managed");
        fs::remove_file(LayoutMetadataService::layout_path(
            &vault_path,
            &managed.document_id,
        ))
        .expect("remove layout");

        RecoveryService::recover_layout_metadata(&vault_path, &managed.document_id)
            .expect("layout recovered");
        let layout =
            LayoutMetadataService::read(&vault_path, &managed.document_id).expect("layout");

        assert_eq!(
            layout
                .cards
                .iter()
                .map(|card| card.section_match.as_str())
                .collect::<Vec<_>>(),
            vec!["# Daily", "## Today"]
        );

        let _ = fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn moves_and_restores_missing_file_metadata_as_orphan() {
        let vault_path = unique_temp_vault("orphan");
        let managed = MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/notes/data/daily.md",
            "# Daily\n",
        )
        .expect("managed");
        fs::rename(
            vault_path.join("modules/notes/data/daily.md"),
            vault_path.join("modules/notes/data/restored.md"),
        )
        .expect("simulate missing original");

        RecoveryService::orphan_missing_document_metadata(&vault_path, &managed.document_id)
            .expect("orphaned");

        assert!(orphan_document_path(&vault_path, &managed.document_id).is_file());
        assert!(
            !DocumentMetadataService::metadata_path(&vault_path, &managed.document_id).exists()
        );

        RecoveryService::restore_orphaned_document_metadata(
            &vault_path,
            &managed.document_id,
            "modules/notes/data/restored.md",
        )
        .expect("restored");
        let metadata =
            DocumentMetadataService::read(&vault_path, &managed.document_id).expect("metadata");

        assert_eq!(metadata.current_path, "modules/notes/data/restored.md");
        assert_eq!(metadata.recovery_status, "managed");
        assert!(!orphan_manifest_path(&vault_path, &managed.document_id).exists());

        let _ = fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn preview_excludes_system_scaffold_orphans_but_keeps_deleted_content_recovery() {
        let vault_path = unique_temp_vault("preview-missing-content");
        VaultService::create_vault(&vault_path).expect("vault");
        let managed = NotesService::create_note(&vault_path, "Daily", None).expect("note");
        fs::remove_file(vault_path.join(&managed.markdown_relative_path)).expect("delete markdown");

        let preview = RecoveryService::preview_workspace_recovery(&vault_path).expect("preview");
        let markdown_missing = preview
            .issues
            .iter()
            .filter(|issue| issue.code == "markdown_missing")
            .collect::<Vec<_>>();

        assert_eq!(markdown_missing.len(), 1);
        assert_eq!(
            markdown_missing[0].document_id.as_deref(),
            Some(managed.document_id.as_str())
        );
        assert_eq!(
            markdown_missing[0].action.as_deref(),
            Some("orphan_missing_document_metadata")
        );

        let _ = fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn repairs_stale_frontmatter_reference_without_losing_body() {
        let vault_path = unique_temp_vault("frontmatter");
        let managed = MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/notes/data/daily.md",
            "# Daily\n\nBody paragraph\n",
        )
        .expect("managed");
        let stale_markdown = fs::read_to_string(vault_path.join("modules/notes/data/daily.md"))
            .expect("markdown")
            .replace(
                &managed.metadata_path,
                ".bentolifelayout/documents/bl_doc_stale.json",
            );
        fs::write(
            vault_path.join("modules/notes/data/daily.md"),
            stale_markdown,
        )
        .expect("fixture");

        RecoveryService::repair_document_frontmatter_reference(&vault_path, &managed.document_id)
            .expect("frontmatter repaired");
        let repaired =
            fs::read_to_string(vault_path.join("modules/notes/data/daily.md")).expect("markdown");

        assert!(repaired.contains(&managed.metadata_path));
        assert!(repaired.contains("Body paragraph"));
        assert_eq!(
            repaired
                .matches(&format!("bentolife:document_id={}", managed.document_id))
                .count(),
            1
        );

        let _ = fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
