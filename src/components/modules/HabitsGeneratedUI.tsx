import { GeneratedModuleUI } from "@/components/modules/GeneratedModuleUI";
import { entityDefinitions } from "@/domain/entities";
import { fieldsForGeneratedView } from "@/domain/entities/generatedFields";
import type { HabitDocument, HabitEntry } from "@/services/habits";

export interface HabitsGeneratedUIProps {
  backlinks?: Array<{ source_path: string; target: string; link_type: string; status: string; raw: string }>;
  document: HabitDocument;
  habit: HabitEntry;
}

const habitFields = fieldsForGeneratedView(entityDefinitions.habit);

export function HabitsGeneratedUI({ backlinks = [], document, habit }: HabitsGeneratedUIProps) {
  return (
    <GeneratedModuleUI
      backlinks={backlinks}
      documentId={document.document_id}
      entity={habit.parsed_entity}
      fields={habitFields}
      moduleId="habits"
      moduleLabel="Habits"
      schemaWarnings={[...document.warnings, ...habit.schema_warnings]}
      sourceMarkdown={habit.raw_markdown}
      title={habit.name}
    />
  );
}
