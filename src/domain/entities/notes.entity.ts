import { graphRelationshipsAdvancedOnly } from "@/domain/entities/relationships";
import type { EntityDefinition } from "@/domain/entities/types";

export const notesEntity: EntityDefinition = {
  entityId: "note",
  moduleId: "notes",
  displayName: "Note",
  pluralName: "Notes",
  primaryTitleField: "title",
  fields: {
    title: { id: "title", label: "Title", renderer: "text", visibility: "primary" },
    body: { id: "body", label: "Body", renderer: "markdown", visibility: "primary" },
    tags: { id: "tags", label: "Tags", renderer: "tags", visibility: "secondary" },
    pinned: { id: "pinned", label: "Pinned", renderer: "toggle", visibility: "system_hidden" },
    relationships: {
      id: "relationships",
      label: "Related entities",
      renderer: "entity_links",
      visibility: "advanced",
      aliases: ["related"],
    },
  },
  relationshipPolicy: graphRelationshipsAdvancedOnly,
  editorLayout: {
    primary: ["title", "body"],
    secondary: ["tags"],
    advanced: ["relationships"],
    hidden: ["pinned"],
  },
};
