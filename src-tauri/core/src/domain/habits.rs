use std::{collections::BTreeSet, path::Path};

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

const HABITS_RELATIVE_PATH: &str = "modules/habits.md";
const HABIT_LEGACY_ENTITY_FOLDER: &str = "modules/habits";
const HABIT_DATA_FOLDER: &str = "modules/habits/data";
const HABIT_INDEX_PATH: &str = "modules/habits/INDEX.md";
const DEFAULT_HABITS_MARKDOWN: &str = "# Habits\n";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HabitInput {
    pub name: String,
    pub frequency: Option<String>,
    pub target: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub relationships: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HabitEntry {
    pub habit_id: String,
    pub name: String,
    pub frequency: Option<String>,
    pub target: Option<String>,
    pub tags: Vec<String>,
    pub relationships: Vec<String>,
    pub notes: Option<String>,
    pub checkins: Vec<String>,
    pub line_index: usize,
    pub raw_markdown: String,
    pub parsed_entity: ParsedEntityContract,
    pub schema_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HabitRecentCheckin {
    pub habit_id: String,
    pub name: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HabitStreak {
    pub habit_id: String,
    pub name: String,
    pub current_streak: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HabitSummary {
    pub total: usize,
    pub summary_date: Option<String>,
    pub checked_in_on_date: usize,
    pub recent_checkins: Vec<HabitRecentCheckin>,
    pub streaks: Vec<HabitStreak>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HabitDocument {
    pub document_id: String,
    pub markdown_relative_path: String,
    pub markdown_body: String,
    pub habits: Vec<HabitEntry>,
    pub summary: HabitSummary,
    pub warnings: Vec<String>,
    pub document_metadata: DocumentMetadata,
    pub layout_metadata: Option<LayoutMetadata>,
}

struct HabitRecordRange {
    start_line: usize,
    end_line: usize,
}

pub struct HabitsService;

impl HabitsService {
    pub fn read_habits(
        vault_path: &Path,
        summary_date: Option<String>,
    ) -> Result<HabitDocument, String> {
        if let Some(date) = &summary_date {
            validate_local_date(date)?;
        }
        read_graph_habits_document(vault_path, summary_date.as_deref())
    }

    pub fn create_habit(
        vault_path: &Path,
        input: HabitInput,
        summary_date: Option<String>,
    ) -> Result<HabitDocument, String> {
        if let Some(date) = &summary_date {
            validate_local_date(date)?;
        }
        create_v2_habit_entity(vault_path, input)?;
        read_graph_habits_document(vault_path, summary_date.as_deref())
    }

    pub fn update_habit(
        vault_path: &Path,
        habit_id: &str,
        input: HabitInput,
        summary_date: Option<String>,
    ) -> Result<HabitDocument, String> {
        if let Some(date) = &summary_date {
            validate_local_date(date)?;
        }
        if habit_id.starts_with("bl_doc_") {
            update_v2_habit_entity(vault_path, habit_id, input)?;
            return read_graph_habits_document(vault_path, summary_date.as_deref());
        }

        let metadata = ensure_habits_document(vault_path)?;
        let document = read_habits_document(vault_path, metadata, summary_date.as_deref())?;
        let range = habit_range_for_id(&document.markdown_body, habit_id)
            .ok_or_else(|| "Habit was not found or was changed outside BentoLife.".to_string())?;
        let existing = document
            .habits
            .iter()
            .find(|habit| habit.habit_id == habit_id)
            .ok_or_else(|| "Habit was not found or was changed outside BentoLife.".to_string())?;
        let cleaned = clean_habit_input(input)?;
        let replacement = render_habit_record(&cleaned, &existing.checkins);
        let markdown_body = replace_line_range(&document.markdown_body, range, &replacement)?;

        persist_habits_body(
            vault_path,
            document.document_metadata,
            &markdown_body,
            summary_date.as_deref(),
        )
    }

    pub fn record_checkin(
        vault_path: &Path,
        habit_id: &str,
        date: &str,
    ) -> Result<HabitDocument, String> {
        validate_local_date(date)?;
        if habit_id.starts_with("bl_doc_") {
            record_v2_habit_checkin(vault_path, habit_id, date)?;
            return read_graph_habits_document(vault_path, Some(date));
        }

        let metadata = ensure_habits_document(vault_path)?;
        let document = read_habits_document(vault_path, metadata, Some(date))?;
        let range = habit_range_for_id(&document.markdown_body, habit_id)
            .ok_or_else(|| "Habit was not found or was changed outside BentoLife.".to_string())?;
        let existing = document
            .habits
            .iter()
            .find(|habit| habit.habit_id == habit_id)
            .ok_or_else(|| "Habit was not found or was changed outside BentoLife.".to_string())?;
        let mut checkins = existing.checkins.clone();
        if !checkins.iter().any(|checkin| checkin == date) {
            checkins.push(date.to_string());
            checkins.sort();
        }
        let input = HabitInput {
            name: existing.name.clone(),
            frequency: existing.frequency.clone(),
            target: existing.target.clone(),
            tags: existing.tags.clone(),
            relationships: existing.relationships.clone(),
            notes: existing.notes.clone(),
        };
        let replacement = render_habit_record(&input, &checkins);
        let markdown_body = replace_line_range(&document.markdown_body, range, &replacement)?;

        persist_habits_body(
            vault_path,
            document.document_metadata,
            &markdown_body,
            Some(date),
        )
    }
}

fn read_graph_habits_document(
    vault_path: &Path,
    summary_date: Option<&str>,
) -> Result<HabitDocument, String> {
    let metadata = ensure_habits_index_document(vault_path)?;
    let mut warnings = Vec::new();
    let mut habits = read_habits_from_folder(vault_path, HABIT_DATA_FOLDER, &mut warnings)?;
    habits.extend(read_habits_from_folder(
        vault_path,
        HABIT_LEGACY_ENTITY_FOLDER,
        &mut warnings,
    )?);
    let legacy_path = resolve_vault_relative_path(vault_path, HABITS_RELATIVE_PATH)?;
    let mut legacy_body = String::new();
    if legacy_path.is_file() {
        let legacy = read_habits_document(
            vault_path,
            ensure_habits_document(vault_path)?,
            summary_date,
        )?;
        legacy_body = legacy.markdown_body;
        habits.extend(legacy.habits);
        warnings.extend(legacy.warnings);
    }
    habits.sort_by_key(|habit| habit.name.to_lowercase());
    let summary = summarize_habits(&habits, summary_date);
    let markdown_body = render_habits_index_body(&habits, &legacy_body);
    Ok(HabitDocument {
        document_id: metadata.document_id.clone(),
        markdown_relative_path: metadata.current_path.clone(),
        markdown_body,
        habits,
        summary,
        warnings,
        document_metadata: metadata,
        layout_metadata: None,
    })
}

fn ensure_habits_index_document(vault_path: &Path) -> Result<DocumentMetadata, String> {
    LayoutFolderService::create_or_repair(vault_path)?;
    WorkspaceMetadataService::write_bootstrap_files(vault_path)?;
    let index_path = resolve_vault_relative_path(vault_path, HABIT_INDEX_PATH)?;
    let body = "# Habits\n\nThis module summarizes per-habit Markdown entities.\n";
    let document_id = generate_document_id(HABIT_INDEX_PATH);
    let metadata = DocumentMetadataService::read(vault_path, &document_id).unwrap_or_else(|_| {
        DocumentMetadataService::create_default_with_type(
            &document_id,
            HABIT_INDEX_PATH,
            body,
            "habit",
        )
        .expect("default Habits index metadata is valid")
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

fn create_v2_habit_entity(vault_path: &Path, input: HabitInput) -> Result<(), String> {
    let cleaned = clean_habit_input(input)?;
    let relative_path = unique_habit_entity_path(vault_path, &cleaned.name)?;
    let document_id = generate_document_id(&relative_path);
    persist_v2_habit_entity(vault_path, &document_id, &relative_path, &cleaned, &[])
}

fn update_v2_habit_entity(
    vault_path: &Path,
    habit_id: &str,
    input: HabitInput,
) -> Result<(), String> {
    let document_id = document_id_from_record_id(habit_id);
    let metadata = DocumentMetadataService::read(vault_path, document_id)?;
    if !is_habit_entity_path(&metadata.current_path) {
        return Err("Habit entity was not found.".to_string());
    }
    let existing = read_v2_habit(vault_path, &metadata.current_path, document_id)?;
    if existing.habit_id != habit_id {
        return Err("Habit was changed outside BentoLife.".to_string());
    }
    let cleaned = clean_habit_input(input)?;
    persist_v2_habit_entity(
        vault_path,
        document_id,
        &metadata.current_path,
        &cleaned,
        &existing.checkins,
    )
}

fn record_v2_habit_checkin(vault_path: &Path, habit_id: &str, date: &str) -> Result<(), String> {
    let document_id = document_id_from_record_id(habit_id);
    let metadata = DocumentMetadataService::read(vault_path, document_id)?;
    if !is_habit_entity_path(&metadata.current_path) {
        return Err("Habit entity was not found.".to_string());
    }
    let existing = read_v2_habit(vault_path, &metadata.current_path, document_id)?;
    if existing.habit_id != habit_id {
        return Err("Habit was changed outside BentoLife.".to_string());
    }
    let mut checkins = existing.checkins.clone();
    if !checkins.iter().any(|checkin| checkin == date) {
        checkins.push(date.to_string());
        checkins.sort();
    }
    let input = HabitInput {
        name: existing.name,
        frequency: existing.frequency,
        target: existing.target,
        tags: existing.tags,
        relationships: existing.relationships,
        notes: existing.notes,
    };
    persist_v2_habit_entity(
        vault_path,
        document_id,
        &metadata.current_path,
        &input,
        &checkins,
    )
}

fn persist_v2_habit_entity(
    vault_path: &Path,
    document_id: &str,
    relative_path: &str,
    input: &HabitInput,
    checkins: &[String],
) -> Result<(), String> {
    let body = render_habit_entity(input, checkins);
    let mut metadata = match DocumentMetadataService::read(vault_path, document_id) {
        Ok(metadata) => metadata,
        Err(_) => DocumentMetadataService::create_default_with_type(
            document_id,
            relative_path,
            &body,
            "habit",
        )?,
    };
    metadata.document_type = "habit".to_string();
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

fn read_habits_from_folder(
    vault_path: &Path,
    relative_folder: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<HabitEntry>, String> {
    let folder = resolve_vault_relative_path(vault_path, relative_folder)?;
    let mut habits = Vec::new();
    if !folder.is_dir() {
        return Ok(habits);
    }
    for entry in std::fs::read_dir(&folder)
        .map_err(|error| format!("Unable to read {}: {error}", folder.display()))?
    {
        let entry = entry.map_err(|error| format!("Unable to read Habit entity: {error}"))?;
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
        let document_id = DocumentIdentityService::find_identity_comment(&markdown)
            .map(|identity| identity.document_id)
            .unwrap_or_else(|| generate_document_id(&path.to_string_lossy()));
        let relative_path = path
            .strip_prefix(vault_path)
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
        if let Some(habit) = read_habit_entity(vault_path, &relative_path, &document_id, warnings)?
        {
            habits.push(habit);
        }
    }
    Ok(habits)
}

fn read_v2_habit(
    vault_path: &Path,
    relative_path: &str,
    document_id: &str,
) -> Result<HabitEntry, String> {
    let mut warnings = Vec::new();
    read_habit_entity(vault_path, relative_path, document_id, &mut warnings)?
        .ok_or_else(|| "Habit entity could not be parsed.".to_string())
}

fn read_habit_entity(
    vault_path: &Path,
    relative_path: &str,
    document_id: &str,
    warnings: &mut Vec<String>,
) -> Result<Option<HabitEntry>, String> {
    let markdown_path = resolve_vault_relative_path(vault_path, relative_path)?;
    let markdown = std::fs::read_to_string(&markdown_path)
        .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
    let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
    let body = DocumentIdentityService::remove_identity_comments(&parsed.body);
    Ok(parse_habit_entity(
        vault_path,
        &body,
        document_id,
        relative_path,
        warnings,
    ))
}

fn ensure_habits_document(vault_path: &Path) -> Result<DocumentMetadata, String> {
    LayoutFolderService::create_or_repair(vault_path)?;
    WorkspaceMetadataService::write_bootstrap_files(vault_path)?;

    let markdown_path = resolve_vault_relative_path(vault_path, HABITS_RELATIVE_PATH)?;
    let existing_markdown = if markdown_path.exists() {
        std::fs::read_to_string(&markdown_path)
            .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?
    } else {
        DEFAULT_HABITS_MARKDOWN.to_string()
    };

    let document_id = DocumentIdentityService::find_identity_comment(&existing_markdown)
        .map(|identity| identity.document_id)
        .unwrap_or_else(|| generate_document_id(HABITS_RELATIVE_PATH));
    let mut metadata =
        DocumentMetadataService::read(vault_path, &document_id).unwrap_or_else(|_| {
            DocumentMetadataService::create_default_with_type(
                &document_id,
                HABITS_RELATIVE_PATH,
                &existing_markdown,
                "habit",
            )
            .expect("default Habits metadata is valid")
        });

    if metadata.current_path != HABITS_RELATIVE_PATH {
        if !metadata.previous_paths.contains(&metadata.current_path) {
            metadata.previous_paths.push(metadata.current_path.clone());
        }
        metadata.current_path = HABITS_RELATIVE_PATH.to_string();
    }
    metadata.document_type = "habit".to_string();

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

fn read_habits_document(
    vault_path: &Path,
    metadata: DocumentMetadata,
    summary_date: Option<&str>,
) -> Result<HabitDocument, String> {
    let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
    let markdown = std::fs::read_to_string(&markdown_path)
        .map_err(|error| format!("Unable to read {}: {error}", markdown_path.display()))?;
    let parsed = MarkdownDocumentService::parse_frontmatter(&markdown);
    let markdown_body = normalize_body_for_response(
        &DocumentIdentityService::remove_identity_comments(&parsed.body),
    );
    let mut warnings = Vec::new();
    let habits = parse_habits(
        vault_path,
        &markdown_body,
        &metadata.current_path,
        &mut warnings,
    );
    let summary = summarize_habits(&habits, summary_date);
    let layout_metadata = LayoutMetadataService::read(vault_path, &metadata.document_id).ok();

    Ok(HabitDocument {
        document_id: metadata.document_id.clone(),
        markdown_relative_path: metadata.current_path.clone(),
        markdown_body,
        habits,
        summary,
        warnings,
        document_metadata: metadata,
        layout_metadata,
    })
}

fn persist_habits_body(
    vault_path: &Path,
    mut metadata: DocumentMetadata,
    markdown_body: &str,
    summary_date: Option<&str>,
) -> Result<HabitDocument, String> {
    let markdown_path = resolve_vault_relative_path(vault_path, &metadata.current_path)?;
    let body = normalize_body_for_write(markdown_body);
    let managed_markdown = MarkdownDocumentService::prepare_managed_markdown(
        &body,
        &metadata.document_id,
        &metadata.frontmatter_contract.required_value,
    );

    metadata.document_type = "habit".to_string();
    metadata.content_hash = content_hash(&managed_markdown);
    metadata.updated_at = current_timestamp_label();

    write_text_atomic(&markdown_path, &managed_markdown)?;
    DocumentMetadataService::write(vault_path, &metadata)?;
    ensure_layout_metadata(vault_path, &metadata.document_id, &managed_markdown)?;
    rebuild_and_register(vault_path, &metadata)?;
    read_habits_document(vault_path, metadata, summary_date)
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

fn parse_habits(
    vault_path: &Path,
    markdown_body: &str,
    source_path: &str,
    warnings: &mut Vec<String>,
) -> Vec<HabitEntry> {
    habit_ranges(markdown_body)
        .into_iter()
        .filter_map(|range| {
            parse_habit_record(vault_path, markdown_body, &range, source_path, warnings)
        })
        .collect()
}

fn parse_habit_record(
    vault_path: &Path,
    markdown_body: &str,
    range: &HabitRecordRange,
    source_path: &str,
    warnings: &mut Vec<String>,
) -> Option<HabitEntry> {
    let lines = markdown_body.lines().collect::<Vec<_>>();
    let heading = lines.get(range.start_line)?.trim();
    let name = heading.strip_prefix("## ")?.trim().to_string();
    if name.is_empty() {
        return None;
    }

    let raw_markdown = lines[range.start_line..range.end_line].join("\n");
    let mut frequency = None;
    let mut target = None;
    let mut tags = Vec::new();
    let mut relationships = Vec::new();
    let mut notes = None;
    let mut checkins = Vec::new();
    let mut in_checkins = false;

    for line in &lines[range.start_line + 1..range.end_line] {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("### Check-ins") {
            in_checkins = true;
            continue;
        }

        if in_checkins {
            if let Some(date) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                if validate_local_date(date.trim()).is_ok() {
                    checkins.push(date.trim().to_string());
                } else {
                    warnings.push(format!(
                        "{source_path}: invalid habit check-in date '{}' remains visible in Markdown.",
                        date.trim()
                    ));
                }
            }
            continue;
        }

        let Some((key, value)) = parse_field_line(line) else {
            continue;
        };
        match key.as_str() {
            "frequency" => frequency = clean_optional_string(&value),
            "target" => target = clean_optional_string(&value),
            "tags" => tags = clean_tags(value),
            "relationships" | "related" => relationships = clean_relationships(&value),
            "notes" => notes = clean_optional_markdown(&value),
            "streak" => {}
            _ => warnings.push(format!(
                "{source_path}: unknown habit field '{key}' remains visible in Markdown."
            )),
        }
    }
    if notes.is_none() {
        notes = section_body(&lines[range.start_line + 1..range.end_line], "### Notes")
            .and_then(|value| clean_optional_markdown(&value));
    }
    checkins.sort();
    checkins.dedup();
    let raw_markdown = format!("{raw_markdown}\n");
    let payload = ParsedHabitPayload {
        name: name.clone(),
        frequency: frequency.clone(),
        target: target.clone(),
        tags: tags.clone(),
        relationships: relationships.clone(),
        notes: notes.clone(),
        checkins: checkins.clone(),
    };
    let (parsed_entity, mut schema_warnings) =
        parsed_habit_payload(vault_path, &raw_markdown, source_path, payload);
    schema_warnings.extend(entry_warnings(warnings, source_path));

    Some(HabitEntry {
        habit_id: habit_id_for_record(range.start_line, raw_markdown.trim_end()),
        name,
        frequency,
        target,
        tags,
        relationships,
        notes,
        checkins,
        line_index: range.start_line,
        raw_markdown,
        parsed_entity,
        schema_warnings,
    })
}

fn habit_ranges(markdown_body: &str) -> Vec<HabitRecordRange> {
    let lines = markdown_body.lines().collect::<Vec<_>>();
    let mut starts = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.trim().starts_with("## ").then_some(index))
        .collect::<Vec<_>>();
    starts.push(lines.len());

    starts
        .windows(2)
        .map(|window| HabitRecordRange {
            start_line: window[0],
            end_line: window[1],
        })
        .collect()
}

fn habit_range_for_id(markdown_body: &str, habit_id: &str) -> Option<HabitRecordRange> {
    habit_ranges(markdown_body).into_iter().find(|range| {
        let raw_markdown = markdown_body
            .lines()
            .collect::<Vec<_>>()
            .get(range.start_line..range.end_line)
            .unwrap_or_default()
            .join("\n");
        habit_id_for_record(range.start_line, &raw_markdown) == habit_id
    })
}

fn summarize_habits(habits: &[HabitEntry], summary_date: Option<&str>) -> HabitSummary {
    let mut recent_checkins = habits
        .iter()
        .flat_map(|habit| {
            habit.checkins.iter().map(|date| HabitRecentCheckin {
                habit_id: habit.habit_id.clone(),
                name: habit.name.clone(),
                date: date.clone(),
            })
        })
        .collect::<Vec<_>>();
    recent_checkins.sort_by(|left, right| {
        right
            .date
            .cmp(&left.date)
            .then_with(|| left.name.cmp(&right.name))
    });
    recent_checkins.truncate(10);

    let checked_in_on_date = summary_date
        .map(|date| {
            habits
                .iter()
                .filter(|habit| habit.checkins.iter().any(|checkin| checkin == date))
                .count()
        })
        .unwrap_or(0);
    let streaks = habits
        .iter()
        .map(|habit| HabitStreak {
            habit_id: habit.habit_id.clone(),
            name: habit.name.clone(),
            current_streak: current_streak(habit, summary_date),
        })
        .collect();

    HabitSummary {
        total: habits.len(),
        summary_date: summary_date.map(str::to_string),
        checked_in_on_date,
        recent_checkins,
        streaks,
    }
}

fn current_streak(habit: &HabitEntry, summary_date: Option<&str>) -> usize {
    let days = habit
        .checkins
        .iter()
        .filter_map(|date| day_number(date).ok())
        .collect::<BTreeSet<_>>();
    if days.is_empty() {
        return 0;
    }
    let Some(mut day) = summary_date
        .and_then(|date| day_number(date).ok())
        .or_else(|| days.iter().next_back().copied())
    else {
        return 0;
    };

    let mut streak = 0;
    while days.contains(&day) {
        streak += 1;
        day -= 1;
    }
    streak
}

fn replace_line_range(
    markdown_body: &str,
    range: HabitRecordRange,
    replacement: &str,
) -> Result<String, String> {
    let mut lines = markdown_body
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if range.start_line >= range.end_line || range.end_line > lines.len() {
        return Err("Habit source block was not found.".to_string());
    }
    lines.splice(
        range.start_line..range.end_line,
        replacement.trim_end().lines().map(str::to_string),
    );
    Ok(format!("{}\n", lines.join("\n").trim_end()))
}

fn render_habit_record(input: &HabitInput, checkins: &[String]) -> String {
    let mut lines = vec![format!("## {}", input.name.trim())];
    push_optional_field(&mut lines, "Frequency", input.frequency.as_deref());
    push_optional_field(&mut lines, "Target", input.target.as_deref());
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
    lines.push("### Check-ins".to_string());
    for checkin in checkins {
        lines.push(format!("- {checkin}"));
    }
    format!("{}\n", lines.join("\n"))
}

fn render_habit_entity(input: &HabitInput, checkins: &[String]) -> String {
    let mut lines = vec![format!("# {}", input.name.trim())];
    push_optional_field(&mut lines, "Frequency", input.frequency.as_deref());
    push_optional_field(&mut lines, "Target", input.target.as_deref());
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
    lines.push("## Check-ins".to_string());
    for checkin in checkins {
        lines.push(format!("- {checkin}"));
    }
    format!("{}\n", lines.join("\n"))
}

fn parse_habit_entity(
    vault_path: &Path,
    markdown_body: &str,
    document_id: &str,
    source_path: &str,
    warnings: &mut Vec<String>,
) -> Option<HabitEntry> {
    let lines = markdown_body.lines().collect::<Vec<_>>();
    let heading = lines
        .iter()
        .find(|line| line.trim().starts_with("# "))?
        .trim();
    let name = heading.strip_prefix("# ")?.trim().to_string();
    let mut frequency = None;
    let mut target = None;
    let mut tags = Vec::new();
    let mut relationships = Vec::new();
    let mut notes = None;
    let mut checkins = Vec::new();
    let mut in_checkins = false;

    for line in lines.iter().skip(1) {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## Check-ins")
            || trimmed.eq_ignore_ascii_case("### Check-ins")
        {
            in_checkins = true;
            continue;
        }
        if in_checkins {
            if let Some(date) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                if validate_local_date(date.trim()).is_ok() {
                    checkins.push(date.trim().to_string());
                } else {
                    warnings.push(format!(
                        "{source_path}: invalid habit check-in date '{}' remains visible in Markdown.",
                        date.trim()
                    ));
                }
            }
            continue;
        }
        let Some((key, value)) = parse_field_line(line) else {
            continue;
        };
        match key.as_str() {
            "frequency" => frequency = clean_optional_string(&value),
            "target" => target = clean_optional_string(&value),
            "tags" => tags = clean_tags(value),
            "relationships" | "related" => relationships = clean_relationships(&value),
            "notes" => notes = clean_optional_markdown(&value),
            "streak" => {}
            _ => warnings.push(format!(
                "{source_path}: unknown habit field '{key}' remains visible in Markdown."
            )),
        }
    }
    if notes.is_none() {
        notes =
            section_body(&lines[1..], "## Notes").and_then(|value| clean_optional_markdown(&value));
    }
    checkins.sort();
    checkins.dedup();
    let raw_markdown = format!("{}\n", markdown_body.trim());
    let payload = ParsedHabitPayload {
        name: name.clone(),
        frequency: frequency.clone(),
        target: target.clone(),
        tags: tags.clone(),
        relationships: relationships.clone(),
        notes: notes.clone(),
        checkins: checkins.clone(),
    };
    let (parsed_entity, mut schema_warnings) =
        parsed_habit_payload(vault_path, &raw_markdown, source_path, payload);
    schema_warnings.extend(entry_warnings(warnings, source_path));
    Some(HabitEntry {
        habit_id: record_id(document_id, markdown_body),
        name,
        frequency,
        target,
        tags,
        relationships,
        notes,
        checkins,
        line_index: 0,
        raw_markdown,
        parsed_entity,
        schema_warnings,
    })
}

struct ParsedHabitPayload {
    name: String,
    frequency: Option<String>,
    target: Option<String>,
    tags: Vec<String>,
    relationships: Vec<String>,
    notes: Option<String>,
    checkins: Vec<String>,
}

fn parsed_habit_payload(
    vault_path: &Path,
    raw_markdown: &str,
    source_path: &str,
    payload: ParsedHabitPayload,
) -> (ParsedEntityContract, Vec<String>) {
    let mut entity = MarkdownParser::parse(raw_markdown);
    entity.module_id = Some("habits".to_string());
    entity.entity_type = Some("habit".to_string());
    entity.path = source_path.to_string();
    entity.content_hash = content_hash(raw_markdown);
    entity
        .fields
        .insert("name".to_string(), payload.name.clone());
    entity.fields.insert("title".to_string(), payload.name);
    if let Some(value) = payload.frequency {
        entity.fields.insert("frequency".to_string(), value);
    }
    if let Some(value) = payload.target {
        entity.fields.insert("target".to_string(), value);
    }
    if let Some(value) = payload.notes {
        entity.fields.insert("notes".to_string(), value);
    }
    if !payload.checkins.is_empty() {
        entity
            .fields
            .insert("checkins".to_string(), payload.checkins.join(", "));
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
    let schema_warnings =
        apply_schema_descriptors(vault_path, "modules/habits/module.schema.json", &mut entity)
            .unwrap_or_else(|error| vec![format!("Habit schema could not be applied: {error}")]);
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

fn render_habits_index_body(habits: &[HabitEntry], legacy_body: &str) -> String {
    let mut body = "# Habits\n".to_string();
    for habit in habits {
        body.push('\n');
        body.push_str(&render_habit_record(
            &HabitInput {
                name: habit.name.clone(),
                frequency: habit.frequency.clone(),
                target: habit.target.clone(),
                tags: habit.tags.clone(),
                relationships: habit.relationships.clone(),
                notes: habit.notes.clone(),
            },
            &habit.checkins,
        ));
    }
    if !legacy_body.trim().is_empty() {
        body.push_str("\n## Legacy compatibility\n- Unmigrated records from modules/habits.md are included until explicit upgrade runs.\n");
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

fn clean_habit_input(mut input: HabitInput) -> Result<HabitInput, String> {
    input.name = collapse_inline_text(&input.name);
    if input.name.is_empty() {
        return Err("Habit name is required.".to_string());
    }
    input.frequency = input
        .frequency
        .and_then(|value| clean_optional_string(&value));
    input.target = input.target.and_then(|value| clean_optional_string(&value));
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

fn unique_habit_entity_path(vault_path: &Path, name: &str) -> Result<String, String> {
    let base_slug = slugify(name);
    for index in 0..100 {
        let candidate = if index == 0 {
            format!("{HABIT_DATA_FOLDER}/{base_slug}.md")
        } else {
            format!("{HABIT_DATA_FOLDER}/{base_slug}-{index}.md")
        };
        if !resolve_vault_relative_path(vault_path, &candidate)?.exists() {
            return Ok(candidate);
        }
    }
    Err("Unable to find an available Habit filename.".to_string())
}

fn is_habit_entity_path(path: &str) -> bool {
    (path.starts_with(HABIT_DATA_FOLDER) || path.starts_with(HABIT_LEGACY_ENTITY_FOLDER))
        && path.ends_with(".md")
        && path != HABIT_INDEX_PATH
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
        "untitled-habit".to_string()
    } else {
        slug
    }
}

fn normalize_body_for_response(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        DEFAULT_HABITS_MARKDOWN.to_string()
    } else {
        format!("{body}\n")
    }
}

fn normalize_body_for_write(body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        DEFAULT_HABITS_MARKDOWN.to_string()
    } else {
        format!("{body}\n")
    }
}

fn habit_id_for_record(line_index: usize, raw_markdown: &str) -> String {
    format!("habit_{line_index}_{}", content_hash(raw_markdown))
}

fn validate_local_date(date: &str) -> Result<(), String> {
    day_number(date).map(|_| ())
}

fn day_number(date: &str) -> Result<i64, String> {
    let parts = date.split('-').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err("Habit check-in dates must use YYYY-MM-DD.".to_string());
    }
    let year = parts[0]
        .parse::<i64>()
        .map_err(|_| "Habit check-in year must be numeric.".to_string())?;
    let month = parts[1]
        .parse::<i64>()
        .map_err(|_| "Habit check-in month must be numeric.".to_string())?;
    let day = parts[2]
        .parse::<i64>()
        .map_err(|_| "Habit check-in day must be numeric.".to_string())?;
    if !(1..=12).contains(&month) || !(1..=days_in_month(year, month)).contains(&day) {
        return Err("Habit check-in date is not a valid calendar date.".to_string());
    }

    Ok(days_from_civil(year, month, day))
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month_prime + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_temp_vault(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "bentolife-habits-{name}-{}",
            current_timestamp_label().replace(':', "-")
        ));
        path.push(".bentolifevault");
        path
    }

    #[test]
    fn scaffolded_habits_index_is_not_exposed_as_a_habit_record() {
        let vault_path = unique_temp_vault("scaffold-index");
        crate::domain::dashboard::DashboardService::ensure_v3_vault_scaffold(&vault_path)
            .expect("scaffold");

        let document = HabitsService::read_habits(&vault_path, Some("2026-06-03".to_string()))
            .expect("habits read");

        assert_eq!(document.markdown_relative_path, HABIT_INDEX_PATH);
        assert!(document.habits.is_empty());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn creates_reads_and_updates_habits() {
        let vault_path = unique_temp_vault("crud");
        let document = HabitsService::create_habit(
            &vault_path,
            HabitInput {
                name: "Walk".to_string(),
                frequency: Some("Daily".to_string()),
                target: Some("20 minutes".to_string()),
                tags: vec!["health".to_string()],
                relationships: vec!["[[Contact:Mina]]".to_string()],
                notes: Some("After lunch".to_string()),
            },
            Some("2026-06-03".to_string()),
        )
        .expect("habit created");
        let habit_id = document.habits[0].habit_id.clone();
        let updated = HabitsService::update_habit(
            &vault_path,
            &habit_id,
            HabitInput {
                name: "Walk outside".to_string(),
                frequency: Some("Daily".to_string()),
                target: Some("25 minutes".to_string()),
                tags: vec!["health".to_string(), "outside".to_string()],
                relationships: vec!["[[Contact:Mina]]".to_string()],
                notes: None,
            },
            Some("2026-06-03".to_string()),
        )
        .expect("habit updated");

        assert_eq!(updated.habits[0].name, "Walk outside");
        assert_eq!(updated.habits[0].target.as_deref(), Some("25 minutes"));
        assert!(vault_path.join("modules/habits/data/walk.md").is_file());
        let metadata =
            DocumentMetadataService::read(&vault_path, &updated.document_id).expect("metadata");
        assert_eq!(metadata.document_type, "habit");

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn records_daily_checkins_idempotently_and_summarizes_streaks() {
        let vault_path = unique_temp_vault("checkins");
        let document = HabitsService::create_habit(
            &vault_path,
            HabitInput {
                name: "Read".to_string(),
                frequency: Some("Daily".to_string()),
                target: None,
                tags: vec![],
                relationships: vec![],
                notes: None,
            },
            Some("2026-06-03".to_string()),
        )
        .expect("habit created");
        let habit_id = document.habits[0].habit_id.clone();

        let first = HabitsService::record_checkin(&vault_path, &habit_id, "2026-06-02")
            .expect("first checkin");
        let next_id = first.habits[0].habit_id.clone();
        let second = HabitsService::record_checkin(&vault_path, &next_id, "2026-06-03")
            .expect("second checkin");
        let same_day_id = second.habits[0].habit_id.clone();
        let duplicate = HabitsService::record_checkin(&vault_path, &same_day_id, "2026-06-03")
            .expect("duplicate checkin");

        assert_eq!(
            duplicate.habits[0].checkins,
            vec!["2026-06-02", "2026-06-03"]
        );
        assert_eq!(duplicate.summary.checked_in_on_date, 1);
        assert_eq!(duplicate.summary.streaks[0].current_streak, 2);

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn rejects_invalid_checkin_dates() {
        assert!(validate_local_date("2026-02-29").is_err());
        assert!(validate_local_date("2024-02-29").is_ok());
        assert!(validate_local_date("06/03/2026").is_err());
    }

    #[test]
    fn rejects_stale_habit_ids_after_external_edits() {
        let vault_path = unique_temp_vault("stale");
        let document = HabitsService::create_habit(
            &vault_path,
            HabitInput {
                name: "Read".to_string(),
                frequency: None,
                target: None,
                tags: vec![],
                relationships: vec![],
                notes: None,
            },
            None,
        )
        .expect("habit created");
        let habit_id = document.habits[0].habit_id.clone();
        let habits_path = vault_path.join("modules/habits/data/read.md");
        let markdown = std::fs::read_to_string(&habits_path).expect("habits markdown");
        std::fs::write(habits_path, markdown.replace("# Read", "# Read Changed"))
            .expect("external edit");

        let result = HabitsService::record_checkin(&vault_path, &habit_id, "2026-06-03");

        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn reads_v3_habit_notes_checkins_and_reports_invalid_dates() {
        let vault_path = unique_temp_vault("external-checkins");
        let document = HabitsService::create_habit(
            &vault_path,
            HabitInput {
                name: "Read".to_string(),
                frequency: Some("Daily".to_string()),
                target: None,
                tags: vec![],
                relationships: vec![],
                notes: Some("Keep the book on the desk.\n\nNo app-only state.".to_string()),
            },
            Some("2026-06-03".to_string()),
        )
        .expect("habit created");
        let habit_id = document.habits[0].habit_id.clone();
        let habits_path = vault_path.join("modules/habits/data/read.md");
        let markdown = std::fs::read_to_string(&habits_path).expect("habit markdown");
        std::fs::write(
            &habits_path,
            markdown.replace(
                "## Check-ins",
                "- Unknown score: 10\n- Relationships: [[Contact:Mina]]\n\n## Check-ins\n- 2026-06-02\n- 2026-06-03\n- 2026-02-29",
            ),
        )
        .expect("external edit");

        let document = HabitsService::read_habits(&vault_path, Some("2026-06-03".to_string()))
            .expect("habits read");
        let habit = document
            .habits
            .iter()
            .find(|habit| {
                habit
                    .habit_id
                    .starts_with(document_id_from_record_id(&habit_id))
            })
            .unwrap_or(&document.habits[0]);

        assert_eq!(habit.checkins, vec!["2026-06-02", "2026-06-03"]);
        assert!(habit
            .notes
            .as_deref()
            .unwrap_or_default()
            .contains("No app-only state"));
        assert_eq!(document.summary.streaks[0].current_streak, 2);
        assert_eq!(habit.parsed_entity.path, "modules/habits/data/read.md");
        assert_eq!(
            habit.parsed_entity.fields.get("name"),
            Some(&"Read".to_string())
        );
        assert!(habit
            .parsed_entity
            .relationships
            .contains(&"[[Contact:Mina]]".to_string()));
        assert!(!habit.parsed_entity.content_hash.is_empty());
        assert!(habit
            .schema_warnings
            .iter()
            .any(|warning| warning.contains("unknown score")));
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.contains("invalid habit check-in date '2026-02-29'")));
        assert!(document
            .warnings
            .iter()
            .any(|warning| warning.contains("unknown habit field 'unknown score'")));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }

    #[test]
    fn preserves_existing_plain_habits_markdown_when_managing() {
        let vault_path = unique_temp_vault("plain");
        let habits_path = vault_path.join(HABITS_RELATIVE_PATH);
        std::fs::create_dir_all(habits_path.parent().expect("habits parent")).expect("parent");
        std::fs::write(
            &habits_path,
            "# Habits\n\n## Existing Habit\n- Frequency: Daily\n### Check-ins\n- 2026-06-03\n",
        )
        .expect("fixture");

        let document = HabitsService::read_habits(&vault_path, Some("2026-06-03".to_string()))
            .expect("habits read");

        assert_eq!(document.habits[0].name, "Existing Habit");
        assert_eq!(document.summary.checked_in_on_date, 1);
        assert!(std::fs::read_to_string(habits_path)
            .expect("managed markdown")
            .contains("bentolife_metadata"));

        let _ = std::fs::remove_dir_all(vault_path.parent().expect("test parent exists"));
    }
}
