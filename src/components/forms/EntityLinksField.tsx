import { TextField } from "@/components/forms/TextField";

export type EntityLinksFieldProps = {
  label?: string;
  onChange: (value: string) => void;
  value: string;
};

export function EntityLinksField({ label = "Related entities", onChange, value }: EntityLinksFieldProps) {
  return <TextField label={label} onChange={onChange} placeholder="[[Note title]], [[Contact:Name]]" value={value} />;
}
