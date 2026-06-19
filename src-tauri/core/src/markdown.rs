//! Markdown parsing: frontmatter extraction, identity comment handling.

use crate::{
    IdentityAnchor, ParsedFrontmatter, FRONTMATTER_REFERENCE_KEY, IDENTITY_COMMENT_END,
    IDENTITY_COMMENT_START,
};

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

pub fn find_identity_comment(markdown: &str) -> Option<IdentityAnchor> {
    let start = markdown.rfind(IDENTITY_COMMENT_START)?;
    let after_start = start + IDENTITY_COMMENT_START.len();
    let relative_end = markdown[after_start..].find("-->")?;
    let end = after_start + relative_end;
    let document_id = markdown[after_start..end].trim().to_string();

    if document_id.is_empty() {
        return None;
    }

    Some(IdentityAnchor {
        comment: markdown[start..end + "-->".len()].to_string(),
        document_id,
    })
}

pub fn format_identity_comment(document_id: &str) -> String {
    format!("{IDENTITY_COMMENT_START}{document_id}{IDENTITY_COMMENT_END}")
}

pub fn remove_identity_comments(markdown: &str) -> String {
    let mut remaining = markdown.to_string();

    while let Some(start) = remaining.find(IDENTITY_COMMENT_START) {
        let after_start = start + IDENTITY_COMMENT_START.len();
        let Some(relative_end) = remaining[after_start..].find("-->") else {
            break;
        };
        let end = after_start + relative_end + "-->".len();
        remaining.replace_range(start..end, "");
    }

    remaining
}

pub fn ensure_identity_comment_at_end(markdown: &str, document_id: &str) -> String {
    let comment = format_identity_comment(document_id);
    let body_without_identity = remove_identity_comments(markdown).trim_end().to_string();

    if body_without_identity.is_empty() {
        format!("{comment}\n")
    } else {
        format!("{body_without_identity}\n\n{comment}\n")
    }
}

pub(crate) fn frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
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

pub(crate) fn with_frontmatter(markdown_body: &str, metadata_path: &str) -> String {
    let parsed = parse_frontmatter(markdown_body);
    format!(
        "---\nbentolife_metadata: {metadata_path}\n---\n\n{}",
        parsed.body.trim_start()
    )
}
