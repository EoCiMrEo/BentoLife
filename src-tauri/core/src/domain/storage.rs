use std::{
    fs,
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{de::DeserializeOwned, Serialize};

pub fn current_timestamp_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

pub fn generate_document_id(seed: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let process_id = std::process::id();
    let hash = stable_hash(&(seed, nanos, process_id));
    format!("bl_doc_{hash:016x}")
}

pub fn content_hash(content: &str) -> String {
    format!("{:016x}", stable_hash(&content))
}

pub fn content_hash_bytes(content: &[u8]) -> String {
    format!("{:016x}", stable_hash(&content))
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Unable to read JSON at {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("Invalid JSON at {}: {error}", path.display()))
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Unable to serialize JSON for {}: {error}", path.display()))?;
    bytes.push(b'\n');
    validate_json_bytes(&bytes, path)?;
    write_bytes_atomic(path, &bytes)
}

pub fn write_text_atomic(path: &Path, content: &str) -> Result<(), String> {
    write_bytes_atomic(path, content.as_bytes())
}

pub fn write_binary_atomic(path: &Path, content: &[u8]) -> Result<(), String> {
    write_bytes_atomic(path, content)
}

pub fn ensure_vault_relative_path(path: &str) -> Result<(), String> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return Err("Vault metadata paths must be vault-relative.".to_string());
    }

    if candidate
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("Vault metadata paths must not escape the vault.".to_string());
    }

    Ok(())
}

pub fn resolve_vault_relative_path(
    vault_path: &Path,
    relative_path: &str,
) -> Result<PathBuf, String> {
    ensure_vault_relative_path(relative_path)?;
    Ok(vault_path.join(relative_path))
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Unable to create {}: {error}", parent.display()))?;
    }

    let tmp_path = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp_path)
        .map_err(|error| format!("Unable to create {}: {error}", tmp_path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("Unable to write {}: {error}", tmp_path.display()))?;
    file.sync_all()
        .map_err(|error| format!("Unable to flush {}: {error}", tmp_path.display()))?;
    if let Err(error) = fs::rename(&tmp_path, path) {
        if path.exists() {
            fs::remove_file(path).map_err(|remove_error| {
                format!(
                    "Unable to replace {} after {error}: {remove_error}",
                    path.display()
                )
            })?;
            fs::rename(&tmp_path, path).map_err(|rename_error| {
                format!("Unable to save {}: {rename_error}", path.display())
            })?;
        } else {
            return Err(format!("Unable to save {}: {error}", path.display()));
        }
    }
    Ok(())
}

fn validate_json_bytes(bytes: &[u8], path: &Path) -> Result<(), String> {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .map(|_| ())
        .map_err(|error| format!("Invalid JSON generated for {}: {error}", path.display()))
}

fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_absolute_or_parent_relative_metadata_paths() {
        assert!(ensure_vault_relative_path("notes/daily.md").is_ok());
        assert!(ensure_vault_relative_path("../daily.md").is_err());
        assert!(ensure_vault_relative_path("C:/Users/Bento/file.md").is_err());
    }

    #[test]
    fn document_ids_use_required_prefix() {
        assert!(generate_document_id("notes/daily.md").starts_with("bl_doc_"));
    }
}
