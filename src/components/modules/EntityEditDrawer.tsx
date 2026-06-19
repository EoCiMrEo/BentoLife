import { AlertTriangle, Save, X } from "lucide-react";
import type { ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
} from "@/components/ui/drawer";
import { useI18n } from "@/i18n";

export type RawConflictChoice = "structured" | "raw" | "convert" | "note" | "cancel";

export type RawConflictRecommendation = {
  choice: RawConflictChoice | null;
  label: string;
};

export function recommendRawConflictChoice(warnings: string[]): RawConflictRecommendation {
  if (!warnings.length) {
    return { choice: "structured", label: "Recommended: apply structured fields." };
  }
  const invalidEnumOnly = warnings.every((warning) => warning.includes("outside the approved options"));
  if (invalidEnumOnly) {
    return { choice: null, label: "Recommended: choose a valid enum value before saving." };
  }
  const unknownWarnings = warnings.filter((warning) => warning.includes("Unknown raw field") || warning.includes("unknown"));
  if (unknownWarnings.length >= 2 || warnings.some((warning) => warning.includes("Unknown content remains heavy"))) {
    return { choice: "note", label: "Recommended: save as Note so unknown-heavy Markdown stays easy to review." };
  }
  return { choice: "structured", label: "Recommended: apply structured fields and preserve unknown Markdown." };
}

const RAW_CONFLICT_OPTIONS: Array<{ value: RawConflictChoice; label: string; description: string }> = [
  {
    value: "structured",
    label: "Apply structured fields",
    description: "Save form fields and keep unknown Markdown recoverable in Notes.",
  },
  {
    value: "raw",
    label: "Keep raw Markdown",
    description: "Close without rewriting source content.",
  },
  {
    value: "convert",
    label: "Convert where possible",
    description: "Use recognized raw fields and preserve the rest as recoverable Markdown.",
  },
  {
    value: "note",
    label: "Save as Note",
    description: "Create a Note copy before applying structured fields.",
  },
  {
    value: "cancel",
    label: "Cancel",
    description: "Return to the drawer without saving.",
  },
];

function rawConflictRecommendationLabel(recommendation: RawConflictRecommendation, t: (key: string) => string) {
  switch (recommendation.choice) {
    case "structured":
      return t("modules.drawer.recommend.structured");
    case "note":
      return t("modules.drawer.recommend.note");
    case null:
      return t("modules.drawer.recommend.validEnum");
    default:
      return recommendation.label;
  }
}

function rawConflictOptionLabel(choice: RawConflictChoice, t: (key: string) => string) {
  return t(`modules.drawer.choice.${choice}`);
}

function rawConflictOptionDescription(choice: RawConflictChoice, t: (key: string) => string) {
  return t(`modules.drawer.choice.${choice}.description`);
}

type EntityEditDrawerProps = {
  children: ReactNode;
  conflictChoice?: RawConflictChoice;
  conflictWarnings?: string[];
  description?: string;
  dirty?: boolean;
  onCancel: () => void;
  onConflictChoiceChange?: (choice: RawConflictChoice) => void;
  onOpenChange: (open: boolean) => void;
  onSave: () => void;
  open: boolean;
  saveDisabled?: boolean;
  saveLabel?: string;
  title: string;
};

export function EntityEditDrawer({
  children,
  conflictChoice = "structured",
  conflictWarnings = [],
  description,
  dirty = false,
  onCancel,
  onConflictChoiceChange,
  onOpenChange,
  onSave,
  open,
  saveDisabled = false,
  saveLabel = "Save",
  title,
}: EntityEditDrawerProps) {
  const { t } = useI18n();
  const recommendation = recommendRawConflictChoice(conflictWarnings);
  const saveRequested = () => {
    if (!saveDisabled) {
      onSave();
    }
  };
  const requestOpenChange = (nextOpen: boolean) => {
    if (!nextOpen && dirty && !window.confirm(t("modules.drawer.discardConfirm"))) {
      return;
    }
    onOpenChange(nextOpen);
  };

  return (
    <Drawer onOpenChange={requestOpenChange} open={open}>
      <DrawerContent
        onKeyDown={(event) => {
          if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
            event.preventDefault();
            saveRequested();
          }
        }}
      >
        <DrawerHeader>
          <div className="flex flex-wrap items-center gap-2">
            <DrawerTitle>{title}</DrawerTitle>
            {dirty ? <Badge variant="outline">{t("modules.drawer.unsaved")}</Badge> : null}
          </div>
          {description ? <DrawerDescription>{description}</DrawerDescription> : null}
        </DrawerHeader>
        <div className="min-h-0 flex-1 overflow-auto p-5">
          <div className="grid gap-4">
            {conflictWarnings.length ? (
              <section className="rounded-md border border-amber-300 bg-amber-50 p-3 text-sm text-amber-950" aria-label={t("modules.drawer.rawConflictChoices")}>
                <div className="flex items-start gap-2">
                  <AlertTriangle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
                  <div className="min-w-0">
                    <p className="font-medium">{t("modules.drawer.rawNeedsChoice")}</p>
                    <ul className="mt-2 list-disc space-y-1 pl-5">
                      {conflictWarnings.map((warning) => (
                        <li key={warning}>{warning}</li>
                      ))}
                    </ul>
                    <p className="mt-2 rounded-md bg-amber-100 px-2 py-1 text-xs font-medium">
                      {rawConflictRecommendationLabel(recommendation, t)}
                    </p>
                  </div>
                </div>
                {onConflictChoiceChange ? (
                  <div className="mt-3 grid gap-2">
                    {RAW_CONFLICT_OPTIONS.map((option) => (
                      <label className="flex items-start gap-2 rounded-md border border-amber-200 bg-background/70 p-2" key={option.value}>
                        <input
                          checked={conflictChoice === option.value}
                          className="mt-1"
                          name={`${title}-raw-conflict-choice`}
                          onChange={() => onConflictChoiceChange(option.value)}
                          type="radio"
                        />
                        <span>
                          <span className="block font-medium">{rawConflictOptionLabel(option.value, t)}</span>
                          <span className="block text-xs leading-5 text-muted-foreground">{rawConflictOptionDescription(option.value, t)}</span>
                        </span>
                      </label>
                    ))}
                  </div>
                ) : null}
              </section>
            ) : null}
            {children}
          </div>
        </div>
        <div className="flex flex-wrap justify-end gap-3 border-t border-border p-4">
          <Button onClick={onCancel} variant="outline">
            <X data-icon="inline-start" />
            {t("app.actions.cancel")}
          </Button>
          <Button disabled={saveDisabled} onClick={saveRequested}>
            <Save data-icon="inline-start" />
            {saveLabel}
          </Button>
        </div>
      </DrawerContent>
    </Drawer>
  );
}
