import { Label } from "@/components/ui/label";
import { fieldId } from "@/components/forms/TextField";
import { useI18n } from "@/i18n";

export type SelectFieldProps = {
  label: string;
  onChange: (value: string) => void;
  options: string[];
  value: string;
};

export function SelectField({ label, onChange, options, value }: SelectFieldProps) {
  const { t } = useI18n();
  const id = fieldId(label);
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      <select
        className="h-10 rounded-md border border-input bg-background px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
        id={id}
        onChange={(event) => onChange(event.target.value)}
        value={value}
      >
        <option value="">{t("forms.select.placeholder")}</option>
        {options.map((option) => (
          <option key={option} value={option}>
            {option}
          </option>
        ))}
      </select>
    </div>
  );
}
