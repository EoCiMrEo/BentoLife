import { graphRelationshipsAdvancedOnly, relatedEntitiesTooltip } from "@/domain/entities/relationships";
import type { EntityDefinition } from "@/domain/entities/types";

export const TODO_STATUS_OPTIONS = ["Not started", "In progress", "Waiting", "Done"];
export const TODO_PRIORITY_OPTIONS = ["Low", "Medium", "High", "Urgent"];

export const todosEntity: EntityDefinition = {
  entityId: "todo",
  moduleId: "todos",
  displayName: "Todo",
  pluralName: "Todos",
  primaryTitleField: "title",
  fields: {
    title: { id: "title", label: "Title", renderer: "text", visibility: "primary" },
    status: {
      id: "status",
      label: "Status",
      renderer: "select",
      visibility: "primary",
      options: TODO_STATUS_OPTIONS,
      defaultValue: "Not started",
    },
    priority: {
      id: "priority",
      label: "Priority",
      renderer: "select",
      visibility: "primary",
      options: TODO_PRIORITY_OPTIONS,
      defaultValue: "Medium",
    },
    due: { id: "due", label: "Due date", renderer: "date", visibility: "primary", aliases: ["due date", "deadline"] },
    tags: { id: "tags", label: "Tags", renderer: "tags", visibility: "primary" },
    checklist: { id: "checklist", label: "Checklist", renderer: "checklist", visibility: "primary" },
    body: { id: "body", label: "Body", renderer: "textarea", visibility: "primary" },
    relationships: {
      id: "relationships",
      label: "Related entities",
      renderer: "entity_links",
      visibility: "advanced_hidden_by_default",
      aliases: ["related"],
      tooltip: relatedEntitiesTooltip,
    },
  },
  relationshipPolicy: graphRelationshipsAdvancedOnly,
  editorLayout: {
    primary: ["title", "status", "priority", "due", "tags", "checklist", "body"],
    advanced: ["relationships"],
  },
};
