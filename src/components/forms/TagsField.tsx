import { TextField } from "@/components/forms/TextField";

export type TagsFieldProps = {
  label?: string;
  onChange: (value: string) => void;
  value: string;
};

export function TagsField({ label = "Tags", onChange, value }: TagsFieldProps) {
  return <TextField label={label} onChange={onChange} placeholder="tag-one, tag-two" value={value} />;
}
