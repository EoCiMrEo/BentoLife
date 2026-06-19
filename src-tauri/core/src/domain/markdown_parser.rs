use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum MarkdownBlock {
    Heading {
        level: usize,
        text: String,
    },
    Paragraph {
        text: String,
    },
    Blockquote {
        children: Vec<MarkdownBlock>,
    },
    HorizontalRule,
    List {
        items: Vec<String>,
    },
    OrderedList {
        items: Vec<String>,
    },
    Checklist {
        items: Vec<ChecklistItem>,
    },
    Code {
        language: String,
        content: String,
    },
    Image {
        alt: String,
        source: String,
        raw: String,
    },
    Table {
        rows: Vec<Vec<String>>,
    },
    Tags {
        tags: Vec<String>,
    },
    Relationships {
        links: Vec<String>,
    },
    Managed {
        name: String,
        content: String,
    },
    Unknown {
        raw: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChecklistItem {
    pub text: String,
    pub checked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedEntityContract {
    pub module_id: Option<String>,
    pub entity_type: Option<String>,
    pub fields: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub field_descriptors: Vec<ParsedFieldDescriptor>,
    pub blocks: Vec<MarkdownBlock>,
    pub unknown_blocks: Vec<MarkdownBlock>,
    pub relationships: Vec<String>,
    pub tags: Vec<String>,
    pub path: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedFieldDescriptor {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub renderer_id: String,
    pub value: String,
    pub editable: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse(markdown_body: &str) -> ParsedEntityContract {
        let mut blocks = Vec::new();
        let mut unknown_blocks = Vec::new();
        let mut tags = Vec::new();
        let mut relationships = Vec::new();
        let mut fields = std::collections::HashMap::new();
        let mut current_paragraph = Vec::new();

        let mut lines = markdown_body.lines().peekable();
        while let Some(line) = lines.next() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                continue;
            }

            if trimmed.starts_with('#') {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let level = trimmed.chars().take_while(|c| *c == '#').count();
                if level > 0 && level <= 6 && trimmed.chars().nth(level) == Some(' ') {
                    blocks.push(MarkdownBlock::Heading {
                        level,
                        text: trimmed[level + 1..].trim().to_string(),
                    });
                } else {
                    let block = MarkdownBlock::Unknown {
                        raw: line.to_string(),
                    };
                    unknown_blocks.push(block.clone());
                    blocks.push(block);
                }
            } else if is_horizontal_rule(trimmed) {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                blocks.push(MarkdownBlock::HorizontalRule);
            } else if let Some(language) = trimmed.strip_prefix("```") {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let language = language.trim().to_string();
                let mut content = Vec::new();
                for code_line in lines.by_ref() {
                    if code_line.trim().starts_with("```") {
                        break;
                    }
                    content.push(code_line.to_string());
                }
                blocks.push(MarkdownBlock::Code {
                    language,
                    content: content.join("\n"),
                });
            } else if let Some((alt, source)) = parse_safe_image(trimmed) {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                blocks.push(MarkdownBlock::Image {
                    alt,
                    source,
                    raw: line.to_string(),
                });
            } else if let Some(source) = parse_obsidian_image(trimmed) {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                blocks.push(MarkdownBlock::Image {
                    alt: source.clone(),
                    source,
                    raw: line.to_string(),
                });
            } else if is_markdown_image(trimmed) {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let block = MarkdownBlock::Unknown {
                    raw: line.to_string(),
                };
                unknown_blocks.push(block.clone());
                blocks.push(block);
            } else if is_table_separator(trimmed) && !blocks.is_empty() {
                let block = MarkdownBlock::Unknown {
                    raw: line.to_string(),
                };
                unknown_blocks.push(block.clone());
                blocks.push(block);
            } else if is_table_row(trimmed) {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let mut table_lines = vec![trimmed.to_string()];
                while let Some(next_line) = lines.peek() {
                    let next_trimmed = next_line.trim();
                    if is_table_row(next_trimmed) || is_table_separator(next_trimmed) {
                        table_lines.push(next_trimmed.to_string());
                        lines.next();
                    } else {
                        break;
                    }
                }
                let rows = table_lines
                    .into_iter()
                    .filter(|row| !is_table_separator(row))
                    .map(|row| parse_table_row(&row))
                    .filter(|row| !row.is_empty())
                    .collect::<Vec<_>>();
                if rows.is_empty() {
                    let block = MarkdownBlock::Unknown {
                        raw: line.to_string(),
                    };
                    unknown_blocks.push(block.clone());
                    blocks.push(block);
                } else {
                    blocks.push(MarkdownBlock::Table { rows });
                }
            } else if trimmed.starts_with("- [ ]")
                || trimmed.starts_with("- [x]")
                || trimmed.starts_with("- [X]")
                || trimmed.starts_with("* [ ]")
                || trimmed.starts_with("* [x]")
                || trimmed.starts_with("* [X]")
            {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let mut items = Vec::new();
                let is_checked = trimmed[3..4].to_lowercase() == "x";
                let text = trimmed[5..].trim().to_string();
                items.push(ChecklistItem {
                    text,
                    checked: is_checked,
                });
                while let Some(next_line) = lines.peek() {
                    let next_trimmed = next_line.trim();
                    if next_trimmed.starts_with("- [ ]")
                        || next_trimmed.starts_with("- [x]")
                        || next_trimmed.starts_with("- [X]")
                        || next_trimmed.starts_with("* [ ]")
                        || next_trimmed.starts_with("* [x]")
                        || next_trimmed.starts_with("* [X]")
                    {
                        let is_checked = next_trimmed[3..4].to_lowercase() == "x";
                        let text = next_trimmed[5..].trim().to_string();
                        items.push(ChecklistItem {
                            text,
                            checked: is_checked,
                        });
                        lines.next();
                    } else {
                        break;
                    }
                }
                blocks.push(MarkdownBlock::Checklist { items });
            } else if trimmed.starts_with('>') {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let mut quote_lines = Vec::new();
                quote_lines.push(trimmed.trim_start_matches('>').trim_start().to_string());
                while let Some(next_line) = lines.peek() {
                    let next_trimmed = next_line.trim();
                    if !next_trimmed.starts_with('>') {
                        break;
                    }
                    quote_lines.push(
                        next_trimmed
                            .trim_start_matches('>')
                            .trim_start()
                            .to_string(),
                    );
                    lines.next();
                }
                let quote = MarkdownParser::parse(&quote_lines.join("\n"));
                blocks.push(MarkdownBlock::Blockquote {
                    children: quote.blocks,
                });
            } else if let Some((key, value)) = parse_strict_field_line(trimmed) {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                if key == "tags" {
                    let current_tags: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    tags.extend(current_tags.clone());
                    blocks.push(MarkdownBlock::Tags { tags: current_tags });
                } else if key == "relationships" || key == "related" {
                    let current_rels: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    relationships.extend(current_rels.clone());
                    blocks.push(MarkdownBlock::Relationships {
                        links: current_rels,
                    });
                }
                fields.insert(key, value);
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let mut items = Vec::new();
                items.push(trimmed[2..].trim().to_string());
                while let Some(next_line) = lines.peek() {
                    let next_trimmed = next_line.trim();
                    if (next_trimmed.starts_with("- ") || next_trimmed.starts_with("* "))
                        && !next_trimmed.starts_with("- [")
                        && !next_trimmed.starts_with("* [")
                    {
                        items.push(next_trimmed[2..].trim().to_string());
                        lines.next();
                    } else {
                        break;
                    }
                }
                blocks.push(MarkdownBlock::List { items });
            } else if let Some(item) = parse_ordered_list_item(trimmed) {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let mut items = vec![item];
                while let Some(next_line) = lines.peek() {
                    let next_trimmed = next_line.trim();
                    if let Some(next_item) = parse_ordered_list_item(next_trimmed) {
                        items.push(next_item);
                        lines.next();
                    } else {
                        break;
                    }
                }
                blocks.push(MarkdownBlock::OrderedList { items });
            } else if trimmed.starts_with("<!-- bentolife:managed-block start") {
                flush_paragraph(&mut blocks, &mut current_paragraph);
                let name =
                    extract_managed_block_name(trimmed).unwrap_or_else(|| "unknown".to_string());
                let mut content = Vec::new();
                for managed_line in lines.by_ref() {
                    if managed_line
                        .trim()
                        .starts_with("<!-- bentolife:managed-block end")
                    {
                        break;
                    }
                    content.push(managed_line.to_string());
                }
                blocks.push(MarkdownBlock::Managed {
                    name,
                    content: content.join("\n"),
                });
            } else if trimmed.starts_with("<!-- bentolife:document_id=")
                || trimmed.starts_with("<!-- bentolife:import_context")
            {
                // App metadata comments are preserved in source Markdown but not rendered as content.
                continue;
            } else {
                current_paragraph.push(line.to_string());
            }
        }
        flush_paragraph(&mut blocks, &mut current_paragraph);

        // The title field is the first H1
        if let Some(MarkdownBlock::Heading { text, level: _ }) = blocks
            .iter()
            .find(|b| matches!(b, MarkdownBlock::Heading { level: 1, .. }))
        {
            fields.insert("title".to_string(), text.clone());
        }

        ParsedEntityContract {
            module_id: None,
            entity_type: None,
            fields,
            field_descriptors: Vec::new(),
            blocks,
            unknown_blocks,
            relationships,
            tags,
            path: String::new(),
            content_hash: String::new(),
        }
    }
}

fn flush_paragraph(blocks: &mut Vec<MarkdownBlock>, current_paragraph: &mut Vec<String>) {
    if !current_paragraph.is_empty() {
        blocks.push(MarkdownBlock::Paragraph {
            text: current_paragraph.join("\n"),
        });
        current_paragraph.clear();
    }
}

fn is_table_row(line: &str) -> bool {
    line.starts_with('|') && line.ends_with('|') && line.matches('|').count() >= 2
}

fn is_table_separator(line: &str) -> bool {
    is_table_row(line)
        && line.trim_matches('|').split('|').all(|cell| {
            cell.trim()
                .chars()
                .all(|character| character == '-' || character == ':')
        })
}

fn is_horizontal_rule(line: &str) -> bool {
    matches!(line, "---" | "***" | "___")
}

fn parse_table_row(line: &str) -> Vec<String> {
    line.trim_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn parse_strict_field_line(line: &str) -> Option<(String, String)> {
    if line.starts_with('-')
        || line.starts_with('*')
        || line.starts_with('+')
        || line.starts_with('>')
        || line.starts_with('#')
        || line.starts_with('|')
        || line.starts_with("```")
    {
        return None;
    }

    let (label, value) = line.split_once(':')?;
    let label = label.trim();
    let value = value.trim();
    if label.is_empty()
        || value.is_empty()
        || label.starts_with("**")
        || label.ends_with("**")
        || label.starts_with('*')
        || label.ends_with('*')
        || label.starts_with('_')
        || label.ends_with('_')
    {
        return None;
    }
    if label.chars().any(|character| {
        !(character.is_alphanumeric() || matches!(character, ' ' | '_' | '-' | '/'))
    }) {
        return None;
    }
    Some((label.to_lowercase(), value.to_string()))
}

fn parse_safe_image(line: &str) -> Option<(String, String)> {
    if !is_markdown_image(line) {
        return None;
    }
    let after_open = &line[2..];
    let (alt, after_alt) = after_open.split_once("](")?;
    let source = after_alt.strip_suffix(')')?.trim();
    if source.is_empty() || is_unsafe_image_source(source) {
        return None;
    }
    Some((alt.to_string(), source.to_string()))
}

fn parse_obsidian_image(line: &str) -> Option<String> {
    let source = line.strip_prefix("![[")?.strip_suffix("]]")?.trim();
    if source.is_empty() || is_unsafe_image_source(source) {
        return None;
    }
    Some(source.to_string())
}

fn is_markdown_image(line: &str) -> bool {
    line.starts_with("![") && line.ends_with(')') && line.contains("](")
}

fn is_unsafe_image_source(source: &str) -> bool {
    let lowered = source.trim().to_ascii_lowercase();
    lowered.starts_with("http://")
        || lowered.starts_with("https://")
        || lowered.starts_with("data:")
        || lowered.starts_with("javascript:")
        || lowered.starts_with("file:")
        || lowered.starts_with('/')
        || lowered.contains('\\')
        || lowered.ends_with(".svg")
}

fn parse_ordered_list_item(line: &str) -> Option<String> {
    let (prefix, text) = line.split_once(". ")?;
    if prefix.is_empty() || !prefix.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    Some(text.trim().to_string())
}

fn extract_managed_block_name(line: &str) -> Option<String> {
    let prefix = "name=\"";
    if let Some(start) = line.find(prefix) {
        let after_start = &line[start + prefix.len()..];
        if let Some(end) = after_start.find('"') {
            return Some(after_start[..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_safe_relative_markdown_images() {
        let parsed = MarkdownParser::parse(
            "# Note\n\n![Pasted image](../../../assets/notes/bl_doc/image.png)\n",
        );

        assert!(parsed.blocks.iter().any(|block| matches!(
            block,
            MarkdownBlock::Image { alt, source, raw }
                if alt == "Pasted image"
                    && source == "../../../assets/notes/bl_doc/image.png"
                    && raw.contains("Pasted image")
        )));
    }

    #[test]
    fn preserves_remote_images_as_unknown_blocks() {
        let parsed = MarkdownParser::parse("# Note\n\n![Remote](https://example.com/image.png)\n");

        assert!(parsed.blocks.iter().any(|block| matches!(block, MarkdownBlock::Unknown { raw } if raw.contains("https://example.com"))));
        assert!(!parsed.unknown_blocks.is_empty());
    }

    #[test]
    fn ignores_import_context_comments_as_app_metadata() {
        let parsed = MarkdownParser::parse(
            "# Imported\n\n<!-- bentolife:import_context Source: modules/todos/data/task.md #imported -->\n",
        );

        assert!(parsed.blocks.iter().any(
            |block| matches!(block, MarkdownBlock::Heading { text, .. } if text == "Imported")
        ));
        assert!(parsed
            .blocks
            .iter()
            .all(|block| !matches!(block, MarkdownBlock::Unknown { raw } if raw.contains("import_context"))));
        assert!(parsed.unknown_blocks.is_empty());
    }

    #[test]
    fn parses_only_strict_top_level_field_lines() {
        let parsed = MarkdownParser::parse(
            "# Task\n\nStatus: Done\nPriority: Medium\nDue date: 2026-06-15\nTags: alpha, product\nRelationships: [[Note:Launch]]\n",
        );

        assert_eq!(
            parsed.fields.get("status").map(String::as_str),
            Some("Done")
        );
        assert_eq!(
            parsed.fields.get("priority").map(String::as_str),
            Some("Medium")
        );
        assert_eq!(
            parsed.fields.get("due date").map(String::as_str),
            Some("2026-06-15")
        );
        assert_eq!(
            parsed.tags,
            vec!["alpha".to_string(), "product".to_string()]
        );
        assert_eq!(parsed.relationships, vec!["[[Note:Launch]]".to_string()]);
    }

    #[test]
    fn preserves_bold_and_list_labels_with_colons_as_markdown_content() {
        let parsed = MarkdownParser::parse(
            "# Roast\n\n**Vibe:** calm\n\n- **JavaScript/TypeScript:** nested label\n- **Phase 4:** parser guards\n1. Status: not metadata\n> Priority: not metadata\n",
        );

        assert!(!parsed.fields.contains_key("**vibe"));
        assert!(!parsed.fields.contains_key("**javascript/typescript"));
        assert!(!parsed.fields.contains_key("**phase 4"));
        assert!(!parsed.fields.contains_key("status"));
        assert!(!parsed.fields.contains_key("priority"));
        assert!(parsed.blocks.iter().any(|block| matches!(block, MarkdownBlock::List { items } if items.iter().any(|item| item.contains("JavaScript/TypeScript")))));
        assert!(parsed.blocks.iter().any(
            |block| matches!(block, MarkdownBlock::Paragraph { text } if text.contains("**Vibe:**"))
        ));
    }

    #[test]
    fn ignores_field_like_text_inside_fenced_code() {
        let parsed = MarkdownParser::parse("# Code\n\n```text\nStatus: Done\n```\n");

        assert!(!parsed.fields.contains_key("status"));
        assert!(parsed.blocks.iter().any(|block| matches!(block, MarkdownBlock::Code { content, .. } if content.contains("Status: Done"))));
    }

    #[test]
    fn keeps_headings_with_colons_out_of_fields() {
        let parsed = MarkdownParser::parse("# Status: Done\n\n## Phase 4: Parser\n");

        assert!(!parsed.fields.contains_key("status"));
        assert!(parsed.blocks.iter().any(
            |block| matches!(block, MarkdownBlock::Heading { text, .. } if text == "Status: Done")
        ));
    }

    #[test]
    fn parses_alpha_preview_markdown_blocks() {
        let parsed = MarkdownParser::parse(
            "# Note\n\n> Quote\n\n---\n\n1. First\n2. Second\n\n![[Pasted image.png]]\n",
        );

        assert!(parsed
            .blocks
            .iter()
            .any(|block| matches!(block, MarkdownBlock::Blockquote { .. })));
        assert!(parsed
            .blocks
            .iter()
            .any(|block| matches!(block, MarkdownBlock::HorizontalRule)));
        assert!(parsed.blocks.iter().any(
            |block| matches!(block, MarkdownBlock::OrderedList { items } if items.len() == 2)
        ));
        assert!(parsed.blocks.iter().any(|block| matches!(
            block,
            MarkdownBlock::Image { source, raw, .. } if source == "Pasted image.png" && raw.contains("![[")
        )));
    }
}
