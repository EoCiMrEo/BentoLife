import { GeneratedModuleUI } from "@/components/modules/GeneratedModuleUI";
import { entityDefinitions } from "@/domain/entities";
import { fieldsForGeneratedView } from "@/domain/entities/generatedFields";
import type { ContactDocument, ContactEntry } from "@/services/contacts";

export interface ContactsGeneratedUIProps {
  backlinks?: Array<{ source_path: string; target: string; link_type: string; status: string; raw: string }>;
  document: ContactDocument;
  contact: ContactEntry;
}

const contactFields = fieldsForGeneratedView(entityDefinitions.contact);

export function ContactsGeneratedUI({ backlinks = [], contact, document }: ContactsGeneratedUIProps) {
  return (
    <GeneratedModuleUI
      backlinks={backlinks}
      documentId={document.document_id}
      entity={contact.parsed_entity}
      fields={contactFields}
      moduleId="contacts"
      moduleLabel="Contacts"
      schemaWarnings={[...document.warnings, ...contact.schema_warnings]}
      sourceMarkdown={contact.raw_markdown}
      title={contact.name}
    />
  );
}
