import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export type TextFieldProps = {
  label: string;
  onChange: (value: string) => void;
  placeholder?: string;
  value: string;
};

export function fieldId(label: string) {
  return label.toLowerCase().replace(/[^a-z0-9]+/g, "-");
}

export function TextField({ label, onChange, placeholder, value }: TextFieldProps) {
  const id = fieldId(label);
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} onChange={(event) => onChange(event.target.value)} placeholder={placeholder} value={value} />
    </div>
  );
}
