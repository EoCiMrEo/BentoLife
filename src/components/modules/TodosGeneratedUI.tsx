import { GeneratedModuleUI } from "@/components/modules/GeneratedModuleUI";
import { entityDefinitions } from "@/domain/entities";
import { fieldsForGeneratedView } from "@/domain/entities/generatedFields";
import type { TodoDocument } from "@/services/todo";

export interface TodosGeneratedUIProps {
  backlinks?: Array<{ source_path: string; target: string; link_type: string; status: string; raw: string }>;
  todos: TodoDocument;
}

const todoFields = fieldsForGeneratedView(entityDefinitions.todo);

export function TodosGeneratedUI({ backlinks = [], todos }: TodosGeneratedUIProps) {
  return (
    <GeneratedModuleUI
      backlinks={backlinks}
      documentId={todos.document_id}
      entity={todos.parsed_entity}
      fields={todoFields}
      moduleId="todos"
      moduleLabel="Todos"
      schemaWarnings={todos.schema_warnings}
      sourceMarkdown={todos.markdown_body}
      title={todos.parsed_entity?.fields?.title || todos.title}
    />
  );
}
