import { TextField } from "@/components/forms/TextField";

export type ChecklistFieldProps = {
  label?: string;
  onChange: (value: string) => void;
  value: string;
};

export function ChecklistField({ label = "Checklist starter item", onChange, value }: ChecklistFieldProps) {
  return <TextField label={label} onChange={onChange} placeholder="First checklist item" value={value} />;
}
