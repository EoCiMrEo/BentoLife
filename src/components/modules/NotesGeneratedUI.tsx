import { GeneratedModuleUI, type GeneratedField } from "@/components/modules/GeneratedModuleUI";
import type { NoteDocument } from "@/services/notes";

export interface NotesGeneratedUIProps {
  backlinks?: Array<{ source_path: string; target: string; link_type: string; status: string; raw: string }>;
  hideBody?: boolean;
  note: NoteDocument;
  vaultPath?: string | null;
}

const noteFields: GeneratedField[] = [
  { id: "title", label: "Title", renderer: "text" },
  { id: "tags", label: "Tags", renderer: "tags" },
  { id: "relationships", label: "Relationships", renderer: "relationships", aliases: ["related"] },
  { id: "body", label: "Body Blocks", renderer: "markdown" },
];

export function NotesGeneratedUI({ backlinks = [], hideBody = false, note, vaultPath }: NotesGeneratedUIProps) {
  return (
    <GeneratedModuleUI
      backlinks={backlinks}
      documentId={note.document_id}
      entity={note.parsed_entity}
      fields={noteFields}
      hideBody={hideBody}
      moduleId="notes"
      moduleLabel="Notes"
      schemaWarnings={note.schema_warnings}
      sourceMarkdown={note.markdown_body}
      title={note.parsed_entity.fields.title || note.title}
      vaultPath={vaultPath}
    />
  );
}
