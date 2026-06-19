import { type ReactNode } from "react";

import { Empty } from "@/components/ui/empty";
import { useI18n } from "@/i18n";

export function ModuleBrowsePanel({ children }: { children: ReactNode }) {
  return <div className="flex min-w-0 flex-col gap-4">{children}</div>;
}

export function FocusSurfaceHeader({ actions, children }: { actions?: ReactNode; children: ReactNode }) {
  return <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto_auto]">{children}{actions}</div>;
}

export function ViewEditToggle({
  isEditing,
  onToggle,
  viewLabel = "View",
  editLabel = "Edit",
}: {
  isEditing: boolean;
  onToggle: () => void;
  viewLabel?: string;
  editLabel?: string;
}) {
  return (
    <button
      className="self-end rounded-md border border-input bg-background px-3 py-2 text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      onClick={onToggle}
      type="button"
    >
      {isEditing ? viewLabel : editLabel}
    </button>
  );
}

export function InspectorPanel({ children, title = "Inspector" }: { children: ReactNode; title?: string }) {
  return (
    <details className="rounded-md border border-border bg-muted/35 p-3 text-sm">
      <summary className="cursor-pointer font-medium">{title}</summary>
      <div className="mt-3 grid gap-2 text-muted-foreground">{children}</div>
    </details>
  );
}

export function ModuleEmptyState({ description, title }: { description: string; title: string }) {
  return <Empty title={title} description={description} />;
}

export function OperationMessage({ message, title }: { message: string; title: string }) {
  return (
    <div className="rounded-md border border-border bg-amber-note/20 p-3 text-sm text-amber-note-foreground">
      <p className="font-medium">{title}</p>
      <p className="mt-1 leading-5">{message}</p>
    </div>
  );
}

export function SchemaWarningSummary({ warnings }: { warnings: string[] }) {
  const { t } = useI18n();
  if (!warnings.length) {
    return <p className="text-sm text-muted-foreground">{t("modules.editor.noSchemaWarnings")}</p>;
  }
  return (
    <div className="grid gap-2">
      {warnings.map((warning) => (
        <p className="rounded-md border border-border bg-amber-note/20 p-2 text-sm text-amber-note-foreground" key={warning}>
          {warning}
        </p>
      ))}
    </div>
  );
}
