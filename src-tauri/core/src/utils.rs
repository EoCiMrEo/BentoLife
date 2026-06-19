//! Low-level utility functions: hashing, timestamps, path helpers, JSON I/O,
//! slug generation, and atomic file writes. These are leaf helpers with no
//! domain dependencies.

use std::{
    collections::BTreeSet,
    fs,
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{de::DeserializeOwned, Serialize};

// ── Timestamps ──────────────────────────────────────────────────────────────

pub fn current_timestamp_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

// ── Hashing ─────────────────────────────────────────────────────────────────

pub fn content_hash(content: &str) -> String {
    format!("{:016x}", stable_hash(&content))
}

pub fn content_hash_bytes(content: &[u8]) -> String {
    format!("{:016x}", stable_hash(&content))
}

pub(crate) fn stable_hash<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub(crate) fn generate_core_document_id(seed: &str) -> String {
    format!("bl_doc_{}", content_hash(seed))
}

// ── Path helpers ────────────────────────────────────────────────────────────

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

pub(crate) fn vault_relative_path(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|error| format!("Unable to make {} vault-relative: {error}", path.display()))
}

pub(crate) fn normalize_relative(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_string()
}

pub(crate) fn canonicalize_existing(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("Unable to resolve {}: {error}", path.display()))
}

pub(crate) fn normalize_path(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        canonicalize_existing(path)
    } else if let Some(parent) = path.parent() {
        let parent = if parent.exists() {
            canonicalize_existing(parent)?
        } else {
            parent.to_path_buf()
        };
        Ok(parent.join(path.file_name().unwrap_or_default()))
    } else {
        Ok(path.to_path_buf())
    }
}

pub(crate) fn unique_relative_path(path: &str, occupied: &mut BTreeSet<String>) -> String {
    let normalized = normalize_relative(path);
    if occupied.insert(normalized.clone()) {
        return normalized;
    }

    let path = Path::new(&normalized);
    let parent = path
        .parent()
        .map(|parent| parent.to_string_lossy().replace('\\', "/"));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("file");
    let extension = path.extension().and_then(|extension| extension.to_str());
    for counter in 2.. {
        let name = match extension {
            Some(extension) => format!("{stem}-{counter}.{extension}"),
            None => format!("{stem}-{counter}"),
        };
        let candidate = match &parent {
            Some(parent) if !parent.is_empty() => format!("{parent}/{name}"),
            _ => name,
        };
        if occupied.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!()
}

// ── Slug helpers ────────────────────────────────────────────────────────────

pub(crate) fn slugify(value: &str) -> String {
    let slug = value
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let mut compact = String::new();
    let mut previous_dash = false;
    for character in slug.chars() {
        if character == '-' {
            if !previous_dash {
                compact.push(character);
            }
            previous_dash = true;
        } else {
            compact.push(character);
            previous_dash = false;
        }
    }
    let compact = compact.trim_matches('-').to_string();
    if compact.is_empty() {
        "untitled".to_string()
    } else {
        compact
    }
}

#[allow(dead_code)]
pub(crate) fn slug_from_path(path: &str) -> String {
    normalize_relative(path)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

// ── JSON I/O ────────────────────────────────────────────────────────────────

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("Unable to serialize JSON for {}: {error}", path.display()))?;
    bytes.push(b'\n');
    serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|error| format!("Invalid JSON generated for {}: {error}", path.display()))?;
    write_bytes_atomic(path, &bytes)
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Unable to read JSON at {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("Invalid JSON at {}: {error}", path.display()))
}

pub fn write_text_atomic(path: &Path, content: &str) -> Result<(), String> {
    write_bytes_atomic(path, content.as_bytes())
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

pub(crate) fn copy_file_verified(source_file: &Path, target_file: &Path) -> Result<(), String> {
    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Unable to create {}: {error}", parent.display()))?;
    }
    fs::copy(source_file, target_file).map_err(|error| {
        format!(
            "Unable to copy {} to {}: {error}",
            source_file.display(),
            target_file.display()
        )
    })?;
    let source_hash = content_hash_bytes(
        &fs::read(source_file)
            .map_err(|error| format!("Unable to verify {}: {error}", source_file.display()))?,
    );
    let target_hash = content_hash_bytes(
        &fs::read(target_file)
            .map_err(|error| format!("Unable to verify {}: {error}", target_file.display()))?,
    );
    if source_hash != target_hash {
        return Err(format!(
            "Copy verification failed for {} to {}.",
            source_file.display(),
            target_file.display()
        ));
    }
    Ok(())
}

// ── File kind classification ────────────────────────────────────────────────

pub(crate) fn file_kind_for_path(path: &Path) -> String {
    if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
        "markdown".to_string()
    } else {
        "asset".to_string()
    }
}
