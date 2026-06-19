import { contactRelationshipPolicy, relatedEntitiesTooltip } from "@/domain/entities/relationships";
import type { EntityDefinition } from "@/domain/entities/types";

export const CONTACT_RELATIONSHIP_OPTIONS = ["Friend", "Family", "Work", "Client", "Vendor", "Other"];

export const contactsEntity: EntityDefinition = {
  entityId: "contact",
  moduleId: "contacts",
  displayName: "Contact",
  pluralName: "Contacts",
  primaryTitleField: "name",
  fields: {
    name: { id: "name", label: "Name", renderer: "text", visibility: "primary", aliases: ["title"] },
    relationship: {
      id: "relationship",
      label: "Relationship",
      renderer: "contact_relationship",
      visibility: "primary",
      options: CONTACT_RELATIONSHIP_OPTIONS,
      defaultValue: "Other",
      tooltip: "Relationship means who this person is to you.",
    },
    organization: { id: "organization", label: "Organization", renderer: "text", visibility: "primary" },
    email: { id: "email", label: "Email", renderer: "text", visibility: "primary" },
    phone: { id: "phone", label: "Phone", renderer: "text", visibility: "primary" },
    tags: { id: "tags", label: "Tags", renderer: "tags", visibility: "primary" },
    body: { id: "body", label: "Notes", renderer: "textarea", visibility: "primary", aliases: ["notes"] },
    relationships: {
      id: "relationships",
      label: "Related entities",
      renderer: "entity_links",
      visibility: "advanced",
      aliases: ["related"],
      tooltip: relatedEntitiesTooltip,
    },
  },
  relationshipPolicy: contactRelationshipPolicy,
  editorLayout: {
    primary: ["name", "relationship", "organization", "email", "phone", "tags", "body"],
    advanced: ["relationships"],
  },
};
