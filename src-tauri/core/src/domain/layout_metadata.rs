use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{
    document_metadata::DocumentMetadataService,
    storage::{current_timestamp_label, read_json, write_json_atomic},
    theme::ThemeService,
};

pub const LAYOUT_METADATA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutCardMetadata {
    pub section_match: String,
    pub card_id: String,
    pub width: String,
    pub order: u32,
    pub widget: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FallbackLayoutMetadata {
    pub strategy: String,
    pub default_width: String,
    pub preserve_markdown_order: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayoutMetadata {
    pub schema_version: u32,
    pub document_id: String,
    pub vault_relative: bool,
    pub layout_mode: String,
    pub theme: String,
    pub cards: Vec<LayoutCardMetadata>,
    pub fallback_layout: FallbackLayoutMetadata,
    pub updated_at: String,
}

pub struct LayoutMetadataService;

impl LayoutMetadataService {
    pub const LAYOUTS_FOLDER: &'static str = ".bentolifelayout/layouts";
    pub const DEFAULT_LAYOUT_MODE: &'static str = "bento-dashboard";
    pub const DEFAULT_CARD_WIDTH: &'static str = "single";
    pub const DEFAULT_WIDGET: &'static str = "rich_text";

    pub fn service_name() -> &'static str {
        "LayoutMetadataService"
    }

    pub fn layout_relative_path(document_id: &str) -> String {
        format!("{}/{}.layout.json", Self::LAYOUTS_FOLDER, document_id)
    }

    pub fn layout_path(vault_path: &Path, document_id: &str) -> PathBuf {
        vault_path.join(Self::layout_relative_path(document_id))
    }

    pub fn create_default(document_id: &str) -> Result<LayoutMetadata, String> {
        DocumentMetadataService::validate_document_id(document_id)?;
        let metadata = LayoutMetadata {
            schema_version: LAYOUT_METADATA_VERSION,
            document_id: document_id.to_string(),
            vault_relative: true,
            layout_mode: Self::DEFAULT_LAYOUT_MODE.to_string(),
            theme: ThemeService::DEFAULT_THEME.to_string(),
            cards: Vec::new(),
            fallback_layout: FallbackLayoutMetadata {
                strategy: "generate_cards_from_markdown_headings".to_string(),
                default_width: Self::DEFAULT_CARD_WIDTH.to_string(),
                preserve_markdown_order: true,
            },
            updated_at: current_timestamp_label(),
        };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn generate_from_markdown(
        document_id: &str,
        markdown: &str,
    ) -> Result<LayoutMetadata, String> {
        let mut metadata = Self::create_default(document_id)?;
        metadata.cards = heading_matches(markdown)
            .into_iter()
            .enumerate()
            .map(|(index, heading)| LayoutCardMetadata {
                section_match: heading.clone(),
                card_id: unique_card_id(&heading, index),
                width: Self::DEFAULT_CARD_WIDTH.to_string(),
                order: index as u32,
                widget: Self::DEFAULT_WIDGET.to_string(),
            })
            .collect();
        metadata.updated_at = current_timestamp_label();
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn stale_section_matches(metadata: &LayoutMetadata, markdown: &str) -> Vec<String> {
        let headings: BTreeSet<String> = heading_matches(markdown).into_iter().collect();
        metadata
            .cards
            .iter()
            .filter(|card| !headings.contains(&card.section_match))
            .map(|card| card.section_match.clone())
            .collect()
    }

    pub fn read(vault_path: &Path, document_id: &str) -> Result<LayoutMetadata, String> {
        let metadata = read_json::<LayoutMetadata>(&Self::layout_path(vault_path, document_id))?;
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn write(vault_path: &Path, metadata: &LayoutMetadata) -> Result<(), String> {
        metadata.validate()?;
        write_json_atomic(
            &Self::layout_path(vault_path, &metadata.document_id),
            metadata,
        )
    }
}

impl LayoutMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != LAYOUT_METADATA_VERSION {
            return Err(format!(
                "Unsupported layout metadata version {}.",
                self.schema_version
            ));
        }
        DocumentMetadataService::validate_document_id(&self.document_id)?;
        if !self.vault_relative {
            return Err("Layout metadata must use vault-relative paths.".to_string());
        }
        if self.theme.trim().is_empty() {
            return Err("Layout metadata must include a theme.".to_string());
        }
        if self.layout_mode.trim().is_empty() {
            return Err("Layout metadata must include a layout mode.".to_string());
        }
        if !matches!(
            self.fallback_layout.default_width.as_str(),
            "single" | "double" | "full"
        ) {
            return Err("Fallback layout width must be single, double, or full.".to_string());
        }

        let mut card_ids = BTreeSet::new();
        for card in &self.cards {
            if card.section_match.trim().is_empty() {
                return Err("Layout cards must include a section match.".to_string());
            }
            if card.card_id.trim().is_empty() {
                return Err("Layout cards must include a card ID.".to_string());
            }
            if !card_ids.insert(card.card_id.clone()) {
                return Err("Layout card IDs must be unique.".to_string());
            }
            if !matches!(card.width.as_str(), "single" | "double" | "full") {
                return Err("Layout card width must be single, double, or full.".to_string());
            }
            if card.widget.trim().is_empty() {
                return Err("Layout cards must include a widget type.".to_string());
            }
        }
        Ok(())
    }
}

fn heading_matches(markdown: &str) -> Vec<String> {
    strip_bentolife_markers(markdown)
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (marks, title) = trimmed.split_once(' ')?;
            if (marks == "#" || marks == "##") && !title.trim().is_empty() {
                Some(format!("{marks} {}", title.trim()))
            } else {
                None
            }
        })
        .collect()
}

fn strip_bentolife_markers(markdown: &str) -> String {
    let mut body = strip_frontmatter(markdown);

    while let Some(start) = body.find("<!-- bentolife:document_id=") {
        let Some(relative_end) = body[start..].find("-->") else {
            break;
        };
        let end = start + relative_end + "-->".len();
        body.replace_range(start..end, "");
    }

    body
}

fn strip_frontmatter(markdown: &str) -> String {
    let Some(after_opening) = markdown.strip_prefix("---") else {
        return markdown.to_string();
    };
    if !after_opening.starts_with('\n') && !after_opening.starts_with("\r\n") {
        return markdown.to_string();
    }

    let line_start = if after_opening.starts_with("\r\n") {
        5
    } else {
        4
    };
    let content_after_open = &markdown[line_start..];
    let Some(closing_offset) = content_after_open.find("\n---") else {
        return markdown.to_string();
    };
    let closing_start = line_start + closing_offset;
    let closing_line_end = markdown[closing_start + 1..]
        .find('\n')
        .map(|offset| closing_start + 1 + offset + 1)
        .unwrap_or(markdown.len());

    markdown[closing_line_end..].to_string()
}

fn unique_card_id(section_match: &str, index: usize) -> String {
    let slug = section_match
        .trim_start_matches('#')
        .trim()
        .to_lowercase()
        .chars()
        .fold(String::new(), |mut slug, character| {
            if character.is_ascii_alphanumeric() {
                slug.push(character);
            } else if !slug.ends_with('-') {
                slug.push('-');
            }
            slug
        })
        .trim_matches('-')
        .to_string();

    format!(
        "card_{}_{}",
        index,
        if slug.is_empty() { "section" } else { &slug }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_uses_heading_fallback() {
        let metadata =
            LayoutMetadataService::create_default("bl_doc_test").expect("layout is valid");

        assert_eq!(metadata.theme, "clean-slate");
        assert_eq!(
            metadata.fallback_layout.strategy,
            "generate_cards_from_markdown_headings"
        );
    }

    #[test]
    fn generates_concrete_cards_from_h1_and_h2_headings() {
        let metadata = LayoutMetadataService::generate_from_markdown(
            "bl_doc_test",
            "---\nbentolife_metadata: .bentolifelayout/documents/bl_doc_test.json\n---\n\n# Daily\n\n## Today\n\n### Detail\n\n## Later\n",
        )
        .expect("layout generated");

        assert_eq!(
            metadata
                .cards
                .iter()
                .map(|card| card.section_match.as_str())
                .collect::<Vec<_>>(),
            vec!["# Daily", "## Today", "## Later"]
        );
        assert!(metadata.cards.iter().all(|card| card.width == "single"));
        assert!(metadata.cards.iter().all(|card| card.widget == "rich_text"));
    }

    #[test]
    fn detects_stale_layout_references() {
        let mut metadata =
            LayoutMetadataService::generate_from_markdown("bl_doc_test", "# Daily\n\n## Today\n")
                .expect("layout generated");
        metadata.cards.push(LayoutCardMetadata {
            section_match: "## Missing".to_string(),
            card_id: "missing".to_string(),
            width: "single".to_string(),
            order: 20,
            widget: "rich_text".to_string(),
        });

        let stale =
            LayoutMetadataService::stale_section_matches(&metadata, "# Daily\n\n## Today\n");

        assert_eq!(stale, vec!["## Missing".to_string()]);
    }
}
