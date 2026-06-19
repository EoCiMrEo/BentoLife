import type { EntityFieldDefinition } from "@/domain/entities";
import { ChecklistField } from "@/components/forms/ChecklistField";
import { DateField } from "@/components/forms/DateField";
import { EntityLinksField } from "@/components/forms/EntityLinksField";
import { SelectField } from "@/components/forms/SelectField";
import { TagsField } from "@/components/forms/TagsField";
import { TextAreaField } from "@/components/forms/TextAreaField";
import { TextField } from "@/components/forms/TextField";

export type EntityFieldRendererProps = {
  field: EntityFieldDefinition;
  onChange: (value: string) => void;
  value: string;
};

export function EntityFieldRenderer({ field, onChange, value }: EntityFieldRendererProps) {
  switch (field.renderer) {
    case "select":
    case "contact_relationship":
      return <SelectField label={field.label} onChange={onChange} options={field.options ?? []} value={value} />;
    case "date":
      return <DateField label={field.label} onChange={onChange} value={value} />;
    case "tags":
      return <TagsField label={field.label} onChange={onChange} value={value} />;
    case "checklist":
      return <ChecklistField label={field.label} onChange={onChange} value={value} />;
    case "entity_links":
      return <EntityLinksField label={field.label} onChange={onChange} value={value} />;
    case "textarea":
    case "markdown":
      return <TextAreaField label={field.label} onChange={onChange} value={value} />;
    default:
      return <TextField label={field.label} onChange={onChange} value={value} />;
  }
}
