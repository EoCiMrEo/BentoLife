import { AlertTriangle, CheckCircle2, RefreshCw, SlidersHorizontal } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { useI18n } from "@/i18n";
import type { VaultInspection } from "@/services/vault";

type VaultPanelProps = {
  inspection?: VaultInspection;
  onResetVault: () => void;
  resetting: boolean;
};

export function VaultStatusPanel({ inspection, onResetVault, resetting }: VaultPanelProps) {
  const { t } = useI18n();
  return (
    <div className="flex flex-col gap-5">
      <div className="grid gap-4 md:grid-cols-2">
        <Card className="shadow-none">
          <CardHeader>
            <CardTitle className="text-base">{t("vault.status.title")}</CardTitle>
            <CardDescription>{t("vault.status.description")}</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-3 text-sm">
            {inspection ? <VaultStatusBlock inspection={inspection} /> : <Skeleton className="h-28" />}
          </CardContent>
        </Card>
        <Card className="shadow-none">
          <CardHeader>
            <CardTitle className="text-base">{t("vault.controls.title")}</CardTitle>
            <CardDescription>{t("vault.controls.description")}</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-3">
            <Dialog>
              <DialogTrigger asChild>
                <Button variant="outline">
                  <SlidersHorizontal data-icon="inline-start" />
                  {t("vault.details.open")}
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>{t("vault.details.title")}</DialogTitle>
                  <DialogDescription>{inspection?.path ?? t("vault.details.noPath")}</DialogDescription>
                </DialogHeader>
                {inspection ? <VaultStatusBlock inspection={inspection} /> : null}
              </DialogContent>
            </Dialog>
            <Button disabled={resetting} onClick={onResetVault} variant="ghost">
              <RefreshCw data-icon="inline-start" />
              {resetting ? t("vault.resetting") : t("vault.chooseAnother")}
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export function VaultStatusBlock({ hideMissingPaths = false, inspection }: { hideMissingPaths?: boolean; inspection: VaultInspection }) {
  const { t } = useI18n();
  const ready = inspection.state === "ready";

  return (
    <div className="flex flex-col gap-3">
      <div className="flex gap-3 rounded-md bg-muted px-3 py-3">
        {ready ? (
          <CheckCircle2 aria-hidden="true" className="mt-0.5 shrink-0 text-sage-foreground" />
        ) : (
          <AlertTriangle aria-hidden="true" className="mt-0.5 shrink-0 text-amber-note-foreground" />
        )}
        <div className="min-w-0">
          <p className="font-medium">{ready ? t("vault.ready") : t("vault.needsAttention")}</p>
          <p className="mt-1 break-words text-muted-foreground">{inspection.message}</p>
        </div>
      </div>
      <div className="rounded-md bg-muted px-3 py-2">
        <p className="text-xs font-medium uppercase text-muted-foreground">{t("vault.path")}</p>
        <p className="mt-1 break-all">{inspection.path}</p>
      </div>
      {inspection.older_version_detected ? (
        <div className="rounded-md bg-muted px-3 py-2">
          <p className="text-xs font-medium uppercase text-muted-foreground">{t("vault.resetPolicy")}</p>
          <p className="mt-1 text-sm text-muted-foreground">{t("vault.resetPolicy.description")}</p>
        </div>
      ) : null}
      {!hideMissingPaths && inspection.missing_paths.length > 0 ? (
        <div className="rounded-md bg-muted px-3 py-2">
          <p className="text-xs font-medium uppercase text-muted-foreground">{t("vault.missing")}</p>
          <ul className="mt-2 flex flex-col gap-1">
            {inspection.missing_paths.map((path) => (
              <li className="break-all" key={path}>
                {path}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
  );
}
