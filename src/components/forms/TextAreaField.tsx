import { Label } from "@/components/ui/label";
import { fieldId } from "@/components/forms/TextField";

export type TextAreaFieldProps = {
  label: string;
  onChange: (value: string) => void;
  value: string;
};

export function TextAreaField({ label, onChange, value }: TextAreaFieldProps) {
  const id = fieldId(label);
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      <textarea
        className="min-h-32 w-full resize-y rounded-md border border-input bg-background px-3 py-3 text-sm leading-6 outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
        id={id}
        onChange={(event) => onChange(event.target.value)}
        value={value}
      />
    </div>
  );
}
