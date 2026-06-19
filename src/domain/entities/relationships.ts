import type { RelationshipPolicy } from "@/domain/entities/types";

export const graphRelationshipsAdvancedOnly: RelationshipPolicy = {
  supportsGraphLinks: true,
  showGraphLinksInPrimaryEditor: false,
  showGraphLinksInAdvancedEditor: true,
};

export const contactRelationshipPolicy: RelationshipPolicy = {
  ...graphRelationshipsAdvancedOnly,
  contactRelationshipField: "relationship",
};

export const relatedEntitiesTooltip =
  "Related entities are optional links to notes, contacts, habits, or other tasks. They are used for graph/search context and are not required for normal todos.";
