import { graphRelationshipsAdvancedOnly, relatedEntitiesTooltip } from "@/domain/entities/relationships";
import type { EntityDefinition } from "@/domain/entities/types";

export const HABIT_FREQUENCY_OPTIONS = ["Daily", "Weekly", "Monthly", "Custom"];

export const habitsEntity: EntityDefinition = {
  entityId: "habit",
  moduleId: "habits",
  displayName: "Habit",
  pluralName: "Habits",
  primaryTitleField: "name",
  fields: {
    name: { id: "name", label: "Name", renderer: "text", visibility: "primary", aliases: ["title"] },
    frequency: {
      id: "frequency",
      label: "Frequency",
      renderer: "select",
      visibility: "primary",
      options: HABIT_FREQUENCY_OPTIONS,
      defaultValue: "Daily",
    },
    goal: { id: "goal", label: "Goal", renderer: "text", visibility: "primary", aliases: ["target"] },
    tags: { id: "tags", label: "Tags", renderer: "tags", visibility: "primary" },
    checkins: { id: "checkins", label: "Check-ins", renderer: "date", visibility: "primary", aliases: ["check-ins"] },
    body: { id: "body", label: "Notes", renderer: "textarea", visibility: "primary", aliases: ["notes"] },
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
    primary: ["name", "frequency", "goal", "tags", "checkins", "body"],
    advanced: ["relationships"],
  },
};
