use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use super::{
    document_identity::DocumentIdentityService,
    document_metadata::{DocumentMetadata, DocumentMetadataService},
    layout_folder::LayoutFolderService,
    layout_metadata::{LayoutMetadata, LayoutMetadataService},
    markdown_document::MarkdownDocumentService,
    markdown_parser::{MarkdownParser, ParsedEntityContract},
    module_schema::apply_schema_descriptors,
    storage::{
        content_hash, current_timestamp_label, generate_document_id, resolve_vault_relative_path,
        write_text_atomic,
    },
    workspace_metadata::WorkspaceMetadataService,
};

const CONTACTS_RELATIVE_PATH: &str = "modules/contacts.md";
const CONTACT_LEGACY_ENTITY_FOLDER: &str = "modules/contacts";
const CONTACT_DATA_FOLDER: &str = "modules/contacts/data";
const CONTACT_INDEX_PATH: &str = "modules/contacts/INDEX.md";
const DEFAULT_CONTACTS_MARKDOWN: &str = "# Contacts\n";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactInput {
    pub name: String,
    pub relationship: Option<String>,
    #[serde(default)]
    pub organization: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub relationships: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactEntry {
    pub contact_id: String,
    pub name: String,
    pub relationship: Option<String>,
    pub organization: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub tags: Vec<String>,
    pub relationships: Vec<String>,
    pub notes: Option<String>,
    pub line_index: usize,
    pub raw_markdown: String,
    pub parsed_entity: ParsedEntityContract,
    pub schema_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactSummary {
    pub total: usize,
    pub relationship_counts: BTreeMap<String, usize>,
    pub top_tags: Vec<TagCount>,
    pub contacts_with_email: usize,
    pub contacts_with_phone: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactDocument {
    pub document_id: String,
    pub markdown_relative_path: String,
    pub markdown_body: String,
    pub contacts: Vec<ContactEntry>,
    pub summary: ContactSummary,
    pub warnings: Vec<String>,
    pub document_metadata: DocumentMetadata,
    pub layout_metadata: Option<LayoutMetadata>,
}

struct ContactRecordRange {
    start_line: usize,
    end_line: usize,
}

pub struct ContactsService;

impl ContactsService {
    pub fn read_contacts(vault_path: &Path) -> Result<ContactDocument, String> {
        read_graph_contacts_document(vault_path)
    }

    pub fn create_contact(
        vault_path: &Path,
        input: ContactInput,
    ) -> Result<ContactDocument, String> {
        create_v2_contact_entity(vault_path, input)?;
        read_graph_contacts_document(vault_path)
    }

    pub fn update_contact(
        vault_path: &Path,
        contact_id: &str,
        input: ContactInput,
    ) -> Result<ContactDocument, String> {
        if contact_id.starts_with("bl_doc_") {
            update_v2_contact_entity(vault_path, contact_id, input)?;
            return read_graph_contacts_document(vault_path);
        }

        let metadata = ensure_contacts_document(vault_path)?;
        let document = read_contacts_document(vault_path, metadata)?;
        let range = contact_range_for_id(&document.markdown_body, contact_id)
            .ok_or_else(|| "Contact was not found or was changed outside BentoLife.".to_string())?;
        let replacement = render_contact_record(&clean_contact_input(input)?);
        let markdown_body = replace_line_range(&document.markdown_body, range, &replacement)?;

        persist_contacts_body(vault_path, document.document_metadata, &markdown_body)
    }
}

fn read_graph_contacts_document(vault_path: &Path) -> Result<ContactDocument, String> {
    let metadata = ensure_contacts_index_document(vault_path)?;
    let mut warnings = Vec::new();
    let mut contacts = read_contacts_from_folder(vault_path, CONTACT_DATA_FOLDER, &mut warnings)?;
    contacts.extend(read_contacts_from_folder(
        vault_path,
        CONTACT_LEGACY_ENTITY_FOLDER,
        &mut warnings,
    )?);
    let legacy_path = resolve_vault_relative_path(vault_path, CONTACTS_RELATIVE_PATH)?;
    let mut legacy_body = String::new();
    if legacy_path.is_file() {
        let legacy = read_contacts_document(vault_path, ensure_contacts_document(vault_path)?)?;
        legacy_body = legacy.markdown_body;
        contacts.extend(legacy.contacts);
        warnings.extend(legacy.warnings);
    }
    contacts.sort_by_key(|contact| contact.name.to_lowercase());
    let summary = summarize_contacts(&contacts);
    let markdown_body = render_contacts_index_body(&contacts, &legacy_body);
    Ok(ContactDocument {
        document_id: metadata.document_id.clone(),
        markdown_relative_path: metadata.current_path.clone(),
        markdown_body,
        contacts,
        summary,
        warnings,
        document_metadata: metadata,
        layout_metadata: None,
    })
}

fn ensure_contacts_index_document(vault_path: &Path) -> Result<DocumentMetadata, String> {
    LayoutFolderService::create_or_repair(vault_path)?;
    WorkspaceMetadataService::write_bootstrap_files(vault_path)?;
    let index_path = resolve_vault_relative_path(vault_path, CONTACT_INDEX_PATH)?;
    let body = "# Contacts\n\nThis module summarizes per-contact Markdown entities.\n";
    let document_id = generate_document_id(CONTACT_INDEX_PATH);
    let metadata = DocumentMetadataService::read(vault_path, &document_id).unwrap_or_else(|_| {
        DocumentMetadataService::create_default_with_type(
            &document_id,
            CONTACT_INDEX_PATH,
            body,
            "contact",
        )
        .expect("default Contacts index metadata is valid")
    });
    if !index_path.is_file() {
        let managed = MarkdownDocumentService::prepare_managed_markdown(
            body,
            &document_id,
            &metadata.frontmatter_contract.required_value,
        );
        write_text_atomic(&index_path, &managed)?;
        ensure_layout_metadata(vault_path, &document_id, &managed)?;
    }
    DocumentMetadataService::write(vault_path, &metadata)?;
    rebuild_and_register(vault_path, &metadata)?;
    Ok(metadata)
}

fn create_v2_contact_entity(vault_path: &Path, input: ContactInput) -> Result<(), String> {
    let cleaned = clean_contact_input(input)?;
    let relative_path = unique_contact_entity_path(vault_path, &cleaned.name)?;
    let document_id = generate_document_id(&relative_path);
    persist_v2_contact_entity(vault_path, &document_id, &relative_path, &cleaned)
}

fn update_v2_contact_entity(
    vault_path: &Path,
    contact_id: &str,
    input: ContactInput,
) -> Result<(), String> {
    let document_id = document_id_from_record_id(contact_id);
    let metadata = DocumentMetadataService::read(vault_path, document_id)?;
    if !is_contact_entity_path(&metadata.current_path) {
        return Err("Contact entity was not found.".to_string());
    }
    let cleaned = clean_contact_input(input)?;
    let existing = read_v2_contact(vault_path, &metadata.current_path, document_id)?;
    if existing.contact_id != contact_id {
        return Err("Contact was changed outside BentoLife.".to_string());
    }
    persist_v2_contact_entity(vault_path, document_id, &metadata.current_path, &cleaned)
}

fn persist_v2_contact_entity(
    vault_path: &Path,
    document_id: &str,
    relative_path: &str,
    input: &ContactInput,
) -> Result<(), String> {
    let body = render_contact_entity(input);
    let mut metadata = match DocumentMetadataService::read(vault_path, document_id) {
        Ok(metadata) => metadata,
        Err(_) => DocumentMetadataService::create_default_with_type(
            document_id,
            relative_path,
            &body,
            "contact",
        )?,
    };
    metadata.document_type = "contact".to_string();
    metadata.current_path = relative_path.to_string();
    let managed = MarkdownDocumentService::prepare_managed_markdown(
        &body,
        document_id,
        &metadata.frontmatter_contract.required_value,
    );
    metadata.content_hash = content_hash(&managed);
    metadata.updated_at = current_timestamp_label();
    let markdown_path = resolve_vault_relative_path(vault_path, relative_path)?;
    write_text_atomic(&markdown_path, &managed)?;
    DocumentMetadataService::write(vault_path, &metadata)?;
    ensure_layout_metadata(vault_path, document_id, &managed)?;
    rebuild_and_register(vault_path, &metadata)
}

fn read_contacts_from_folder(
    vault_path: &Path,
    relative_folder: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<ContactEntry>, String> {
    let folder = resolve_vault_relative_path(vault_path, relative_folder)?;
    let mut contacts = Vec::new();
    if !folder.is_dir() {
        return Ok(contacts);
    }
    for entry in std::fs::read_dir(&folder)
        .map_err(|error| format!("Unable to read {}: {error}", folder.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read Contact entity: {error}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name == "INDEX.md" || file_name == "MODULE.md" {
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        let markdown = std::fs::read_to_string(&path)
            .map_err(|error| format!("Unable to read {}: {error}", path.display()))?;
        let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
        let document_id = DocumentIdentityService::find_identity_comment(&markdown)
            .map(|identity| identity.document_id)
            .unwrap_or_else(|| generate_document_id(&path.to_string_lossy()));
        let body = DocumentIdentityService::remove_identity_comments(&parsed.body);
        let relative_path = path
            .strip_prefix(vault_path)
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
        if let Some(contact) =
            parse_contact_entity(vault_path, &body, &document_id, &relative_path, warnings)
        {
            contacts.push(contact);
        }
    }
    Ok(contacts)
}

fn read_v2_contact(
    vault_path: &Path,
    relative_path: &str,
    document_id: &str,
) -> Result<ContactEntry, String> {
    let markdown_path = resolve_vault_relative_path(vault_path, relative_path)?;
    let markdown = std::fs::read_to_string(&markdown_path)
        .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
    let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
    let body = DocumentIdentityService::remove_identity_comments(&parsed.body);
    let mut warnings = Vec::new();
    parse_contact_entity(vault_path, &body, document_id, relative_path, &mut warnings)
        .ok_or_else(|| "Contact entity could not be parsed.".to_string())
}

fn ensure_contacts_document(vault_path: &Path) -> Result<DocumentMetadata, String> {
    LayoutFolderService::create_or_repair(vault_path)?;
    WorkspaceMetadataService::write_bootstrap_files(vault_path)?;

    let markdown_path = resolve_vault_relative_path(vault_path, CONTACTS_RELATIVE_PATH)?;
    let existing_markdown = if markdown_path.exists() {
        std::fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?
    } else {
        DEFAULT_CONTACTS_MARKDOWN.to_string()
    };

    let document_id = DocumentIdentityService::find_identity_comment(&existing_markdown)
        .map(|identity| identity.document_id)
        .unwrap_or_else(|| generate_document_id(CONTACTS_RELATIVE_PATH));
    let mut metadata =
        DocumentMetadataService::read(vault_path, &document_id).unwrap_or_else(|_| {
            DocumentMetadataService::create_default_with_type(
                &document_id,
                CONTACTS_RELATIVE_PATH,
                &existing_markdown,
                "contact",
            )
            .expect("default Contacts metadata is valid")
        });

    if metadata.current_path != CONTACTS_RELATIVE_PATH {
        if !metadata.previous_paths.contains(&metadata.current_path) {
            metadata.previous_paths.push(metadata.current_path.clone());
        }
        metadata.current_path = CONTACTS_RELATIVE_PATH.to_string();
    }
    metadata.document_type = "contact".to_string();

    let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
        &existing_markdown,
        &document_id,
        &metadata.frontmatter_contract.required_value,
    );
    metadata.content_hash = content_hash(&managed_markdown);
    metadata.updated_at = current_timestamp_label();

    write_text_atomic(&markdown_path, &managed_markdown)?;
    DocumentMetadataService::write(vault_path, &metadata)?;
    ensure_layout_metadata(vault_path, &document_id, &managed_markdown)?;
    rebuild_and_register(vault_path, &metadata)?;

    Ok(metadata)
}

fn read_contacts_document(
    vault_path: &Path,
    metadata: DocumentMetadata,
) -> Result<ContactDocument, String> {
    let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
    let markdown = std::fs::read_to_string(&markdown_path)
        .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
    let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
    let markdown_body = normalize_body_for_response(
        &DocumentIdentityService::remove_identity_comments(&parsed.body),
    );
    let mut warnings = Vec::new();
    let contacts = parse_contacts(
        vault_path,
        &markdown_body,
        &metadata.current_path,
        &mut warnings,
    );
    let summary = summarize_contacts(&contacts);
    let layout_metadata = LayoutMetadataService::read(vault_path, &metadata.document_id).ok();

    Ok(ContactDocument {
        document_id: metadata.document_id.clone(),
        markdown_relative_path: metadata.current_path.clone(),
        markdown_body,
        contacts,
        summary,
        warnings,
        document_metadata: metadata,
        layout_metadata,
    })
}

fn persist_contacts_body(
    vault_path: &Path,
    mut metadata: DocumentMetadata,
    markdown_body: &str,
) -> Result<ContactDocument, String> {
    let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
    let body = normalize_body_for_write(markdown_body);
    let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
        &body,
        &metadata.document_id,
        &metadata.frontmatter_contract.required_value,
    );

    metadata.document_type = "contact".to_string();
    metadata.content_hash = content_hash(&managed_markdown);
    metadata.updated_at = current_timestamp_label();

    write_text_atomic(&markdown_path, &managed_markdown)?;
    DocumentMetadataService::write(vault_path, &metadata)?;
    ensure_layout_metadata(vault_path, &metadata.document_id, &managed_markdown)?;
    rebuild_and_register(vault_path, &metadata)?;
    read_contacts_document(vault_path, metadata)
}

fn ensure_layout_metadata(
    vault_path: &Path,
    document_id: &str,
    markdown: &str,
) -> Result<(), String> {
    if LayoutMetadataService::read(vault_path, document_id).is_ok() {
        return Ok(());
    }

    let layout = LayoutMetadataService::generate_from_markdown(document_id, markdown)?;
    LayoutMetadataService::write(vault_path, &layout)
}

fn rebuild_and_register(vault_path: &Path, metadata: &DocumentMetadata) -> Result<(), String> {
    let documents = DocumentMetadataService::list(vault_path)?;
    let index = WorkspaceMetadataService::rebuild_index_from_documents(&documents)?;
    WorkspaceMetadataService::write_index(vault_path, &index)?;
    WorkspaceMetadataService::register_document(vault_path, metadata)
}

fn parse_contacts(
    vault_path: &Path,
    markdown_body: &str,
    source_path: &str,
    warnings: &mut Vec<String>,
) -> Vec<ContactEntry> {
    contact_ranges(markdown_body)
        .into_iter()
        .filter_map(|range| {
            parse_contact_record(vault_path, markdown_body, &range, source_path, warnings)
        })
        .collect()
}

fn parse_contact_record(
    vault_path: &Path,
    markdown_body: &str,
    range: &ContactRecordRange,
    source_path: &str,
    warnings: &mut Vec<String>,
) -> Option<ContactEntry> {
    let lines = markdown_body.lines().collect::<Vec<_>>();
    let heading = lines.get(range.start_line)?.trim();
    let name = heading.strip_prefix("## ")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let raw_markdown = lines[range.start_line..range.end_line].join("\n");
    let mut relationship = None;
    let mut organization = None;
    let mut email = None;
    let mut phone = None;
    let mut tags = Vec::new();
    let mut relationships = Vec::new();
    let mut notes = None;

    for line in &lines[range.start_line + 1..range.end_line] {
        let Some((key, value)) = parse_field_line(line) else {
            continue;
        };
        match key.as_str() {
            "relationship" => relationship = clean_optional_string(&value),
            "organization" => organization = clean_optional_string(&value),
            "email" => email = clean_optional_string(&value),
            "phone" => phone = clean_optional_string(&value),
            "tags" => tags = clean_tags(value),
            "relationships" | "related" => relationships = clean_relationships(&value),
            "notes" => notes = clean_optional_markdown(&value),
            "attachments" => {}
            _ => warnings.push(format!(
                "{source_path}: unknown contact field '{key}' remains visible in Markdown."
            )),
        }
    }
    if notes.is_none() {
        notes = section_body(&lines[range.start_line + 1..range.end_line], "### Notes")
            .and_then(|value| clean_optional_markdown(&value));
    }
    let raw_markdown = format!("{raw_markdown}\n");
    let payload = ParsedContactPayload {
        name: name.clone(),
        relationship: relationship.clone(),
        organization: organization.clone(),
        email: email.clone(),
        phone: phone.clone(),
        tags: tags.clone(),
        relationships: relationships.clone(),
        notes: notes.clone(),
    };
    let (parsed_entity, mut schema_warnings) =
        parsed_contact_payload(vault_path, &raw_markdown, source_path, payload);
    schema_warnings.extend(entry_warnings(warnings, source_path));

    Some(ContactEntry {
        contact_id: contact_id_for_record(range.start_line, raw_markdown.trim_end()),
        name,
        relationship,
        organization,
        email,
        phone,
        tags,
        relationships,
        notes,
        line_index: range.start_line,
        raw_markdown,
        parsed_entity,
        schema_warnings,
    })
}

fn contact_ranges(markdown_body: &str) -> Vec<ContactRecordRange> {
    let lines = markdown_body.lines().collect::<Vec<_>>();
    let mut starts = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.trim().starts_with("## ").then_some(index))
        .collect::<Vec<_>>();
    starts.push(lines.len());

    starts
        .windows(2)
        .map(|window| ContactRecordRange {
            start_line: window[0],
            end_line: window[1],
        })
        .collect()
}

fn contact_range_for_id(markdown_body: &str, contact_id: &str) -> Option<ContactRecordRange> {
    contact_ranges(markdown_body).into_iter().find(|range| {
        let raw_markdown = markdown_body
            .lines()
            .collect::<Vec<_>>()
            .get(range.start_line..range.end_line)
            .unwrap_or_default()
            .join("\n");
        contact_id_for_record(range.start_line, &raw_markdown) == contact_id
    })
}

fn summarize_contacts(contacts: &[ContactEntry]) -> ContactSummary {
    let mut relationship_counts = BTreeMap::new();
    let mut tag_counts = BTreeMap::new();
    for contact in contacts {
        if let Some(relationship) = &contact.relationship {
            *relationship_counts.entry(relationship.clone()).or_insert(0) += 1;
        }
        for tag in &contact.tags {
            *tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }
    }

    let mut top_tags = tag_counts
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect::<Vec<_>>();
    top_tags.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.tag.cmp(&right.tag))
    });
    top_tags.truncate(5);

    ContactSummary {
        total: contacts.len(),
        relationship_counts,
        top_tags,
        contacts_with_email: contacts
            .iter()
            .filter(|contact| contact.email.is_some())
            .count(),
        contacts_with_phone: contacts
            .iter()
            .filter(|contact| contact.phone.is_some())
            .count(),
    }
}

fn replace_line_range(
    markdown_body: &str,
    range: ContactRecordRange,
    replacement: &str,
) -> Result<String, String> {
    let mut lines = markdown_body
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if range.start_line >= range.end_line || range.end_line > lines.len() {
        return Err("Contact source block was not found.".to_string());
    }
    lines.splice(
        range.start_line..range.end_line,
        replacement.trim_end().lines().map(str::to_string),
    );
    Ok(format!("{}\n", lines.join("\n").trim_end()))
}

fn render_contact_record(input: &ContactInput) -> String {
    let mut lines = vec![format!("## {}", input.name.trim())];
    push_optional_field(&mut lines, "Relationship", input.relationship.as_deref());
    push_optional_field(&mut lines, "Organization", input.organization.as_deref());
    push_optional_field(&mut lines, "Email", input.email.as_deref());
    push_optional_field(&mut lines, "Phone", input.phone.as_deref());
    if !input.tags.is_empty() {
        lines.push(format!("- Tags: {}", input.tags.join(", ")));
    }
    if !input.relationships.is_empty() {
        lines.push(format!(
            "- Relationships: {}",
            input.relationships.join(", ")
        ));
    }
    push_notes_section(&mut lines, "### Notes", input.notes.as_deref());
    format!("{}\n", lines.join("\n"))
}

fn render_contact_entity(input: &ContactInput) -> String {
    let mut lines = vec![format!("# {}", input.name.trim())];
    push_optional_field(&mut lines, "Relationship", input.relationship.as_deref());
    push_optional_field(&mut lines, "Organization", input.organization.as_deref());
    push_optional_field(&mut lines, "Email", input.email.as_deref());
    push_optional_field(&mut lines, "Phone", input.phone.as_deref());
    if !input.tags.is_empty() {
        lines.push(format!("- Tags: {}", input.tags.join(", ")));
    }
    if !input.relationships.is_empty() {
        lines.push(format!(
            "- Relationships: {}",
            input.relationships.join(", ")
        ));
    }
    push_notes_section(&mut lines, "## Notes", input.notes.as_deref());
    format!("{}\n", lines.join("\n"))
}

fn parse_contact_entity(
    vault_path: &Path,
    markdown_body: &str,
    document_id: &str,
    source_path: &str,
    warnings: &mut Vec<String>,
) -> Option<ContactEntry> {
    let lines = markdown_body.lines().collect::<Vec<_>>();
    let heading = lines
        .iter()
        .find(|line| line.trim().starts_with("# "))?
        .trim();
    let name = heading.strip_prefix("# ")?.trim().to_string();
    let mut relationship = None;
    let mut organization = None;
    let mut email = None;
    let mut phone = None;
    let mut tags = Vec::new();
    let mut relationships = Vec::new();
    let mut notes = None;
    for line in lines.iter().skip(1) {
        let Some((key, value)) = parse_field_line(line) else {
            continue;
        };
        match key.as_str() {
            "relationship" => relationship = clean_optional_string(&value),
            "organization" => organization = clean_optional_string(&value),
            "email" => email = clean_optional_string(&value),
            "phone" => phone = clean_optional_string(&value),
            "tags" => tags = clean_tags(value),
            "relationships" | "related" => relationships = clean_relationships(&value),
            "notes" => notes = clean_optional_markdown(&value),
            "attachments" => {}
            _ => warnings.push(format!(
                "{source_path}: unknown contact field '{key}' remains visible in Markdown."
            )),
        }
    }
    if notes.is_none() {
        notes =
            section_body(&lines[1..], "## Notes").and_then(|value| clean_optional_markdown(&value));
    }
    let raw_markdown = format!("{}\n", markdown_body.trim());
    let payload = ParsedContactPayload {
        name: name.clone(),
        relationship: relationship.clone(),
        organization: organization.clone(),
        email: email.clone(),
        phone: phone.clone(),
        tags: tags.clone(),
        relationships: relationships.clone(),
        notes: notes.clone(),
    };
    let (parsed_entity, mut schema_warnings) =
        parsed_contact_payload(vault_path, &raw_markdown, source_path, payload);
    schema_warnings.extend(entry_warnings(warnings, source_path));
    Some(ContactEntry {
        contact_id: record_id(document_id, markdown_body),
        name,
        relationship,
        organization,
        email,
        phone,
        tags,
        relationships,
        notes,
        line_index: 0,
        raw_markdown,
        parsed_entity,
        schema_warnings,
    })
}

struct ParsedContactPayload {
    name: String,
    relationship: Option<String>,
    organization: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    tags: Vec<String>,
    relationships: Vec<String>,
    notes: Option<String>,
}

fn parsed_contact_payload(
    vault_path: &Path,
    raw_markdown: &str,
    source_path: &str,
    payload: ParsedContactPayload,
) -> (ParsedEntityContract, Vec<String>) {
    let mut entity = MarkdownParser::parse(raw_markdown);
    entity.module_id = Some("contacts".to_string());
    entity.entity_type = Some("contact".to_string());
    entity.path = source_path.to_string();
    entity.content_hash = content_hash(raw_markdown);
    entity
        .fields
        .insert("name".to_string(), payload.name.clone());
    entity.fields.insert("title".to_string(), payload.name);
    if let Some(value) = payload.relationship {
        entity.fields.insert("relationship".to_string(), value);
    }
    if let Some(value) = payload.organization {
        entity.fields.insert("organization".to_string(), value);
    }
    if let Some(value) = payload.email {
        entity.fields.insert("email".to_string(), value);
    }
    if let Some(value) = payload.phone {
        entity.fields.insert("phone".to_string(), value);
    }
    if let Some(value) = payload.notes {
        entity.fields.insert("notes".to_string(), value);
    }
    if entity.tags.is_empty() {
        entity.tags = payload.tags;
    }
    if !payload.relationships.is_empty() {
        entity.fields.insert(
            "relationships".to_string(),
            payload.relationships.join(", "),
        );
        entity.relationships = payload.relationships;
    }
    if entity.relationships.is_empty() {
        entity.relationships = relationship_values(&entity.fields);
    }
    let schema_warnings = apply_schema_descriptors(
        vault_path,
        "modules/contacts/module.schema.json",
        &mut entity,
    )
    .unwrap_or_else(|error| vec![format!("Contact schema could not be applied: {error}")]);
    (entity, schema_warnings)
}

fn relationship_values(fields: &std::collections::HashMap<String, String>) -> Vec<String> {
    fields
        .get("relationships")
        .or_else(|| fields.get("related"))
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn entry_warnings(warnings: &[String], source_path: &str) -> Vec<String> {
    warnings
        .iter()
        .filter(|warning| warning.contains(source_path))
        .cloned()
        .collect()
}

fn document_id_from_record_id(record_id: &str) -> &str {
    record_id
        .split_once("::")
        .map(|(document_id, _)| document_id)
        .unwrap_or(record_id)
}

fn record_id(document_id: &str, raw_markdown: &str) -> String {
    format!("{document_id}::{}", content_hash(raw_markdown))
}

fn render_contacts_index_body(contacts: &[ContactEntry], legacy_body: &str) -> String {
    let mut body = "# Contacts\n".to_string();
    for contact in contacts {
        body.push('\n');
        body.push_str(&render_contact_record(&ContactInput {
            name: contact.name.clone(),
            relationship: contact.relationship.clone(),
            organization: contact.organization.clone(),
            email: contact.email.clone(),
            phone: contact.phone.clone(),
            tags: contact.tags.clone(),
            relationships: contact.relationships.clone(),
            notes: contact.notes.clone(),
        }));
    }
    if !legacy_body.trim().is_empty() {
        body.push_str("\n## Legacy compatibility\n- Unmigrated records from modules/contacts.md are included until explicit upgrade runs.\n");
    }
    body
}

fn push_optional_field(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value.and_then(clean_optional_string) {
        lines.push(format!("- {label}: {value}"));
    }
}

fn push_notes_section(lines: &mut Vec<String>, heading: &str, value: Option<&str>) {
    if let Some(value) = value.and_then(clean_optional_markdown) {
        lines.push(heading.to_string());
        lines.push(value);
    }
}

fn parse_field_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let without_bullet = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))?;
    let (key, value) = without_bullet.split_once(':')?;
    Some((key.trim().to_lowercase(), value.trim().to_string()))
}

fn clean_contact_input(mut input: ContactInput) -> Result<ContactInput, String> {
    input.name = collapse_inline_text(&input.name);
    if input.name.is_empty() {
        return Err("Contact name is required.".to_string());
    }
    input.relationship = input
        .relationship
        .and_then(|value| clean_optional_string(&value));
    input.organization = input
        .organization
        .and_then(|value| clean_optional_string(&value));
    input.email = input.email.and_then(|value| clean_optional_string(&value));
    input.phone = input.phone.and_then(|value| clean_optional_string(&value));
    input.notes = input
        .notes
        .and_then(|value| clean_optional_markdown(&value));
    input.tags = input
        .tags
        .into_iter()
        .flat_map(clean_tags)
        .collect::<Vec<_>>();
    input.tags.sort();
    input.tags.dedup();
    input.relationships = input
        .relationships
        .into_iter()
        .flat_map(|value| clean_relationships(&value))
        .collect::<Vec<_>>();
    input.relationships.sort();
    input.relationships.dedup();
    Ok(input)
}

fn clean_optional_string(value: &str) -> Option<String> {
    let cleaned = collapse_inline_text(value);
    (!cleaned.is_empty()).then_some(cleaned)
}

fn clean_optional_markdown(value: &str) -> Option<String> {
    let cleaned = value
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    (!cleaned.is_empty()).then_some(cleaned)
}

fn section_body(lines: &[&str], heading: &str) -> Option<String> {
    let start = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case(heading))?;
    let heading_level = heading
        .chars()
        .take_while(|character| *character == '#')
        .count();
    let mut body = Vec::new();
    for line in lines.iter().skip(start + 1) {
        let trimmed = line.trim_start();
        let level = trimmed
            .chars()
            .take_while(|character| *character == '#')
            .count();
        if level > 0 && level <= heading_level && trimmed.chars().nth(level) == Some(' ') {
            break;
        }
        body.push(*line);
    }
    Some(body.join("\n"))
}

fn clean_tags(value: String) -> Vec<String> {
    value
        .split(',')
        .map(collapse_inline_text)
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn clean_relationships(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(collapse_inline_text)
        .filter(|relationship| !relationship.is_empty())
        .collect()
}

fn collapse_inline_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn unique_contact_entity_path(vault_path: &Path, name: &str) -> Result<String, String> {
    let base_slug = slugify(name);
    for index in 0..100 {
        let candidate = if index == 0 {
            format!("{CONTACT_DATA_FOLDER}/{base_slug}.md")
        } else {
            format!("{CONTACT_DATA_FOLDER}/{base_slug}-{index}.md")
        };
        if !resolve_vault_relative_path(vault_path, &candidate)?.exists() {
            return Ok(candidate);
        }
    }
    Err("Unable to find an available Contact filename.".to_string())
}

fn is_contact_entity_path(path: &str) -> bool {
    (path.starts_with(CONTACT_DATA_FOLDER) || path.starts_with(CONTACT_LEGACY_ENTITY_FOLDER))
        && path.ends_with(".md")
        && path != CONTACT_INDEX_PATH
        && !path.ends_with("/MODULE.md")
}

fn slugify(value: &str) -> String {
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
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if slug.is_empty() {
        "untitled-contact".to_string()
    } else {
        slug
    }
}

fn normalize_body_for_response(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        DEFAULT_CONTACTS_MARKDOWN.to_string()
    } else {
        format!("{body}\n")
    }
}

fn normalize_body_for_write(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        DEFAULT_CONTACTS_MARKDOWN.to_string()
    } else {
        format!("{body}\n")
    }
}

fn contact_id_for_record(line_index: usize, raw_markdown: &str) -> String {
    format!("contact_{line_index}_{}", content_hash(raw_markdown))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-contacts-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    #[test]
    fn scaffolded_contacts_index_is_not_exposed_as_a_contact_record() {
        let vault_path = unique_temp_vault("scaffold-index");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");

        let document = ContactsService::read_contacts(&vault_path).expect("contacts read");

        assert_eq!(document.markdown_relative_path, CONTACT_INDEX_PATH);
        assert!(document.contacts.is_empty());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn creates_reads_and_summarizes_contacts() {
        let vault_path = unique_temp_vault("create");
        let document = ContactsService::create_contact(
            &vault_path,
            ContactInput {
                name: "Mina Park".to_string(),
                relationship: Some("Friend".to_string()),
                organization: Some("Studio".to_string()),
                email: Some("mina@example.com".to_string()),
                phone: None,
                tags: vec!["design".to_string(), "friend".to_string()],
                relationships: vec!["[[Launch Review]]".to_string()],
                notes: Some("Met at studio".to_string()),
            },
        )
        .expect("contact created");

        assert_eq!(document.markdown_relative_path, CONTACT_INDEX_PATH);
        assert_eq!(document.contacts[0].name, "Mina Park");
        assert_eq!(document.summary.total, 1);
        assert_eq!(document.summary.contacts_with_email, 1);
        assert_eq!(document.summary.relationship_counts.get("Friend"), Some(&1));
        assert!(vault_path
            .join("modules/contacts/data/mina-park.md")
            .is_file());
        let metadata =
            DocumentMetadataService::read(&vault_path, &document.document_id).expect("metadata");
        assert_eq!(metadata.document_type, "contact");

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn updates_contact_by_current_record_identity() {
        let vault_path = unique_temp_vault("update");
        let document = ContactsService::create_contact(
            &vault_path,
            ContactInput {
                name: "Mina".to_string(),
                relationship: Some("Friend".to_string()),
                organization: None,
                email: None,
                phone: None,
                tags: vec![],
                relationships: vec![],
                notes: None,
            },
        )
        .expect("contact created");
        let contact_id = document.contacts[0].contact_id.clone();
        let updated = ContactsService::update_contact(
            &vault_path,
            &contact_id,
            ContactInput {
                name: "Mina Park".to_string(),
                relationship: Some("Collaborator".to_string()),
                organization: Some("BentoLab".to_string()),
                email: Some("mina@example.com".to_string()),
                phone: None,
                tags: vec!["design".to_string()],
                relationships: vec!["[[Design System]]".to_string()],
                notes: None,
            },
        )
        .expect("contact updated");

        assert_eq!(updated.contacts[0].name, "Mina Park");
        assert_eq!(
            updated.contacts[0].relationship.as_deref(),
            Some("Collaborator")
        );

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rejects_stale_contact_ids_after_external_edits() {
        let vault_path = unique_temp_vault("stale");
        let document = ContactsService::create_contact(
            &vault_path,
            ContactInput {
                name: "Mina".to_string(),
                relationship: None,
                organization: None,
                email: None,
                phone: None,
                tags: vec![],
                relationships: vec![],
                notes: None,
            },
        )
        .expect("contact created");
        let contact_id = document.contacts[0].contact_id.clone();
        let contacts_path = vault_path.join("modules/contacts/data/mina.md");
        let markdown = std::fs::read_to_string(&contacts_path).expect("contacts markdown");
        std::fs::write(contacts_path, markdown.replace("# Mina", "# Mina Changed"))
            .expect("external edit");

        let result = ContactsService::update_contact(
            &vault_path,
            &contact_id,
            ContactInput {
                name: "Mina".to_string(),
                relationship: None,
                organization: None,
                email: None,
                phone: None,
                tags: vec![],
                relationships: vec![],
                notes: None,
            },
        );

        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn reads_v3_long_form_contact_notes_and_reports_unknown_fields() {
        let vault_path = unique_temp_vault("long-form");
        let document = ContactsService::create_contact(
            &vault_path,
            ContactInput {
                name: "Mina".to_string(),
                relationship: None,
                organization: None,
                email: None,
                phone: None,
                tags: vec![],
                relationships: vec![],
                notes: Some("Met at studio.\n\nFollow up about launch.".to_string()),
            },
        )
        .expect("contact created");
        let contact_id = document.contacts[0].contact_id.clone();
        let contacts_path = vault_path.join("modules/contacts/data/mina.md");
        let markdown = std::fs::read_to_string(&contacts_path).expect("contacts markdown");
        std::fs::write(
            &contacts_path,
            markdown.replace(
                "## Notes",
                "- Favorite color: Blue\n- Relationships: [[Mina Mentor]]\n\n## Notes",
            ),
        )
        .expect("external edit");

        let document = ContactsService::read_contacts(&vault_path).expect("contacts read");
        let contact = document
            .contacts
            .iter()
            .find(|contact| {
                contact
                    .contact_id
                    .starts_with(document_id_from_record_id(&contact_id))
            })
            .unwrap_or(&document.contacts[0]);

        assert!(contact
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("Follow up"));
        assert!(contact.raw_markdown.contains("Favorite color"));
        assert_eq!(contact.parsed_entity.path, "modules/contacts/data/mina.md");
        assert_eq!(
            contact.parsed_entity.fields.get("name"),
            Some(&"Mina".to_string())
        );
        assert!(contact
            .parsed_entity
            .relationships
            .contains(&"[[Mina Mentor]]".to_string()));
        assert!(!contact.parsed_entity.content_hash.is_empty());
        assert!(contact
            .schema_warnings
            .iter()
            .any(|warning| warning.contains("favorite color")));
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.contains("unknown contact field 'favorite color'")));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn preserves_existing_plain_contacts_markdown_when_managing() {
        let vault_path = unique_temp_vault("plain");
        let contacts_path = vault_path.join(CONTACTS_RELATIVE_PATH);
        std::fs::create_dir_all(contacts_path.parent().expect("contacts parent")).expect("parent");
        std::fs::write(
            &contacts_path,
            "# Contacts\n\n## Existing Person\n- Relationship: Family\n",
        )
        .expect("fixture");

        let document = ContactsService::read_contacts(&vault_path).expect("contacts read");

        assert_eq!(document.contacts[0].name, "Existing Person");
        assert!(std::fs::read_to_string(contacts_path)
            .expect("managed markdown")
            .contains("bentolife_metadata"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
