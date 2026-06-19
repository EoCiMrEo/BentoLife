import type { GeneratedField } from "@/components/modules/GeneratedModuleUI";
import type { EntityDefinition, EntityFieldDefinition, EntityFieldRenderer } from "@/domain/entities/types";

const rendererMap: Record<EntityFieldRenderer, GeneratedField["renderer"]> = {
  checklist: "markdown",
  contact_relationship: "status",
  date: "date",
  entity_links: "relationships",
  markdown: "markdown",
  select: "status",
  tags: "tags",
  text: "text",
  textarea: "markdown",
  toggle: "status",
};

export function fieldsForGeneratedView(entityDefinition: EntityDefinition): GeneratedField[] {
  return fieldsForVisibility(entityDefinition, ["primary", "secondary", "advanced", "advanced_hidden_by_default"]);
}

export function fieldsForEditor(entityDefinition: EntityDefinition): GeneratedField[] {
  return fieldsForVisibility(entityDefinition, ["primary", "secondary", "advanced", "advanced_hidden_by_default"]);
}

export function fieldsForWidgetSummary(entityDefinition: EntityDefinition): GeneratedField[] {
  return fieldsForVisibility(entityDefinition, ["primary", "secondary"]);
}

function fieldsForVisibility(entityDefinition: EntityDefinition, visibility: EntityFieldDefinition["visibility"][]): GeneratedField[] {
  return Object.values(entityDefinition.fields)
    .filter((field) => visibility.includes(field.visibility))
    .map((field) => ({
      id: field.id,
      label: field.label,
      renderer: rendererMap[field.renderer] ?? "generic",
      aliases: normalizeAliases(field),
      editable: false,
    }));
}

function normalizeAliases(field: EntityFieldDefinition) {
  const aliases = new Set(field.aliases ?? []);
  aliases.add(field.id.replace(/_/g, " "));
  if (field.id === "due") aliases.add("due_date");
  if (field.id === "goal") aliases.add("target");
  if (field.id === "body") aliases.add("notes");
  return [...aliases].filter((alias) => alias !== field.id);
}
