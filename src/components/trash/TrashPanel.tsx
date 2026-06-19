import { AlertTriangle, RefreshCw, RotateCcw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Empty } from "@/components/ui/empty";
import { useI18n } from "@/i18n";
import {
  deleteTrashEntryPermanently,
  emptyTrash,
  listTrashEntries,
  restoreTrashEntry,
  type FileLifecycleEntry,
} from "@/services/backendCore";
import type { WorkspaceRecoveryPreview } from "@/services/notes";

export function TrashPanel({ vaultPath }: { recoveryPreview: WorkspaceRecoveryPreview | null; vaultPath?: string }) {
  const { t } = useI18n();
  const [entries, setEntries] = useState<FileLifecycleEntry[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [systemMessage, setSystemMessage] = useState<string | null>(null);
  const [systemError, setSystemError] = useState<string | null>(null);

  const refreshEntries = async () => {
    if (!vaultPath) {
      setEntries([]);
      return;
    }
    setSystemError(null);
    try {
      setEntries(await listTrashEntries(vaultPath));
    } catch (error) {
      setSystemError(messageFromUnknown(error));
    }
  };

  useEffect(() => {
    void refreshEntries();
  }, [vaultPath]);

  const restoreEntry = async (entry: FileLifecycleEntry) => {
    if (!vaultPath) return;
    setBusy(entry.id);
    setSystemError(null);
    try {
      const result = await restoreTrashEntry(vaultPath, entry.id);
      setSystemMessage(`${t("lifecycle.restore")}: ${entry.file_name} -> ${result.entry.original_relative_path}.`);
      await refreshEntries();
    } catch (error) {
      setSystemError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  const deleteEntry = async (entry: FileLifecycleEntry) => {
    if (!vaultPath || !window.confirm(`${t("trash.confirmDelete")} (${entry.file_name})`)) return;
    setBusy(entry.id);
    setSystemError(null);
    try {
      const report = await deleteTrashEntryPermanently(vaultPath, entry.id);
      setEntries(report.entries);
      setSystemMessage(report.message);
    } catch (error) {
      setSystemError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  const runEmptyTrash = async () => {
    if (!vaultPath || !entries.length || !window.confirm(t("trash.confirmEmpty"))) return;
    setBusy("empty-trash");
    setSystemError(null);
    try {
      const report = await emptyTrash(vaultPath);
      setEntries(report.entries);
      setSystemMessage(report.message);
    } catch (error) {
      setSystemError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="grid gap-5">
      <Card className="shadow-none">
        <CardHeader>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <CardTitle className="text-base">{t("trash.title")}</CardTitle>
              <CardDescription>{t("trash.description")}</CardDescription>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button disabled={!vaultPath || busy !== null} onClick={() => void refreshEntries()} size="sm" variant="outline">
                <RefreshCw data-icon="inline-start" />
                {t("lifecycle.refresh")}
              </Button>
              <Button disabled={!vaultPath || !entries.length || busy !== null} onClick={runEmptyTrash} size="sm" variant="outline">
                <Trash2 data-icon="inline-start" />
                {t("trash.emptyAction")}
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="grid gap-4">
          {systemError ? <PanelNotice message={systemError} title={t("trash.failed")} tone="error" /> : null}
          {systemMessage ? <PanelNotice message={systemMessage} title={t("trash.action")} /> : null}
          {!entries.length ? (
            <Empty title={t("trash.empty.title")} description={t("trash.empty.description")} />
          ) : (
            <div className="grid gap-3">
              {entries.map((entry) => (
                <LifecycleEntryRow
                  actionLabel={t("lifecycle.restore")}
                  busy={busy === entry.id}
                  deleteLabel={t("lifecycle.deletePermanently")}
                  entry={entry}
                  key={entry.id}
                  labels={{
                    current: t("lifecycle.current"),
                    missingInternalFile: t("lifecycle.missingInternalFile"),
                    original: t("lifecycle.original"),
                    timestampUnknown: t("lifecycle.timestampUnknown"),
                    working: t("app.common.working"),
                  }}
                  onDelete={() => void deleteEntry(entry)}
                  onRestore={() => void restoreEntry(entry)}
                  showDelete
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function LifecycleEntryRow({
  actionLabel,
  busy,
  deleteLabel,
  entry,
  labels,
  onDelete,
  onRestore,
  showDelete,
}: {
  actionLabel: string;
  busy: boolean;
  deleteLabel: string;
  entry: FileLifecycleEntry;
  labels: {
    current: string;
    missingInternalFile: string;
    original: string;
    timestampUnknown: string;
    working: string;
  };
  onDelete?: () => void;
  onRestore: () => void;
  showDelete?: boolean;
}) {
  return (
    <div className="grid gap-3 rounded-md border border-border bg-background p-3 lg:grid-cols-[minmax(0,1fr)_auto]">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-2">
          <p className="font-medium">{entry.file_name}</p>
          {entry.module_id ? <Badge variant="secondary">{entry.module_id}</Badge> : null}
          {!entry.can_restore ? <Badge variant="outline">{labels.missingInternalFile}</Badge> : null}
        </div>
        <p className="mt-2 break-all text-xs text-muted-foreground">{labels.original}: {entry.original_path}</p>
        <p className="mt-1 break-all text-xs text-muted-foreground">{labels.current}: {entry.current_path}</p>
        <p className="mt-1 text-xs text-muted-foreground">
          {entry.deleted_or_archived_at ?? labels.timestampUnknown}
          {entry.size_bytes != null ? ` - ${entry.size_bytes} bytes` : ""}
        </p>
      </div>
      <div className="flex flex-wrap items-start gap-2 lg:justify-end">
        <Button disabled={busy || !entry.can_restore} onClick={onRestore} size="sm" variant="outline">
          <RotateCcw data-icon="inline-start" />
          {busy ? labels.working : actionLabel}
        </Button>
        {showDelete && onDelete ? (
          <Button disabled={busy} onClick={onDelete} size="sm" variant="outline">
            <Trash2 data-icon="inline-start" />
            {deleteLabel}
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function PanelNotice({ message, title, tone = "info" }: { message: string; title: string; tone?: "error" | "info" }) {
  return (
    <div className="flex gap-3 rounded-md border border-border bg-muted/55 p-4 text-sm">
      <AlertTriangle aria-hidden="true" className={tone === "error" ? "mt-0.5 shrink-0 text-destructive" : "mt-0.5 shrink-0 text-amber-note-foreground"} />
      <div className="min-w-0">
        <p className="font-medium">{title}</p>
        <p className="mt-1 text-muted-foreground">{message}</p>
      </div>
    </div>
  );
}

function messageFromUnknown(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
