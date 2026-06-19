export type EntityId = "note" | "todo" | "contact" | "habit";

export type EntityFieldVisibility =
  | "primary"
  | "secondary"
  | "advanced"
  | "advanced_hidden_by_default"
  | "system_hidden";

export type EntityFieldRenderer =
  | "text"
  | "textarea"
  | "markdown"
  | "select"
  | "date"
  | "tags"
  | "checklist"
  | "entity_links"
  | "contact_relationship"
  | "toggle";

export type RelationshipPolicy = {
  supportsGraphLinks: boolean;
  showGraphLinksInPrimaryEditor: boolean;
  showGraphLinksInAdvancedEditor: boolean;
  contactRelationshipField?: string;
};

export type EntityFieldDefinition = {
  id: string;
  label: string;
  renderer: EntityFieldRenderer;
  visibility: EntityFieldVisibility;
  aliases?: string[];
  options?: string[];
  defaultValue?: string | boolean;
  tooltip?: string;
};

export type EntityDefinition = {
  entityId: EntityId;
  moduleId: string;
  displayName: string;
  pluralName: string;
  primaryTitleField: string;
  fields: Record<string, EntityFieldDefinition>;
  relationshipPolicy: RelationshipPolicy;
  editorLayout: {
    primary: string[];
    secondary?: string[];
    advanced?: string[];
    hidden?: string[];
  };
};
