//! Graph relationship extraction, linking, and warnings.

use std::collections::{BTreeMap, BTreeSet};

use crate::{EntityMetadata, GraphHealthWarning, GraphLink};

pub fn extract_graph_links(markdown: &str, source_path: &str) -> Vec<GraphLink> {
    let mut links = Vec::new();
    let bytes = markdown.as_bytes();
    let mut cursor = 0;

    while cursor + 3 < bytes.len() {
        if &bytes[cursor..cursor + 2] == b"[[" {
            if let Some(end) = markdown[cursor + 2..].find("]]") {
                let raw_inner = markdown[cursor + 2..cursor + 2 + end].trim();
                if !raw_inner.is_empty() {
                    let (link_type, target) = raw_inner
                        .split_once(':')
                        .map(|(kind, target)| {
                            (kind.trim().to_lowercase(), target.trim().to_string())
                        })
                        .unwrap_or_else(|| ("wiki".to_string(), raw_inner.to_string()));
                    links.push(GraphLink {
                        source_path: source_path.to_string(),
                        target,
                        link_type,
                        raw: format!("[[{raw_inner}]]"),
                        status: "unresolved".to_string(),
                        resolved_document_id: None,
                        resolved_path: None,
                    });
                }
                cursor += end + 4;
                continue;
            }
        }
        cursor += 1;
    }

    links
}

pub(crate) fn resolve_cache_graph(
    entities_by_path: &mut BTreeMap<String, EntityMetadata>,
    graph_links: &mut [GraphLink],
) {
    let mut targets = BTreeMap::<String, (Option<String>, String, String)>::new();
    for entity in entities_by_path.values() {
        let key = normalize_graph_target(&entity.title);
        targets.entry(key).or_insert((
            entity.document_id.clone(),
            entity.current_path.clone(),
            entity.entity_type.clone(),
        ));
        targets
            .entry(normalize_graph_target(&format!(
                "{}:{}",
                entity.entity_type, entity.title
            )))
            .or_insert((
                entity.document_id.clone(),
                entity.current_path.clone(),
                entity.entity_type.clone(),
            ));
        targets
            .entry(normalize_graph_target(&entity.current_path))
            .or_insert((
                entity.document_id.clone(),
                entity.current_path.clone(),
                entity.entity_type.clone(),
            ));
    }

    for link in graph_links.iter_mut() {
        resolve_graph_link(link, &targets);
    }

    for entity in entities_by_path.values_mut() {
        for link in entity.backlinks.iter_mut() {
            resolve_graph_link(link, &targets);
        }
        entity.unresolved_links = entity
            .backlinks
            .iter()
            .filter(|link| link.status != "resolved")
            .cloned()
            .collect();
    }
}

pub(crate) fn resolve_graph_link(
    link: &mut GraphLink,
    targets: &BTreeMap<String, (Option<String>, String, String)>,
) {
    let key = normalize_graph_target(&link.target);
    let resolved = targets.get(&key).or_else(|| {
        targets.iter().find_map(|(candidate, value)| {
            let type_prefix = format!("{}:", value.2);
            (candidate == &normalize_graph_target(&format!("{type_prefix}{}", link.target)))
                .then_some(value)
        })
    });

    if let Some((document_id, path, _)) = resolved {
        link.status = "resolved".to_string();
        link.resolved_document_id = document_id.clone();
        link.resolved_path = Some(path.clone());
    } else {
        link.status = "broken".to_string();
    }
}

pub(crate) fn graph_health_warnings(
    entities_by_path: &BTreeMap<String, EntityMetadata>,
    graph_links: &[GraphLink],
) -> Vec<GraphHealthWarning> {
    let mut warnings = Vec::new();
    for entity in entities_by_path.values() {
        if entity.document_id.is_none() {
            warnings.push(GraphHealthWarning {
                code: "missing_identity".to_string(),
                message: format!(
                    "{} has no hidden BentoLife document identity.",
                    entity.current_path
                ),
                document_id: None,
                path: Some(entity.current_path.clone()),
            });
        }
        if entity.metadata_path.is_none() {
            warnings.push(GraphHealthWarning {
                code: "missing_metadata".to_string(),
                message: format!(
                    "{} has no document metadata reference.",
                    entity.current_path
                ),
                document_id: entity.document_id.clone(),
                path: Some(entity.current_path.clone()),
            });
        }
    }
    for link in graph_links.iter().filter(|link| link.status != "resolved") {
        warnings.push(GraphHealthWarning {
            code: "broken_link".to_string(),
            message: format!(
                "{} references unresolved entity {}.",
                link.source_path, link.raw
            ),
            document_id: None,
            path: Some(link.source_path.clone()),
        });
    }
    warnings
}

pub(crate) fn normalize_graph_target(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(".md")
        .replace('\\', "/")
        .to_lowercase()
}

pub(crate) fn extract_tags(markdown_body: &str) -> Vec<String> {
    let mut tags = BTreeSet::new();
    for line in markdown_body.lines() {
        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.trim_start_matches("- ").split_once(':') {
            if key.trim().eq_ignore_ascii_case("tags") {
                for tag in value.split(',') {
                    let tag = tag.trim().trim_start_matches('#').to_lowercase();
                    if !tag.is_empty() {
                        tags.insert(tag);
                    }
                }
            }
        }
        for word in trimmed.split_whitespace() {
            let tag = word.trim_matches(|character: char| {
                !character.is_ascii_alphanumeric() && character != '#'
            });
            if let Some(tag) = tag.strip_prefix('#') {
                if !tag.is_empty() {
                    tags.insert(tag.to_lowercase());
                }
            }
        }
    }
    tags.into_iter().collect()
}

pub(crate) fn markdown_headings(markdown_body: &str) -> Vec<String> {
    markdown_body
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("# ")
                .or_else(|| trimmed.strip_prefix("## "))
                .or_else(|| trimmed.strip_prefix("### "))
                .map(|heading| heading.trim().to_string())
        })
        .filter(|heading| !heading.is_empty())
        .collect()
}

pub(crate) fn markdown_excerpt(markdown_body: &str) -> String {
    markdown_body
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line.starts_with('#')
                && !line.starts_with("<!--")
                && !line.starts_with("- Tags:")
        })
        .unwrap_or("No preview yet.")
        .chars()
        .take(160)
        .collect()
}
