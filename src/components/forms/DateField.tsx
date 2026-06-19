import { CalendarDays } from "lucide-react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { fieldId } from "@/components/forms/TextField";
import { useI18n } from "@/i18n";

export type DateFieldProps = {
  label: string;
  onChange: (value: string) => void;
  value: string;
};

export function isValidDateFieldValue(value: string) {
  if (!value.trim()) return true;
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
  const date = new Date(`${value}T00:00:00Z`);
  return !Number.isNaN(date.getTime()) && date.toISOString().slice(0, 10) === value;
}

export function DateField({ label, onChange, value }: DateFieldProps) {
  const { t } = useI18n();
  const id = fieldId(label);
  const valid = isValidDateFieldValue(value);
  return (
    <div className="flex min-w-0 flex-col gap-2">
      <Label htmlFor={id}>{label}</Label>
      <div className="relative">
        <CalendarDays aria-hidden="true" className="pointer-events-none absolute left-3 top-3 size-4 text-muted-foreground" />
        <Input
          aria-describedby={valid ? undefined : `${id}-error`}
          aria-invalid={!valid}
          className="pl-9"
          id={id}
          onChange={(event) => onChange(event.target.value)}
          placeholder="YYYY-MM-DD"
          type="date"
          value={valid ? value : ""}
        />
      </div>
      {!valid ? (
        <p className="text-xs text-destructive" id={`${id}-error`}>
          {t("forms.date.invalid")}
        </p>
      ) : (
        <p className="text-xs text-muted-foreground">{t("forms.date.savedFormat")}</p>
      )}
    </div>
  );
}
