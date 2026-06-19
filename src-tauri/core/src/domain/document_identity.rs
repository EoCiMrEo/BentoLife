use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityComment {
    pub document_id: String,
    pub comment: String,
}

pub struct DocumentIdentityService;

impl DocumentIdentityService {
    pub const COMMENT_PREFIX: &'static str = "bentolife:document_id=";
    pub const COMMENT_START: &'static str = "<!-- bentolife:document_id=";
    pub const COMMENT_END: &'static str = " -->";

    pub fn service_name() -> &'static str {
        "DocumentIdentityService"
    }

    pub fn format_comment(document_id: &str) -> String {
        format!(
            "{}{}{}",
            Self::COMMENT_START,
            document_id,
            Self::COMMENT_END
        )
    }

    pub fn find_identity_comment(markdown: &str) -> Option<IdentityComment> {
        let start = markdown.rfind(Self::COMMENT_START)?;
        let after_start = start + Self::COMMENT_START.len();
        let relative_end = markdown[after_start..].find("-->")?;
        let end = after_start + relative_end;
        let document_id = markdown[after_start..end].trim().to_string();

        if document_id.is_empty() {
            return None;
        }

        Some(IdentityComment {
            comment: markdown[start..end + "-->".len()].to_string(),
            document_id,
        })
    }

    pub fn ensure_identity_comment_at_end(markdown: &str, document_id: &str) -> String {
        let comment = Self::format_comment(document_id);
        let body_without_identity = Self::remove_identity_comments(markdown)
            .trim_end()
            .to_string();

        if body_without_identity.is_empty() {
            format!("{comment}\n")
        } else {
            format!("{body_without_identity}\n\n{comment}\n")
        }
    }

    pub fn remove_identity_comments(markdown: &str) -> String {
        let mut remaining = markdown.to_string();

        while let Some(start) = remaining.find(Self::COMMENT_START) {
            let after_start = start + Self::COMMENT_START.len();
            let Some(relative_end) = remaining[after_start..].find("-->") else {
                break;
            };
            let end = after_start + relative_end + "-->".len();
            remaining.replace_range(start..end, "");
        }

        remaining
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_existing_identity_comment() {
        let markdown = "# Daily\n\n<!-- bentolife:document_id=bl_doc_daily -->\n";
        let identity =
            DocumentIdentityService::find_identity_comment(markdown).expect("identity exists");

        assert_eq!(identity.document_id, "bl_doc_daily");
    }

    #[test]
    fn inserts_identity_comment_at_end() {
        let markdown = "# Daily\n";
        let managed =
            DocumentIdentityService::ensure_identity_comment_at_end(markdown, "bl_doc_daily");

        assert!(managed.ends_with("<!-- bentolife:document_id=bl_doc_daily -->\n"));
    }
}
