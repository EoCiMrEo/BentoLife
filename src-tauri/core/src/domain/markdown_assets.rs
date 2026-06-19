use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    document_metadata::DocumentMetadataService,
    storage::{
        content_hash_bytes, current_timestamp_label, resolve_vault_relative_path,
        write_binary_atomic,
    },
};

const MAX_MARKDOWN_ASSET_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownAsset {
    pub module_id: String,
    pub document_id: String,
    pub vault_relative_path: String,
    pub markdown_link: String,
    pub content_hash: String,
    pub byte_len: u64,
    pub mime_type: String,
}

pub struct MarkdownAssetService;

impl MarkdownAssetService {
    pub fn save_markdown_asset(
        vault_path: &Path,
        module_id: &str,
        document_id: &str,
        file_name: Option<String>,
        mime_type: &str,
        bytes: Vec<u8>,
    ) -> Result<MarkdownAsset, String> {
        if bytes.is_empty() {
            return Err("Pasted image was empty.".to_string());
        }
        if bytes.len() > MAX_MARKDOWN_ASSET_BYTES {
            return Err("Pasted image is larger than the 10 MB Markdown asset limit.".to_string());
        }

        let extension = extension_for_mime(mime_type)?;
        let module_id = safe_segment(module_id, "module")?;
        let metadata = DocumentMetadataService::read(vault_path, document_id)?;
        if !metadata
            .current_path
            .starts_with(&format!("modules/{module_id}/"))
        {
            return Err("Markdown asset module does not match the target document.".to_string());
        }

        let content_hash = content_hash_bytes(&bytes);
        let safe_stem = file_name
            .as_deref()
            .and_then(file_stem)
            .map(safe_file_stem)
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| "pasted-image".to_string());
        let timestamp = current_timestamp_label().replace(':', "-");
        let short_hash = &content_hash[..8.min(content_hash.len())];
        let asset_folder = format!("assets/{module_id}/{document_id}");

        let mut selected_relative_path = None;
        for counter in 0..100 {
            let suffix = if counter == 0 {
                String::new()
            } else {
                format!("-{counter}")
            };
            let candidate =
                format!("{asset_folder}/{timestamp}-{short_hash}-{safe_stem}{suffix}.{extension}");
            let candidate_path = resolve_vault_relative_path(vault_path, &candidate)?;
            if !candidate_path.exists() {
                selected_relative_path = Some(candidate);
                break;
            }
        }
        let vault_relative_path = selected_relative_path
            .ok_or_else(|| "Unable to allocate a unique asset path.".to_string())?;
        let target_path = resolve_vault_relative_path(vault_path, &vault_relative_path)?;
        write_binary_atomic(&target_path, &bytes)?;
        let markdown_link = relative_markdown_link(&metadata.current_path, &vault_relative_path)?;

        Ok(MarkdownAsset {
            module_id,
            document_id: document_id.to_string(),
            vault_relative_path,
            markdown_link,
            content_hash,
            byte_len: bytes.len() as u64,
            mime_type: mime_type.to_string(),
        })
    }

    pub fn read_markdown_asset(
        vault_path: &Path,
        module_id: &str,
        document_id: &str,
        source: &str,
    ) -> Result<MarkdownAssetRead, String> {
        let module_id = safe_segment(module_id, "module")?;
        let metadata = DocumentMetadataService::read(vault_path, document_id)?;
        if !metadata
            .current_path
            .starts_with(&format!("modules/{module_id}/"))
        {
            return Err("Markdown asset module does not match the target document.".to_string());
        }

        let vault_relative_path = resolve_markdown_asset_source(&metadata.current_path, source)?;
        let expected_prefix = format!("assets/{module_id}/{document_id}/");
        let document_folder = metadata
            .current_path
            .replace('\\', "/")
            .rsplit_once('/')
            .map(|(folder, _)| format!("{folder}/"))
            .unwrap_or_default();
        if !vault_relative_path.starts_with(&expected_prefix)
            && !vault_relative_path.starts_with(&document_folder)
        {
            return Err(
                "Markdown asset source is outside the document folder or document asset folder."
                    .to_string(),
            );
        }

        let mime_type = mime_for_asset_path(&vault_relative_path)?;
        let target_path = resolve_vault_relative_path(vault_path, &vault_relative_path)?;
        let bytes = fs::read(&target_path).map_err(|error| {
            format!(
                "Unable to read Markdown asset {}: {error}",
                vault_relative_path
            )
        })?;
        if bytes.len() > MAX_MARKDOWN_ASSET_BYTES {
            return Err("Markdown asset is larger than the 10 MB preview limit.".to_string());
        }

        Ok(MarkdownAssetRead {
            module_id,
            document_id: document_id.to_string(),
            vault_relative_path,
            mime_type: mime_type.to_string(),
            byte_len: bytes.len() as u64,
            bytes,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarkdownAssetRead {
    pub module_id: String,
    pub document_id: String,
    pub vault_relative_path: String,
    pub mime_type: String,
    pub byte_len: u64,
    pub bytes: Vec<u8>,
}

fn extension_for_mime(mime_type: &str) -> Result<&'static str, String> {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/png" => Ok("png"),
        "image/jpeg" | "image/jpg" => Ok("jpg"),
        "image/webp" => Ok("webp"),
        _ => Err("Only pasted PNG, JPEG, and WEBP images are supported.".to_string()),
    }
}

fn mime_for_asset_path(path: &str) -> Result<&'static str, String> {
    let extension = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        _ => Err("Only PNG, JPEG, and WEBP Markdown assets can be previewed.".to_string()),
    }
}

fn safe_segment(value: &str, label: &str) -> Result<String, String> {
    let normalized = value.trim();
    if normalized.is_empty()
        || !normalized.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
    {
        return Err(format!(
            "Invalid {label} identifier for Markdown asset storage."
        ));
    }
    Ok(normalized.to_string())
}

fn file_stem(file_name: &str) -> Option<&str> {
    let file_name = file_name.rsplit(['/', '\\']).next()?;
    file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .or(Some(file_name))
}

fn safe_file_stem(value: &str) -> String {
    let mut stem = String::new();
    let mut last_dash = false;
    for character in value.trim().to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            stem.push(character);
            last_dash = false;
        } else if !last_dash {
            stem.push('-');
            last_dash = true;
        }
    }
    stem.trim_matches('-').to_string()
}

fn relative_markdown_link(
    source_relative_path: &str,
    asset_relative_path: &str,
) -> Result<String, String> {
    let source_parent_depth = source_relative_path
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty())
        .count()
        .saturating_sub(1);
    let prefix = if source_parent_depth == 0 {
        String::new()
    } else {
        "../".repeat(source_parent_depth)
    };
    Ok(format!("{prefix}{asset_relative_path}"))
}

fn resolve_markdown_asset_source(
    document_relative_path: &str,
    source: &str,
) -> Result<String, String> {
    let source = source.trim().replace('\\', "/");
    let lower = source.to_ascii_lowercase();
    if source.is_empty()
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("data:")
        || lower.starts_with("javascript:")
        || lower.starts_with("file:")
        || Path::new(&source).is_absolute()
    {
        return Err("Markdown asset source must be a safe vault-relative image path.".to_string());
    }

    let document_path = document_relative_path.replace('\\', "/");
    let document_parts: Vec<&str> = document_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();
    let mut parts: Vec<String> = if source.starts_with("assets/") {
        Vec::new()
    } else {
        document_parts
            .iter()
            .take(document_parts.len().saturating_sub(1))
            .map(|part| (*part).to_string())
            .collect()
    };

    for part in source.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("Markdown asset source must not escape the vault.".to_string());
                }
            }
            _ if part.contains(':') => {
                return Err("Markdown asset source must not contain a URL scheme.".to_string());
            }
            _ => parts.push(part.to_string()),
        }
    }

    if parts.first().map(String::as_str) != Some("assets") {
        return Err("Markdown asset source must resolve under assets/.".to_string());
    }

    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::notes::NotesService;

    fn unique_temp_vault(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-assets-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    #[test]
    fn saves_note_image_asset_with_relative_markdown_link() {
        let vault_path = unique_temp_vault("save");
        let note = NotesService::create_note(&vault_path, "Asset Note", None).expect("note");

        let asset = MarkdownAssetService::save_markdown_asset(
            &vault_path,
            "notes",
            &note.document_id,
            Some("Paste.PNG".to_string()),
            "image/png",
            vec![137, 80, 78, 71],
        )
        .expect("asset");

        assert!(asset.vault_relative_path.starts_with("assets/notes/"));
        assert!(asset.markdown_link.starts_with("../../../assets/notes/"));
        assert!(vault_path.join(asset.vault_relative_path).is_file());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rejects_unsafe_mime_types() {
        let vault_path = unique_temp_vault("mime");
        let note = NotesService::create_note(&vault_path, "Asset Note", None).expect("note");
        let result = MarkdownAssetService::save_markdown_asset(
            &vault_path,
            "notes",
            &note.document_id,
            Some("icon.svg".to_string()),
            "image/svg+xml",
            b"<svg></svg>".to_vec(),
        );

        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn reads_note_image_asset_from_relative_markdown_link() {
        let vault_path = unique_temp_vault("read");
        let note = NotesService::create_note(&vault_path, "Asset Note", None).expect("note");
        let asset = MarkdownAssetService::save_markdown_asset(
            &vault_path,
            "notes",
            &note.document_id,
            Some("Paste.PNG".to_string()),
            "image/png",
            vec![137, 80, 78, 71],
        )
        .expect("asset");

        let read = MarkdownAssetService::read_markdown_asset(
            &vault_path,
            "notes",
            &note.document_id,
            &asset.markdown_link,
        )
        .expect("read asset");

        assert_eq!(read.vault_relative_path, asset.vault_relative_path);
        assert_eq!(read.mime_type, "image/png");
        assert_eq!(read.bytes, vec![137, 80, 78, 71]);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rejects_asset_reads_outside_document_asset_folder() {
        let vault_path = unique_temp_vault("read-unsafe");
        let note = NotesService::create_note(&vault_path, "Asset Note", None).expect("note");

        let result = MarkdownAssetService::read_markdown_asset(
            &vault_path,
            "notes",
            &note.document_id,
            "../../../assets/notes/other_doc/image.png",
        );

        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
