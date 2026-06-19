use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{
    document_identity::DocumentIdentityService,
    document_metadata::{DocumentMetadata, DocumentMetadataService},
    layout_metadata::{LayoutMetadata, LayoutMetadataService},
    markdown_document::MarkdownDocumentService,
    storage::content_hash,
    workspace_metadata::{DocumentIndexEntry, WorkspaceIndex, WorkspaceMetadataService},
};
use crate::domain::module_schema::{normalize_field, ModuleSchema};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScannedDocumentStatus {
    Managed,
    PlainMarkdown,
    MetadataMissing,
    LayoutMissing,
    MetadataPathMismatch,
    DuplicateIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScannedDocument {
    pub document_id: Option<String>,
    pub title: String,
    pub markdown_relative_path: String,
    pub metadata_path: Option<String>,
    pub layout_path: Option<String>,
    pub document_type: String,
    pub status: ScannedDocumentStatus,
    pub markdown: String,
    pub markdown_body: String,
    pub layout_metadata: Option<LayoutMetadata>,
    pub stale_layout_references: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanIssue {
    pub code: String,
    pub message: String,
    pub document_id: Option<String>,
    pub markdown_relative_path: Option<String>,
    pub classification: String,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceScanResult {
    pub vault_path: String,
    pub documents: Vec<ScannedDocument>,
    pub issues: Vec<ScanIssue>,
    pub index: WorkspaceIndex,
}

pub struct WorkspaceScanner;

impl WorkspaceScanner {
    pub fn service_name() -> &'static str {
        "WorkspaceScanner"
    }

    pub fn scan(vault_path: &Path) -> Result<WorkspaceScanResult, String> {
        reject_older_vault_scan(vault_path)?;
        WorkspaceMetadataService::write_bootstrap_files(vault_path)?;

        let metadata_by_id = Self::metadata_by_id(vault_path)?;
        let markdown_paths = Self::markdown_paths(vault_path)?;
        let mut scanned_candidates = Vec::new();
        let mut paths_by_id: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut seen_paths = BTreeSet::new();
        let mut issues = Vec::new();

        for markdown_path in markdown_paths {
            let markdown_relative_path = vault_relative_path(vault_path, &markdown_path)?;
            seen_paths.insert(markdown_relative_path.clone());
            let markdown = std::fs::read_to_string(&markdown_path)
                .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
            let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
            let identity = DocumentIdentityService::find_identity_comment(&markdown);
            let document_id = identity.map(|identity| identity.document_id);

            if let Some(document_id) = &document_id {
                paths_by_id
                    .entry(document_id.clone())
                    .or_default()
                    .push(markdown_relative_path.clone());
            }

            scanned_candidates.push(ScanCandidate {
                document_id,
                frontmatter_reference: parsed.metadata_reference,
                markdown,
                markdown_body: parsed.body,
                markdown_relative_path,
            });
        }

        let duplicate_ids: BTreeSet<String> = paths_by_id
            .iter()
            .filter_map(|(document_id, paths)| (paths.len() > 1).then_some(document_id.clone()))
            .collect();
        for duplicate_id in &duplicate_ids {
            issues.push(ScanIssue {
                code: "duplicate_identity".to_string(),
                message: "More than one Markdown file contains the same BentoLife document ID."
                    .to_string(),
                document_id: Some(duplicate_id.clone()),
                markdown_relative_path: None,
                classification: "recovery_issue".to_string(),
                suggested_action: Some("Open Recovery".to_string()),
            });
        }

        let mut documents = Vec::new();
        let mut index = WorkspaceMetadataService::default_index();

        for candidate in scanned_candidates {
            let mut status = if candidate.document_id.is_some() {
                ScannedDocumentStatus::Managed
            } else {
                ScannedDocumentStatus::PlainMarkdown
            };
            let mut metadata_path = candidate.frontmatter_reference.clone();
            let mut layout_path = None;
            let mut document_type = "markdown_document".to_string();
            let mut layout_metadata = None;
            let mut stale_layout_references = Vec::new();

            if let Some(document_id) = &candidate.document_id {
                if duplicate_ids.contains(document_id) {
                    status = ScannedDocumentStatus::DuplicateIdentity;
                } else if let Some(mut metadata) = metadata_by_id.get(document_id).cloned() {
                    if metadata.current_path != candidate.markdown_relative_path {
                        if !metadata.previous_paths.contains(&metadata.current_path) {
                            metadata.previous_paths.push(metadata.current_path.clone());
                        }
                        metadata.current_path = candidate.markdown_relative_path.clone();
                        metadata.content_hash = content_hash(&candidate.markdown);
                        metadata.updated_at = super::storage::current_timestamp_label();
                        DocumentMetadataService::write(vault_path, &metadata)?;
                    }

                    metadata_path = Some(metadata.frontmatter_contract.required_value.clone());
                    layout_path = Some(metadata.layout_path.clone());
                    document_type = metadata.document_type.clone();

                    if candidate.frontmatter_reference.as_deref()
                        != Some(metadata.frontmatter_contract.required_value.as_str())
                    {
                        status = ScannedDocumentStatus::MetadataPathMismatch;
                        issues.push(ScanIssue {
                            code: "metadata_path_mismatch".to_string(),
                            message: "Markdown frontmatter points to stale or mismatched metadata."
                                .to_string(),
                            document_id: Some(document_id.clone()),
                            markdown_relative_path: Some(candidate.markdown_relative_path.clone()),
                            classification: "recovery_issue".to_string(),
                            suggested_action: Some("Open Recovery".to_string()),
                        });
                    }

                    match LayoutMetadataService::read(vault_path, document_id) {
                        Ok(layout) => {
                            stale_layout_references = LayoutMetadataService::stale_section_matches(
                                &layout,
                                &candidate.markdown_body,
                            );
                            for _stale_reference in &stale_layout_references {
                                issues.push(ScanIssue {
                                    code: "layout_reference_stale".to_string(),
                                    message: "Layout metadata references a Markdown heading that no longer exists.".to_string(),
                                    document_id: Some(document_id.clone()),
                                    markdown_relative_path: Some(candidate.markdown_relative_path.clone()),
                                    classification: "recovery_issue".to_string(),
                                    suggested_action: Some("Open Recovery".to_string()),
                                });
                            }
                            layout_metadata = Some(layout);
                        }
                        Err(_) => {
                            status = ScannedDocumentStatus::LayoutMissing;
                            issues.push(ScanIssue {
                                code: "layout_missing".to_string(),
                                message: "Layout metadata is missing; a generated dashboard fallback will be used.".to_string(),
                                document_id: Some(document_id.clone()),
                                markdown_relative_path: Some(candidate.markdown_relative_path.clone()),
                                classification: "recovery_issue".to_string(),
                                suggested_action: Some("Open Recovery".to_string()),
                            });
                        }
                    }

                    if !matches!(status, ScannedDocumentStatus::DuplicateIdentity) {
                        index.documents_by_id.insert(
                            document_id.clone(),
                            DocumentIndexEntry {
                                current_path: candidate.markdown_relative_path.clone(),
                                metadata_path: metadata.frontmatter_contract.required_value.clone(),
                                layout_path: metadata.layout_path.clone(),
                            },
                        );
                        index.document_ids_by_path.insert(
                            candidate.markdown_relative_path.clone(),
                            document_id.clone(),
                        );
                    }
                } else {
                    status = ScannedDocumentStatus::MetadataMissing;
                    metadata_path =
                        Some(DocumentMetadataService::metadata_relative_path(document_id));
                    layout_path = Some(LayoutMetadataService::layout_relative_path(document_id));
                    issues.push(ScanIssue {
                        code: "metadata_missing".to_string(),
                        message: "Document identity exists, but document metadata is missing."
                            .to_string(),
                        document_id: Some(document_id.clone()),
                        markdown_relative_path: Some(candidate.markdown_relative_path.clone()),
                        classification: "recovery_issue".to_string(),
                        suggested_action: Some("Open Recovery".to_string()),
                    });
                }
            }

            let is_note = is_module_data_markdown(&candidate.markdown_relative_path, "notes");
            if is_note {
                push_schema_issues(
                    vault_path,
                    "modules/notes/module.schema.json",
                    "Note",
                    &candidate.markdown_body,
                    candidate.document_id.clone(),
                    &candidate.markdown_relative_path,
                    &mut issues,
                )?;
            }

            let is_todo = is_module_data_markdown(&candidate.markdown_relative_path, "todos");
            if is_todo {
                push_schema_issues(
                    vault_path,
                    "modules/todos/module.schema.json",
                    "Todos",
                    &candidate.markdown_body,
                    candidate.document_id.clone(),
                    &candidate.markdown_relative_path,
                    &mut issues,
                )?;
            }

            documents.push(ScannedDocument {
                document_id: candidate.document_id,
                title: markdown_title(&candidate.markdown_body, &candidate.markdown_relative_path),
                markdown_relative_path: candidate.markdown_relative_path,
                metadata_path,
                layout_path,
                document_type,
                status,
                markdown: candidate.markdown,
                markdown_body: candidate.markdown_body,
                layout_metadata,
                stale_layout_references,
            });
        }

        for metadata in metadata_by_id.values() {
            if is_app_owned_system_markdown_path(&metadata.current_path) {
                continue;
            }
            if !seen_paths.contains(&metadata.current_path) {
                index
                    .orphaned_document_ids
                    .push(metadata.document_id.clone());
                issues.push(ScanIssue {
                    code: "markdown_missing".to_string(),
                    message: "Document metadata exists, but the Markdown file is missing."
                        .to_string(),
                    document_id: Some(metadata.document_id.clone()),
                    markdown_relative_path: Some(metadata.current_path.clone()),
                    classification: "recovery_issue".to_string(),
                    suggested_action: Some("Open Recovery".to_string()),
                });
            }
        }

        index.duplicate_identity_conflicts = duplicate_ids.into_iter().collect();
        index.validate()?;
        WorkspaceMetadataService::write_index(vault_path, &index)?;

        documents.sort_by(|left, right| {
            left.markdown_relative_path
                .cmp(&right.markdown_relative_path)
        });

        Ok(WorkspaceScanResult {
            vault_path: vault_path.to_string_lossy().to_string(),
            documents,
            issues,
            index,
        })
    }

    fn metadata_by_id(vault_path: &Path) -> Result<BTreeMap<String, DocumentMetadata>, String> {
        let mut metadata_by_id = BTreeMap::new();
        for metadata in DocumentMetadataService::list(vault_path)? {
            metadata_by_id.insert(metadata.document_id.clone(), metadata);
        }
        Ok(metadata_by_id)
    }

    fn markdown_paths(vault_path: &Path) -> Result<Vec<PathBuf>, String> {
        let mut paths = Vec::new();
        collect_markdown_paths(vault_path, vault_path, &mut paths)?;
        paths.sort();
        Ok(paths)
    }
}

fn push_schema_issues(
    vault_path: &Path,
    schema_path: &str,
    label: &str,
    markdown_body: &str,
    document_id: Option<String>,
    markdown_relative_path: &str,
    issues: &mut Vec<ScanIssue>,
) -> Result<(), String> {
    let parsed = crate::domain::markdown_parser::MarkdownParser::parse(markdown_body);
    let schema = match ModuleSchema::load(vault_path, schema_path) {
        Ok(schema) => schema,
        Err(error) => {
            issues.push(ScanIssue {
                code: "module_schema_missing".to_string(),
                message: format!("{label} module schema could not be loaded: {error}"),
                document_id,
                markdown_relative_path: Some(markdown_relative_path.to_string()),
                classification: "schema_warning".to_string(),
                suggested_action: None,
            });
            return Ok(());
        }
    };
    let allowed = schema.allowed_field_names();
    for field in parsed.fields.keys() {
        if !allowed.contains(&normalize_field(field)) {
            issues.push(ScanIssue {
                code: "unknown_schema_field".to_string(),
                message: format!(
                    "{label} contains unknown schema field '{}', which will be safely preserved.",
                    field
                ),
                document_id: document_id.clone(),
                markdown_relative_path: Some(markdown_relative_path.to_string()),
                classification: "schema_warning".to_string(),
                suggested_action: None,
            });
        }
    }
    if !parsed.unknown_blocks.is_empty() {
        issues.push(ScanIssue {
            code: "unknown_markdown_block".to_string(),
            message: format!(
                "{label} contains {} unknown Markdown block(s), preserved for fallback rendering.",
                parsed.unknown_blocks.len()
            ),
            document_id,
            markdown_relative_path: Some(markdown_relative_path.to_string()),
            classification: "preserved_unknown_content".to_string(),
            suggested_action: None,
        });
    }
    Ok(())
}

fn reject_older_vault_scan(vault_path: &Path) -> Result<(), String> {
    if !vault_path.is_dir() {
        return Ok(());
    }
    if vault_path.join("notes").is_dir()
        || vault_path.join("modules/todos.md").is_file()
        || vault_path.join("modules/contacts.md").is_file()
        || vault_path.join("modules/habits.md").is_file()
    {
        return Err("Older BentoLife vault structure detected. Scan will not convert it; back up or snapshot the folder and import copied data into a fresh V3 vault.".to_string());
    }
    for module in ["notes", "todos", "contacts", "habits"] {
        let module_path = vault_path.join("modules").join(module);
        let Ok(entries) = std::fs::read_dir(module_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("md")
                && path.file_name().and_then(|name| name.to_str()) != Some("INDEX.md")
                && path.file_name().and_then(|name| name.to_str()) != Some("MODULE.md")
            {
                return Err("Older BentoLife vault structure detected. Scan will not convert it; back up or snapshot the folder and import copied data into a fresh V3 vault.".to_string());
            }
        }
    }
    Ok(())
}

struct ScanCandidate {
    document_id: Option<String>,
    frontmatter_reference: Option<String>,
    markdown: String,
    markdown_body: String,
    markdown_relative_path: String,
}

fn collect_markdown_paths(
    vault_path: &Path,
    folder: &Path,
    paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if should_skip_active_scan_folder(vault_path, folder) {
        return Ok(());
    }

    for entry in std::fs::read_dir(folder)
        .map_err(|error| format!("Unable to scan {}: {error}", folder.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read scan entry: {error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_paths(vault_path, &path, paths)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("md")
            && !should_skip_active_scan_markdown_file(vault_path, &path)
        {
            paths.push(path);
        }
    }

    let _ = vault_path;
    Ok(())
}

fn should_skip_active_scan_folder(vault_path: &Path, folder: &Path) -> bool {
    let Ok(relative) = folder.strip_prefix(vault_path) else {
        return false;
    };
    let normalized = normalize_vault_relative_path(relative);
    normalized == ".bentolifelayout"
        || normalized.starts_with(".bentolifelayout/")
        || normalized == "modules/trash"
        || normalized.starts_with("modules/trash/")
        || normalized == "modules/archive"
        || normalized.starts_with("modules/archive/")
}

fn should_skip_active_scan_markdown_file(vault_path: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(vault_path) else {
        return false;
    };
    is_app_owned_system_markdown_path(&normalize_vault_relative_path(relative))
}

fn is_app_owned_system_markdown_path(relative_path: &str) -> bool {
    relative_path == "modules/navigator/INDEX.md"
        || relative_path == "modules/navigator/NAVIGATOR.md"
        || relative_path == "modules/trash/INDEX.md"
        || relative_path == "modules/archive/INDEX.md"
        || (relative_path.starts_with("modules/") && relative_path.ends_with("/MODULE.md"))
}

fn is_module_data_markdown(relative_path: &str, module_id: &str) -> bool {
    relative_path.starts_with(&format!("modules/{module_id}/data/"))
        && relative_path.ends_with(".md")
}

fn normalize_vault_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn vault_relative_path(vault_path: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(vault_path)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|error| format!("Unable to make {} vault-relative: {error}", path.display()))
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
        .unwrap_or_else(|| {
            Path::new(markdown_relative_path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Untitled")
                .replace('-', " ")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::markdown_document::MarkdownDocumentService;

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-scanner-{name}-{}",
            super::super::storage::current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    #[test]
    fn scans_markdown_and_ignores_layout_folder() {
        let vault_path = unique_temp_vault("ignore-layout");
        MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/notes/data/daily.md",
            "# Daily\n",
        )
        .expect("managed");
        std::fs::write(vault_path.join(".bentolifelayout/ignored.md"), "# Ignore\n")
            .expect("fixture");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert_eq!(scan.documents.len(), 1);
        assert_eq!(
            scan.documents[0].markdown_relative_path,
            "modules/notes/data/daily.md"
        );

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn reconnects_moved_files_by_identity() {
        let vault_path = unique_temp_vault("rename");
        let managed = MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/notes/data/daily.md",
            "# Daily\n",
        )
        .expect("managed");
        std::fs::create_dir_all(vault_path.join("modules/notes/data/renamed")).expect("folder");
        std::fs::rename(
            vault_path.join("modules/notes/data/daily.md"),
            vault_path.join("modules/notes/data/renamed/daily.md"),
        )
        .expect("moved");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");
        let metadata =
            DocumentMetadataService::read(&vault_path, &managed.document_id).expect("metadata");

        assert_eq!(
            scan.documents[0].document_id.as_deref(),
            Some(managed.document_id.as_str())
        );
        assert_eq!(metadata.current_path, "modules/notes/data/renamed/daily.md");
        assert!(metadata
            .previous_paths
            .contains(&"modules/notes/data/daily.md".to_string()));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn detects_duplicate_identity_conflicts() {
        let vault_path = unique_temp_vault("duplicate");
        let managed = MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/notes/data/daily.md",
            "# Daily\n",
        )
        .expect("managed");
        std::fs::write(
            vault_path.join("modules/notes/data/copy.md"),
            managed.markdown,
        )
        .expect("copy");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert!(scan
            .index
            .duplicate_identity_conflicts
            .contains(&managed.document_id));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn excludes_trash_archive_and_layout_storage_from_active_scan() {
        let vault_path = unique_temp_vault("skip-system-storage");
        MarkdownDocumentService::manage_document(
            &vault_path,
            "modules/notes/data/daily.md",
            "# Daily\n",
        )
        .expect("managed");
        std::fs::create_dir_all(vault_path.join("modules/trash")).expect("trash folder");
        std::fs::create_dir_all(vault_path.join("modules/archive")).expect("archive folder");
        std::fs::create_dir_all(vault_path.join(".bentolifelayout/imports/staged"))
            .expect("staged folder");
        std::fs::write(vault_path.join("modules/trash/INDEX.md"), "# Trash\n")
            .expect("trash index");
        std::fs::write(vault_path.join("modules/archive/INDEX.md"), "# Archive\n")
            .expect("archive index");
        std::fs::create_dir_all(vault_path.join("modules/navigator")).expect("navigator folder");
        std::fs::write(
            vault_path.join("modules/navigator/INDEX.md"),
            "# Navigator\n",
        )
        .expect("navigator index");
        std::fs::write(
            vault_path.join("modules/navigator/NAVIGATOR.md"),
            "# Navigator\n",
        )
        .expect("navigator document");
        std::fs::write(
            vault_path.join("modules/notes/MODULE.md"),
            "# Notes Module\n",
        )
        .expect("module doc");
        std::fs::write(
            vault_path.join(".bentolifelayout/imports/staged/imported.md"),
            "# Imported\n",
        )
        .expect("staged import");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert_eq!(scan.documents.len(), 1);
        assert_eq!(
            scan.documents[0].markdown_relative_path,
            "modules/notes/data/daily.md"
        );

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn scaffold_system_metadata_does_not_create_missing_markdown_issues() {
        let vault_path = unique_temp_vault("system-metadata");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert!(!scan
            .issues
            .iter()
            .any(|issue| issue.code == "markdown_missing"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn scaffolded_todos_index_does_not_receive_todo_schema_diagnostics() {
        let vault_path = unique_temp_vault("todos-index-not-entity");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert!(!scan.issues.iter().any(|issue| {
            issue.markdown_relative_path.as_deref() == Some("modules/todos/INDEX.md")
                && (issue.code == "unknown_schema_field"
                    || issue.code == "unknown_markdown_block"
                    || issue.code == "module_schema_missing")
        }));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn bold_list_labels_do_not_emit_schema_or_recovery_issues() {
        let vault_path = unique_temp_vault("bold-list-labels");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        crate::domain::notes::NotesService::create_note(
            &vault_path,
            "Memeguy",
            Some(
                "# Memeguy\n\n- **JavaScript/TypeScript:** roast text\n- **Vibe:** calm\n"
                    .to_string(),
            ),
        )
        .expect("note");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert!(!scan.issues.iter().any(|issue| {
            issue.code == "unknown_schema_field" || issue.classification == "recovery_issue"
        }));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn unknown_strict_schema_fields_are_warnings_not_recovery_issues() {
        let vault_path = unique_temp_vault("schema-warning-classification");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        crate::domain::notes::NotesService::create_note(
            &vault_path,
            "Unknown Field",
            Some("# Unknown Field\n\nMood: Bright\n".to_string()),
        )
        .expect("note");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert!(scan.issues.iter().any(|issue| {
            issue.code == "unknown_schema_field" && issue.classification == "schema_warning"
        }));
        assert!(!scan.issues.iter().any(|issue| {
            issue.code == "unknown_schema_field" && issue.classification == "recovery_issue"
        }));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn scaffold_plus_note_does_not_report_system_orphans() {
        let vault_path = unique_temp_vault("note-no-system-orphans");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");
        crate::domain::notes::NotesService::create_note(&vault_path, "Daily Plan", None)
            .expect("note");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert!(!scan
            .issues
            .iter()
            .any(|issue| issue.code == "markdown_missing"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn deleted_content_markdown_still_reports_missing_markdown() {
        let vault_path = unique_temp_vault("real-missing-note");
        let note = crate::domain::notes::NotesService::create_note(&vault_path, "Daily Plan", None)
            .expect("note");
        std::fs::remove_file(vault_path.join(&note.markdown_relative_path)).expect("deleted note");

        let scan = WorkspaceScanner::scan(&vault_path).expect("scan succeeds");

        assert!(scan.issues.iter().any(|issue| {
            issue.code == "markdown_missing"
                && issue.document_id.as_deref() == Some(note.document_id.as_str())
        }));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
