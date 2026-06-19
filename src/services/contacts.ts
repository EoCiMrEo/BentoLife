import { invoke } from "@tauri-apps/api/core";

import { emptyContactDocument, normalizeContactDocument } from "@/services/contracts/contacts.contract";
import { safeInvoke } from "@/services/contracts/invokeClient";
import type { LayoutMetadata } from "@/services/notes";
import type { ParsedEntityContract } from "@/services/backendCore";
import { isTauriRuntime } from "@/services/vault";

export type ContactInput = {
  name: string;
  relationship?: string | null;
  organization?: string | null;
  email?: string | null;
  phone?: string | null;
  tags?: string[];
  relationships?: string[];
  notes?: string | null;
};

export type TagCount = {
  tag: string;
  count: number;
};

export type ContactEntry = Required<Omit<ContactInput, "relationship" | "organization" | "email" | "phone" | "notes">> & {
  contact_id: string;
  relationship: string | null;
  organization: string | null;
  email: string | null;
  phone: string | null;
  notes: string | null;
  line_index: number;
  raw_markdown: string;
  parsed_entity: ParsedEntityContract;
  schema_warnings: string[];
};

export type ContactSummary = {
  total: number;
  relationship_counts: Record<string, number>;
  top_tags: TagCount[];
  contacts_with_email: number;
  contacts_with_phone: number;
};

export type ContactDocument = {
  document_id: string;
  markdown_relative_path: string;
  markdown_body: string;
  contacts: ContactEntry[];
  summary: ContactSummary;
  warnings: string[];
  layout_metadata: LayoutMetadata | null;
};

const mockContactsStorageKey = "bentolife:mockContacts";

export async function readContacts(vaultPath: string) {
  if (!isTauriRuntime()) {
    return mockContactDocument();
  }

  const result = await safeInvoke("read_contacts", { vaultPath }, normalizeContactDocument, emptyContactDocument());
  return result.data;
}

export async function createContact(vaultPath: string, contact: ContactInput) {
  if (!isTauriRuntime()) {
    return mockCreateContact(contact);
  }

  return normalizeContactDocument(await invoke<unknown>("create_contact", { vaultPath, contact }));
}

export async function updateContact(vaultPath: string, contactId: string, contact: ContactInput) {
  if (!isTauriRuntime()) {
    return mockUpdateContact(contactId, contact);
  }

  return normalizeContactDocument(await invoke<unknown>("update_contact", { vaultPath, contactId, contact }));
}

function mockCreateContact(input: ContactInput): ContactDocument {
  const contacts = readMockContacts();
  const contact = normalizeContactInput(input, `contact_${Date.now().toString(36)}_${contacts.length}`);
  writeMockContacts([...contacts, contact]);
  return mockContactDocument();
}

function mockUpdateContact(contactId: string, input: ContactInput): ContactDocument {
  const contacts = readMockContacts();
  const index = contacts.findIndex((contact) => contact.contact_id === contactId);
  if (index < 0) {
    throw new Error("Contact was not found or was changed outside BentoLife.");
  }
  contacts[index] = normalizeContactInput(input, contactId);
  writeMockContacts(contacts);
  return mockContactDocument();
}

function mockContactDocument(): ContactDocument {
  const contacts = readMockContacts().map((contact, index) => ({
    ...contact,
    line_index: index * 7 + 2,
    raw_markdown: renderContact(contact),
    parsed_entity: parsedContactEntity(contact),
    schema_warnings: [],
  }));

  return {
    document_id: "bl_doc_mock_contacts",
    markdown_relative_path: "modules/contacts/INDEX.md",
    markdown_body: `# Contacts\n\n${contacts.map(renderContact).join("\n")}`.trimEnd() + "\n",
    contacts,
    summary: summarizeContacts(contacts),
    warnings: [],
    layout_metadata: null,
  };
}

function parsedContactEntity(contact: MockContact): ParsedEntityContract {
  const rawMarkdown = renderContact(contact);
  const fields: Record<string, string> = {
    name: contact.name,
    title: contact.name,
  };
  if (contact.relationship) fields.relationship = contact.relationship;
  if (contact.organization) fields.organization = contact.organization;
  if (contact.email) fields.email = contact.email;
  if (contact.phone) fields.phone = contact.phone;
  if (contact.relationships.length) fields.relationships = contact.relationships.join(", ");
  if (contact.notes) fields.notes = contact.notes;
  return {
    module_id: "contacts",
    entity_type: "contact",
    fields,
    field_descriptors: [
      { id: "name", label: "Name", type: "text", renderer_id: "text", value: contact.name, editable: false, aliases: ["title"], warnings: [] },
      { id: "relationship", label: "Relationship", type: "enum", renderer_id: "status", value: contact.relationship ?? "Other", editable: false, aliases: [], options: ["Friend", "Family", "Work", "Client", "Vendor", "Other"], default_value: "Other", warnings: [] },
      { id: "organization", label: "Organization", type: "text", renderer_id: "text", value: contact.organization ?? "", editable: false, aliases: [], warnings: [] },
      { id: "email", label: "Email", type: "text", renderer_id: "text", value: contact.email ?? "", editable: false, aliases: [], warnings: [] },
      { id: "phone", label: "Phone", type: "text", renderer_id: "text", value: contact.phone ?? "", editable: false, aliases: [], warnings: [] },
      { id: "tags", label: "Tags", type: "tags", renderer_id: "tags", value: contact.tags.join(", "), editable: false, aliases: [], warnings: [] },
      { id: "relationships", label: "Relationships", type: "relationship", renderer_id: "relationships", value: contact.relationships.join(", "), editable: false, aliases: ["related"], warnings: [] },
      { id: "notes", label: "Notes", type: "markdown", renderer_id: "markdown", value: contact.notes ?? "", editable: false, aliases: [], warnings: [] },
    ],
    blocks: [{ type: "paragraph", text: rawMarkdown }],
    unknown_blocks: [],
    relationships: contact.relationships,
    tags: contact.tags,
    path: "modules/contacts/INDEX.md",
    content_hash: contact.contact_id,
  };
}

type MockContact = Omit<ContactEntry, "line_index" | "raw_markdown" | "parsed_entity" | "schema_warnings">;

function readMockContacts(): MockContact[] {
  const serialized = window.localStorage.getItem(mockContactsStorageKey);
  if (!serialized) {
    return [];
  }
  try {
    return JSON.parse(serialized) as MockContact[];
  } catch {
    return [];
  }
}

function writeMockContacts(contacts: MockContact[]) {
  window.localStorage.setItem(mockContactsStorageKey, JSON.stringify(contacts));
}

function normalizeContactInput(input: ContactInput, contactId: string): MockContact {
  const name = collapseText(input.name);
  if (!name) {
    throw new Error("Contact name is required.");
  }
  return {
    contact_id: contactId,
    name,
    relationship: cleanOptional(input.relationship),
    organization: cleanOptional(input.organization),
    email: cleanOptional(input.email),
    phone: cleanOptional(input.phone),
    tags: normalizeTags(input.tags ?? []),
    relationships: normalizeTags(input.relationships ?? []),
    notes: cleanOptionalMarkdown(input.notes),
  };
}

function summarizeContacts(contacts: MockContact[]): ContactSummary {
  const relationship_counts: Record<string, number> = {};
  const tagCounts = new Map<string, number>();
  for (const contact of contacts) {
    if (contact.relationship) {
      relationship_counts[contact.relationship] = (relationship_counts[contact.relationship] ?? 0) + 1;
    }
    for (const tag of contact.tags) {
      tagCounts.set(tag, (tagCounts.get(tag) ?? 0) + 1);
    }
  }
  const top_tags = [...tagCounts.entries()]
    .map(([tag, count]) => ({ tag, count }))
    .sort((left, right) => right.count - left.count || left.tag.localeCompare(right.tag))
    .slice(0, 5);

  return {
    total: contacts.length,
    relationship_counts,
    top_tags,
    contacts_with_email: contacts.filter((contact) => contact.email).length,
    contacts_with_phone: contacts.filter((contact) => contact.phone).length,
  };
}

function renderContact(contact: MockContact) {
  const lines = [`## ${contact.name}`];
  if (contact.relationship) lines.push(`- Relationship: ${contact.relationship}`);
  if (contact.organization) lines.push(`- Organization: ${contact.organization}`);
  if (contact.email) lines.push(`- Email: ${contact.email}`);
  if (contact.phone) lines.push(`- Phone: ${contact.phone}`);
  if (contact.tags.length) lines.push(`- Tags: ${contact.tags.join(", ")}`);
  if (contact.relationships.length) lines.push(`- Relationships: ${contact.relationships.join(", ")}`);
  if (contact.notes) lines.push("### Notes", contact.notes);
  return `${lines.join("\n")}\n`;
}

function normalizeTags(tags: string[]) {
  return [...new Set(tags.flatMap((tag) => tag.split(",")).map(collapseText).filter(Boolean))].sort();
}

function cleanOptional(value?: string | null) {
  const cleaned = collapseText(value ?? "");
  return cleaned || null;
}

function cleanOptionalMarkdown(value?: string | null) {
  const cleaned = (value ?? "")
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .join("\n")
    .trim();
  return cleaned || null;
}

function collapseText(value: string) {
  return value.trim().split(/\s+/).filter(Boolean).join(" ");
}
