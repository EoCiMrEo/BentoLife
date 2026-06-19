import { AlertTriangle, CopyCheck, Eye, FolderOpen, Globe2, Palette, SlidersHorizontal, Upload } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { VaultStatusPanel } from "@/components/vault/VaultStatusPanel";
import {
  createVaultSnapshot as createVaultTransferSnapshot,
  acceptImportIntoModule,
  importFolderIntoVault,
  ignoreStagedImport,
  listStagedImports,
  previewFolderImport,
  previewSnapshotRestore,
  previewVaultSnapshot,
  stageSnapshotForImport,
  restoreVaultSnapshot,
  type FolderImportManifest,
  type ImportAcceptanceOptions,
  type FolderImportPreview,
  type SnapshotStageReport,
  type SnapshotRestorePreview,
  type SnapshotRestoreReport,
  type StagedImportIndex,
  type StagedImportRecord,
  type VaultSnapshotManifest,
  type VaultSnapshotPreview,
} from "@/services/backendCore";
import type { ActiveThemeState, ThemePreview, ThemeScope } from "@/services/theme";
import { selectFolder, type VaultInspection } from "@/services/vault";
import { useI18n, type AppLanguage } from "@/i18n";

export type SettingsPanelProps = {
  activeModuleId: string | null;
  activeTheme: ActiveThemeState | null;
  onApplyTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  onCancelThemePreview: () => void;
  onOpenArchitect: () => void;
  onOpenModule: (moduleId: string) => void;
  onPreviewTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  language: AppLanguage;
  onLanguageChange: (language: AppLanguage) => void;
  onRefreshWorkspace: () => Promise<void> | void;
  onReviewRecoveryIssues: () => void;
  onRollbackTheme: (scope: ThemeScope, moduleId: string | null) => void;
  onResetVault: () => void;
  recoveryIssueCount: number;
  resetting: boolean;
  themePreview: ThemePreview | null;
  vaultInspection?: VaultInspection;
  vaultPath?: string;
};

type ImportMappingRule = "suggested" | "source-folder" | "current";
type ImportConflictChoice = "accept" | "note" | "edit" | "ignore";

export function SettingsPanel({
  activeModuleId,
  activeTheme,
  onApplyTheme,
  onCancelThemePreview,
  onOpenArchitect,
  onOpenModule,
  language,
  onLanguageChange,
  onPreviewTheme,
  onRefreshWorkspace,
  onReviewRecoveryIssues,
  onRollbackTheme,
  onResetVault,
  recoveryIssueCount,
  resetting,
  themePreview,
  vaultInspection,
  vaultPath,
}: SettingsPanelProps) {
  const { t } = useI18n();
  void activeModuleId;
  void onApplyTheme;
  void onCancelThemePreview;
  void onPreviewTheme;
  void onRollbackTheme;
  void themePreview;

  return (
    <div className="grid gap-5">
      <Card className="shadow-none">
        <CardHeader>
          <CardTitle className="text-base">{t("settings.appearance.title")}</CardTitle>
          <CardDescription>{t("settings.appearance.description")}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 lg:grid-cols-[1fr_auto]">
          <div className="grid gap-2 text-sm">
            <SummaryRow label={t("settings.theme.workspace")} value={activeTheme?.workspace_theme.theme_id ?? "clean-slate"} />
            <SummaryRow label={t("settings.theme.moduleOverrides")} value={`${Object.keys(activeTheme?.module_themes ?? {}).length}`} />
          </div>
          <div className="flex flex-wrap items-start justify-end gap-2">
            <Button onClick={onOpenArchitect} variant="outline">
              <Palette data-icon="inline-start" />
              {t("settings.theme.openRegistry")}
            </Button>
          </div>
          <div className="rounded-md border border-border bg-muted/35 p-3 lg:col-span-2">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <div className="flex items-center gap-2">
                  <Globe2 className="size-4 text-muted-foreground" aria-hidden="true" />
                  <p className="text-sm font-medium">{t("app.language.title")}</p>
                </div>
                <p className="mt-1 text-sm text-muted-foreground">{t("app.language.description")}</p>
              </div>
              <div className="flex rounded-md border border-border bg-background p-1" role="group" aria-label={t("app.language.title")}>
                <Button
                  aria-label={`${t("app.language.title")}: ${t("app.language.english")}`}
                  aria-pressed={language === "en"}
                  onClick={() => onLanguageChange("en")}
                  size="sm"
                  type="button"
                  variant={language === "en" ? "default" : "ghost"}
                >
                  🇺🇸 English
                </Button>
                <Button
                  aria-label={`${t("app.language.title")}: ${t("app.language.vietnamese")}`}
                  aria-pressed={language === "vi"}
                  onClick={() => onLanguageChange("vi")}
                  size="sm"
                  type="button"
                  variant={language === "vi" ? "default" : "ghost"}
                >
                  🇻🇳 Tiếng Việt
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <section className="grid gap-3" aria-label={t("settings.vaultData.title")}>
        <div>
          <h2 className="text-lg font-semibold">{t("settings.vaultData.title")}</h2>
          <p className="mt-1 text-sm text-muted-foreground">{t("settings.vaultData.description")}</p>
        </div>
        <VaultStatusPanel
          inspection={vaultInspection}
          onResetVault={onResetVault}
          resetting={resetting}
        />
        {recoveryIssueCount > 0 ? (
          <Card className="border-amber-note-border bg-amber-note shadow-none">
            <CardHeader>
              <CardTitle className="text-base">{t("settings.review.title")}</CardTitle>
              <CardDescription>
                {recoveryIssueCount} vault metadata or recovery {recoveryIssueCount === 1 ? "issue needs" : "issues need"} attention.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Button onClick={onReviewRecoveryIssues} variant="outline">
                <SlidersHorizontal data-icon="inline-start" />
                {t("settings.review.openRecovery")}
              </Button>
            </CardContent>
          </Card>
        ) : null}
        <ImportTransferPanel onOpenModule={onOpenModule} onWorkspaceChanged={onRefreshWorkspace} vaultPath={vaultPath} />
      </section>
    </div>
  );
}

function ImportTransferPanel({
  onOpenModule,
  onWorkspaceChanged,
  vaultPath,
}: {
  onOpenModule: (moduleId: string) => void;
  onWorkspaceChanged?: () => Promise<void> | void;
  vaultPath?: string;
}) {
  const { t } = useI18n();
  const [folderSourcePath, setFolderSourcePath] = useState("");
  const [folderPreview, setFolderPreview] = useState<FolderImportPreview | null>(null);
  const [folderManifest, setFolderManifest] = useState<FolderImportManifest | null>(null);
  const [folderError, setFolderError] = useState<string | null>(null);
  const [folderRefreshMessage, setFolderRefreshMessage] = useState<string | null>(null);
  const [snapshotSourcePath, setSnapshotSourcePath] = useState("");
  const [snapshotPath, setSnapshotPath] = useState("snapshot-export");
  const [snapshotPreview, setSnapshotPreview] = useState<VaultSnapshotPreview | null>(null);
  const [snapshotManifest, setSnapshotManifest] = useState<VaultSnapshotManifest | null>(null);
  const [snapshotError, setSnapshotError] = useState<string | null>(null);
  const [restoreSnapshotPath, setRestoreSnapshotPath] = useState("snapshot-export");
  const [restoreTargetPath, setRestoreTargetPath] = useState("");
  const [restorePreview, setRestorePreview] = useState<SnapshotRestorePreview | null>(null);
  const [restoreReport, setRestoreReport] = useState<SnapshotRestoreReport | null>(null);
  const [snapshotStageReport, setSnapshotStageReport] = useState<SnapshotStageReport | null>(null);
  const [restoreError, setRestoreError] = useState<string | null>(null);
  const [restoreRefreshMessage, setRestoreRefreshMessage] = useState<string | null>(null);
  const [stagedIndex, setStagedIndex] = useState<StagedImportIndex | null>(null);
  const [importReviewFilter, setImportReviewFilter] = useState<"accepted" | "all" | "conflicts" | "ignored" | "unreviewed">("unreviewed");
  const [selectedStagedPath, setSelectedStagedPath] = useState<string | null>(null);
  const [selectedBulkPaths, setSelectedBulkPaths] = useState<string[]>([]);
  const [selectedTargetModule, setSelectedTargetModule] = useState("notes");
  const [mappingRule, setMappingRule] = useState<ImportMappingRule>("suggested");
  const [schemaConflictChoice, setSchemaConflictChoice] = useState<ImportConflictChoice>("accept");
  const [targetFilename, setTargetFilename] = useState("");
  const [importTags, setImportTags] = useState("imported");
  const [importReviewMessage, setImportReviewMessage] = useState<string | null>(null);
  const [importReviewError, setImportReviewError] = useState<string | null>(null);
  const [showImportReview, setShowImportReview] = useState(false);
  const importReviewRef = useRef<HTMLDivElement | null>(null);
  const [busy, setBusy] = useState<
    | "folder-preview"
    | "folder-import"
    | "snapshot-preview"
    | "snapshot-create"
    | "restore-preview"
    | "restore-apply"
    | "restore-stage"
    | "import-accept"
    | "import-bulk"
    | "import-ignore"
    | "import-refresh"
    | null
  >(null);

  useEffect(() => {
    if (!vaultPath) {
      return;
    }
    setSnapshotSourcePath((current) => current || vaultPath);
    setRestoreTargetPath((current) => current || vaultPath);
    void refreshStagedImports(vaultPath);
  }, [vaultPath]);

  const refreshStagedImports = async (path = vaultPath) => {
    if (!path) {
      return;
    }
    setBusy((current) => current ?? "import-refresh");
    try {
      const index = await listStagedImports(path);
      setStagedIndex(index);
      setShowImportReview((current) => current || index.records.length > 0);
      setSelectedStagedPath((current) => current ?? index.records.find((record) => !record.accepted && !record.ignored)?.staged_file_path ?? null);
      setSelectedBulkPaths((current) =>
        current.filter((stagedPath) =>
          index.records.some((record) => record.staged_file_path === stagedPath && !record.accepted && !record.ignored),
        ),
      );
    } catch (error) {
      setImportReviewError(getErrorMessage(error));
    } finally {
      setBusy((current) => current === "import-refresh" ? null : current);
    }
  };

  const runFolderPreview = async () => {
    if (!vaultPath) {
      setFolderError("Select or create a vault before importing folders.");
      return;
    }
    setBusy("folder-preview");
    setFolderError(null);
    setFolderManifest(null);
    setFolderRefreshMessage(null);
    try {
      setFolderPreview(await previewFolderImport(folderSourcePath, vaultPath));
    } catch (error) {
      setFolderPreview(null);
      setFolderError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runFolderImport = async () => {
    if (!vaultPath) {
      setFolderError("Select or create a vault before importing folders.");
      return;
    }
    setBusy("folder-import");
    setFolderError(null);
    setFolderRefreshMessage(null);
    try {
      setFolderManifest(await importFolderIntoVault(folderSourcePath, vaultPath));
      await refreshStagedImports(vaultPath);
      setShowImportReview(true);
      setImportReviewMessage(t("settings.import.review.description"));
      window.setTimeout(() => importReviewRef.current?.scrollIntoView({ block: "start", behavior: "smooth" }), 0);
      await onWorkspaceChanged?.();
      setFolderRefreshMessage(t("app.actions.refresh"));
    } catch (error) {
      setFolderError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runSnapshotPreview = async () => {
    setBusy("snapshot-preview");
    setSnapshotError(null);
    setSnapshotManifest(null);
    try {
      setSnapshotPreview(await previewVaultSnapshot(snapshotSourcePath, snapshotPath));
    } catch (error) {
      setSnapshotPreview(null);
      setSnapshotError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runSnapshotCreate = async () => {
    setBusy("snapshot-create");
    setSnapshotError(null);
    try {
      setSnapshotManifest(await createVaultTransferSnapshot(snapshotSourcePath, snapshotPath));
    } catch (error) {
      setSnapshotError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runRestorePreview = async () => {
    setBusy("restore-preview");
    setRestoreError(null);
    setRestoreReport(null);
    setSnapshotStageReport(null);
    setRestoreRefreshMessage(null);
    try {
      setRestorePreview(await previewSnapshotRestore(restoreSnapshotPath, restoreTargetPath));
    } catch (error) {
      setRestorePreview(null);
      setRestoreError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runRestoreApply = async () => {
    if (restorePreview && !restorePreview.direct_restore_allowed) {
      setRestoreError(restorePreview.blocked_reason ?? t("settings.restore.staged"));
      return;
    }
    setBusy("restore-apply");
    setRestoreError(null);
    setRestoreRefreshMessage(null);
    try {
      setRestoreReport(await restoreVaultSnapshot(restoreSnapshotPath, restoreTargetPath));
      if (vaultPath && restoreTargetPath === vaultPath) {
        await onWorkspaceChanged?.();
        setRestoreRefreshMessage(t("app.actions.refresh"));
      }
    } catch (error) {
      setRestoreError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runSnapshotStage = async () => {
    setBusy("restore-stage");
    setRestoreError(null);
    setRestoreReport(null);
    setRestoreRefreshMessage(null);
    try {
      const report = await stageSnapshotForImport(restoreSnapshotPath, restoreTargetPath);
      setSnapshotStageReport(report);
      setStagedIndex(report.index);
      setSelectedStagedPath(report.staged_files[0]?.staged_file_path ?? null);
      setImportReviewFilter("unreviewed");
      setShowImportReview(true);
      setImportReviewMessage(t("settings.import.review.description"));
      window.setTimeout(() => importReviewRef.current?.scrollIntoView({ block: "start", behavior: "smooth" }), 0);
    } catch (error) {
      setRestoreError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const selectedRecord = stagedIndex?.records.find((record) => record.staged_file_path === selectedStagedPath) ?? null;
  const hasStagedRecords = (stagedIndex?.records.length ?? 0) > 0;

  const runAcceptStagedImport = async () => {
    if (!vaultPath || !selectedRecord) {
      return;
    }
    setBusy("import-accept");
    setImportReviewError(null);
    setImportReviewMessage(null);
    const options: ImportAcceptanceOptions = {
      target_filename: targetFilename.trim() || null,
      tags: splitTags(importTags),
      preserve_source_path: true,
      batch_tag: null,
    };
    try {
      if (schemaConflictChoice === "edit") {
        setImportReviewMessage(t("settings.import.review.choice.edit.description"));
        return;
      }
      if (schemaConflictChoice === "ignore") {
        const ignored = await ignoreStagedImport(vaultPath, selectedRecord.staged_file_path);
        setStagedIndex(ignored.index);
        setImportReviewMessage(t("settings.import.review.ignored"));
        return;
      }
      const targetModule = schemaConflictChoice === "note" ? "notes" : moduleForImportRecord(selectedRecord, selectedTargetModule, mappingRule);
      const report = await acceptImportIntoModule(vaultPath, selectedRecord.staged_file_path, targetModule, options);
      await refreshStagedImports(vaultPath);
      await onWorkspaceChanged?.();
      setImportReviewMessage(`${t("settings.import.review.accepted")}: ${report.accepted_relative_path}.`);
    } catch (error) {
      setImportReviewError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runIgnoreStagedImport = async () => {
    if (!vaultPath || !selectedRecord) {
      return;
    }
    setBusy("import-ignore");
    setImportReviewError(null);
    setImportReviewMessage(null);
    try {
      const report = await ignoreStagedImport(vaultPath, selectedRecord.staged_file_path);
      setStagedIndex(report.index);
      setImportReviewMessage(t("settings.import.review.ignored"));
    } catch (error) {
      setImportReviewError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runBulkAcceptImports = async () => {
    if (!vaultPath || !selectedBulkPaths.length) {
      return;
    }
    setBusy("import-bulk");
    setImportReviewError(null);
    setImportReviewMessage(null);
    const options: ImportAcceptanceOptions = {
      target_filename: null,
      tags: splitTags(importTags),
      preserve_source_path: true,
      batch_tag: "imported",
    };
    try {
      const accepted = [];
      const errors: string[] = [];
      for (const stagedPath of selectedBulkPaths) {
        const record = stagedIndex?.records.find((candidate) => candidate.staged_file_path === stagedPath);
        if (!record) {
          errors.push(`${stagedPath}: Staged import was not found.`);
          continue;
        }
        try {
          accepted.push(await acceptImportIntoModule(vaultPath, stagedPath, moduleForImportRecord(record, selectedTargetModule, mappingRule), options));
        } catch (error) {
          errors.push(`${stagedPath}: ${getErrorMessage(error)}`);
        }
      }
      await refreshStagedImports(vaultPath);
      await onWorkspaceChanged?.();
      setSelectedBulkPaths([]);
      setImportReviewMessage(`${t("settings.import.review.accepted")}: ${accepted.length} (${mappingRuleLabel(mappingRule, t)}).`);
      if (errors.length) {
        setImportReviewError(errors.join("; "));
      }
    } catch (error) {
      setImportReviewError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const runBulkIgnoreImports = async () => {
    if (!vaultPath || !selectedBulkPaths.length) {
      return;
    }
    setBusy("import-bulk");
    setImportReviewError(null);
    setImportReviewMessage(null);
    try {
      let nextIndex = stagedIndex;
      for (const stagedPath of selectedBulkPaths) {
        const report = await ignoreStagedImport(vaultPath, stagedPath);
        nextIndex = report.index;
      }
      setStagedIndex(nextIndex);
      setSelectedBulkPaths([]);
      setImportReviewMessage(t("settings.import.review.ignoreSelected"));
    } catch (error) {
      setImportReviewError(getErrorMessage(error));
    } finally {
      setBusy(null);
    }
  };

  const chooseFolder = async (title: string, update: (path: string) => void) => {
    const selected = await selectFolder(title);
    if (selected) {
      update(selected);
    }
  };

  return (
    <Card className="shadow-none">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <CopyCheck aria-hidden="true" data-icon="inline-start" />
          {t("settings.importTransfer.title")}
        </CardTitle>
        <CardDescription>
          {t("settings.importTransfer.description")}
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-5 xl:grid-cols-2">
        <div className="flex min-w-0 flex-col gap-4">
          <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
            <div className="flex min-w-0 flex-col gap-2">
              <Label htmlFor="folder-import-source">{t("settings.import.folder.source")}</Label>
              <Input
                id="folder-import-source"
                onChange={(event) => setFolderSourcePath(event.target.value)}
                placeholder="obsidian-source"
                value={folderSourcePath}
              />
            </div>
            <Button
              className="self-end"
              onClick={() => void chooseFolder(t("settings.import.folder.selectTitle"), setFolderSourcePath)}
              type="button"
              variant="outline"
            >
              <FolderOpen data-icon="inline-start" />
              {t("settings.import.folder.choose")}
            </Button>
          </div>
          <div className="flex flex-wrap gap-3">
            <Button disabled={busy !== null || !folderSourcePath.trim()} onClick={runFolderPreview} variant="outline">
              <Eye data-icon="inline-start" />
              {busy === "folder-preview" ? t("settings.import.folder.previewing") : t("settings.import.folder.previewAction")}
            </Button>
            <Button disabled={busy !== null || !folderPreview} onClick={runFolderImport}>
              <Upload data-icon="inline-start" />
              {busy === "folder-import" ? t("app.common.importing") : t("settings.import.folder.importAction")}
            </Button>
          </div>
          {folderError ? <RepairNotice title={t("settings.import.folder.failed")} message={folderError} /> : null}
          {folderPreview ? <FolderImportSummary preview={folderPreview} /> : null}
          {folderManifest ? (
            <div className="rounded-md border border-border bg-muted/55 p-3 text-sm">
              <Badge variant="status">{t("settings.import.folder.complete")}</Badge>
              <p className="mt-2 text-muted-foreground">
                {folderManifest.files.filter((file) => file.copied).length} {t("settings.import.folder.filesCopied")} {folderManifest.target_root}.
              </p>
              {folderRefreshMessage ? <p className="mt-2 text-xs text-muted-foreground">{folderRefreshMessage}</p> : null}
            </div>
          ) : null}
        </div>

        <div className="flex min-w-0 flex-col gap-5">
          <div className="grid gap-3 md:grid-cols-2">
            <div className="flex min-w-0 flex-col gap-2">
              <Label htmlFor="snapshot-source-vault">{t("settings.snapshot.sourceVault")}</Label>
              <Input id="snapshot-source-vault" onChange={(event) => setSnapshotSourcePath(event.target.value)} value={snapshotSourcePath} />
            </div>
            <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
              <div className="flex min-w-0 flex-col gap-2">
                <Label htmlFor="snapshot-output-path">{t("settings.snapshot.folderPath")}</Label>
                <Input id="snapshot-output-path" onChange={(event) => setSnapshotPath(event.target.value)} value={snapshotPath} />
              </div>
              <Button
                aria-label={t("settings.snapshot.chooseOutput")}
                className="self-end"
                onClick={() => void chooseFolder(t("settings.snapshot.selectFolder"), setSnapshotPath)}
                size="icon"
                type="button"
                variant="outline"
              >
                <FolderOpen className="size-4" aria-hidden="true" />
              </Button>
            </div>
          </div>
          <div className="flex flex-wrap gap-3">
            <Button disabled={busy !== null || !snapshotSourcePath.trim() || !snapshotPath.trim()} onClick={runSnapshotPreview} variant="outline">
              <Eye data-icon="inline-start" />
              {busy === "snapshot-preview" ? t("settings.snapshot.previewing") : t("settings.snapshot.previewAction")}
            </Button>
            <Button disabled={busy !== null || !snapshotPreview} onClick={runSnapshotCreate}>
              <Upload data-icon="inline-start" />
              {busy === "snapshot-create" ? t("settings.snapshot.creating") : t("settings.snapshot.createAction")}
            </Button>
          </div>
          {snapshotError ? <RepairNotice title={t("settings.snapshot.failed")} message={snapshotError} /> : null}
          {snapshotPreview ? (
            <div className="rounded-md border border-border bg-muted/55 p-3 text-sm">
              <Badge variant="status">{t("settings.snapshot.preview")}</Badge>
              <SummaryRow label={t("settings.summary.files")} value={`${snapshotPreview.file_count}`} />
              <SummaryRow label={t("settings.summary.bytes")} value={`${snapshotPreview.total_bytes}`} />
            </div>
          ) : null}
          {snapshotManifest ? (
            <div className="rounded-md border border-border bg-muted/55 p-3 text-sm">
              <Badge variant="status">{t("settings.snapshot.created")}</Badge>
              <p className="mt-2 break-all text-muted-foreground">{snapshotManifest.snapshot_path}</p>
            </div>
          ) : null}

          <Separator />

          <div className="grid gap-3 md:grid-cols-2">
            <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
              <div className="flex min-w-0 flex-col gap-2">
                <Label htmlFor="restore-snapshot-path">{t("settings.restore.snapshotPath")}</Label>
                <Input id="restore-snapshot-path" onChange={(event) => setRestoreSnapshotPath(event.target.value)} value={restoreSnapshotPath} />
              </div>
              <Button
                aria-label={t("settings.restore.chooseSnapshot")}
                className="self-end"
                onClick={() => void chooseFolder(t("settings.restore.selectSnapshot"), setRestoreSnapshotPath)}
                size="icon"
                type="button"
                variant="outline"
              >
                <FolderOpen className="size-4" aria-hidden="true" />
              </Button>
            </div>
            <div className="flex min-w-0 flex-col gap-2">
              <Label htmlFor="restore-target-vault">{t("settings.restore.targetVault")}</Label>
              <Input id="restore-target-vault" onChange={(event) => setRestoreTargetPath(event.target.value)} value={restoreTargetPath} />
            </div>
          </div>
          <div className="flex flex-wrap gap-3">
            <Button disabled={busy !== null || !restoreSnapshotPath.trim() || !restoreTargetPath.trim()} onClick={runRestorePreview} variant="outline">
              <Eye data-icon="inline-start" />
              {busy === "restore-preview" ? t("settings.restore.previewing") : t("settings.restore.previewAction")}
            </Button>
            <Button disabled={busy !== null || !restorePreview || !restorePreview.direct_restore_allowed} onClick={runRestoreApply}>
              <Upload data-icon="inline-start" />
              {busy === "restore-apply" ? t("settings.restore.restoring") : t("settings.restore.directAction")}
            </Button>
            <Button
              disabled={busy !== null || !restorePreview || restorePreview.direct_restore_allowed}
              onClick={runSnapshotStage}
              variant="outline"
            >
              <Upload data-icon="inline-start" />
              {busy === "restore-stage" ? t("settings.restore.staging") : t("settings.restore.stageAction")}
            </Button>
          </div>
          {restoreError ? <RepairNotice title={t("settings.restore.failed")} message={restoreError} /> : null}
          {restorePreview ? (
            <div className="rounded-md border border-border bg-muted/55 p-3 text-sm">
              <Badge variant={restorePreview.conflicts.length ? "outline" : "status"}>{t("settings.restore.preview")}</Badge>
              <SummaryRow label={t("settings.summary.files")} value={`${restorePreview.file_count}`} />
              <SummaryRow label={t("settings.summary.snapshotShape")} value={restorePreview.snapshot_shape.replace(/_/g, " ")} />
              <SummaryRow label={t("settings.summary.legacyFiles")} value={`${restorePreview.legacy_file_count}`} />
              <SummaryRow label={t("settings.summary.activeV3Files")} value={`${restorePreview.active_v3_file_count}`} />
              <SummaryRow label={t("settings.summary.recommendedAction")} value={restorePreview.recommended_action} />
              <SummaryRow label={t("settings.summary.conflicts")} value={`${restorePreview.conflicts.length}`} />
              <SnapshotPathDetails label={t("settings.summary.legacyFilePaths")} paths={restorePreview.legacy_file_paths} />
              <SnapshotPathDetails label={t("settings.summary.activeV3FilePaths")} paths={restorePreview.active_v3_file_paths} />
              <SnapshotPathDetails label={t("settings.summary.hiddenRuntimeFilePaths")} paths={restorePreview.hidden_runtime_file_paths} />
              {!restorePreview.direct_restore_allowed ? (
                <p className="mt-3 rounded-md border border-warning/40 bg-warning/10 p-2 text-xs text-foreground">
                  {t("settings.restore.legacyWarning")}
                </p>
              ) : null}
            </div>
          ) : null}
          {snapshotStageReport ? (
            <div className="rounded-md border border-border bg-muted/55 p-3 text-sm">
              <Badge variant="status">{t("settings.restore.staged")}</Badge>
              <p className="mt-2 break-all text-muted-foreground">{snapshotStageReport.staged_root}</p>
              <p className="mt-2 text-xs text-muted-foreground">{snapshotStageReport.staged_files.length} {t("settings.restore.filesReadyForReview")}</p>
            </div>
          ) : null}
          {restoreReport ? (
            <div className="rounded-md border border-border bg-muted/55 p-3 text-sm">
              <Badge variant="status">{t("settings.restore.restored")}</Badge>
              <p className="mt-2 text-muted-foreground">
                {restoreReport.restored_files.length} {t("settings.restore.filesRestored")} {restoreReport.cache.index.updated_at}.
              </p>
              {restoreRefreshMessage ? <p className="mt-2 text-xs text-muted-foreground">{restoreRefreshMessage}</p> : null}
            </div>
          ) : null}
        </div>
      </CardContent>
      {showImportReview && hasStagedRecords ? (
        <CardContent className="mt-5 border-t border-border pt-5" ref={importReviewRef}>
          <ImportReviewPanel
            busy={busy}
            filter={importReviewFilter}
            index={stagedIndex}
            message={importReviewMessage}
            onAccept={runAcceptStagedImport}
            onBulkAccept={runBulkAcceptImports}
            onBulkIgnore={runBulkIgnoreImports}
            onFilterChange={setImportReviewFilter}
            onIgnore={runIgnoreStagedImport}
            onMappingRuleChange={setMappingRule}
            onOpenAcceptedModule={() => {
              if (selectedRecord) {
                onOpenModule(schemaConflictChoice === "note" ? "notes" : moduleForImportRecord(selectedRecord, selectedTargetModule, mappingRule));
              }
            }}
            onRefresh={() => void refreshStagedImports()}
            onSelect={(path) => {
              setSelectedStagedPath(path);
              const record = stagedIndex?.records.find((candidate) => candidate.staged_file_path === path);
              setSelectedTargetModule(record?.suggested_module ?? "notes");
              setSchemaConflictChoice("accept");
              setTargetFilename(record?.detected_title ?? "");
            }}
            onSchemaConflictChoiceChange={setSchemaConflictChoice}
            onToggleBulkPath={(path) =>
              setSelectedBulkPaths((current) =>
                current.includes(path) ? current.filter((candidate) => candidate !== path) : [...current, path],
              )
            }
            onTagsChange={setImportTags}
            onTargetFilenameChange={setTargetFilename}
            onTargetModuleChange={setSelectedTargetModule}
            selectedPath={selectedStagedPath}
            selectedBulkPaths={selectedBulkPaths}
            selectedRecord={selectedRecord}
            schemaConflictChoice={schemaConflictChoice}
            tags={importTags}
            targetFilename={targetFilename}
            mappingRule={mappingRule}
            targetModule={selectedTargetModule}
            error={importReviewError}
          />
        </CardContent>
      ) : null}
    </Card>
  );
}

function splitTags(value: string) {
  const seen = new Set<string>();
  return value
    .split(/[,\s]+/)
    .map((tag) => tag.trim().replace(/^#/, ""))
    .filter(Boolean)
    .filter((tag) => {
      const key = tag.toLowerCase();
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

function ImportReviewPanel({
  busy,
  error,
  filter,
  index,
  message,
  onAccept,
  onBulkAccept,
  onBulkIgnore,
  onFilterChange,
  onIgnore,
  onMappingRuleChange,
  onOpenAcceptedModule,
  onRefresh,
  onSelect,
  onSchemaConflictChoiceChange,
  onToggleBulkPath,
  onTagsChange,
  onTargetFilenameChange,
  onTargetModuleChange,
  selectedPath,
  selectedBulkPaths,
  selectedRecord,
  schemaConflictChoice,
  tags,
  targetFilename,
  mappingRule,
  targetModule,
}: {
  busy: string | null;
  error: string | null;
  filter: "accepted" | "all" | "conflicts" | "ignored" | "unreviewed";
  index: StagedImportIndex | null;
  message: string | null;
  onAccept: () => void;
  onBulkAccept: () => void;
  onBulkIgnore: () => void;
  onFilterChange: (filter: "accepted" | "all" | "conflicts" | "ignored" | "unreviewed") => void;
  onIgnore: () => void;
  onMappingRuleChange: (rule: ImportMappingRule) => void;
  onOpenAcceptedModule: () => void;
  onRefresh: () => void;
  onSelect: (path: string) => void;
  onSchemaConflictChoiceChange: (choice: ImportConflictChoice) => void;
  onToggleBulkPath: (path: string) => void;
  onTagsChange: (tags: string) => void;
  onTargetFilenameChange: (filename: string) => void;
  onTargetModuleChange: (module: string) => void;
  selectedPath: string | null;
  selectedBulkPaths: string[];
  selectedRecord: StagedImportRecord | null;
  schemaConflictChoice: ImportConflictChoice;
  tags: string;
  targetFilename: string;
  mappingRule: ImportMappingRule;
  targetModule: string;
}) {
  const { t } = useI18n();
  const records = index?.records ?? [];
  const filtered = records.filter((record) => {
    switch (filter) {
      case "accepted":
        return record.accepted;
      case "ignored":
        return record.ignored;
      case "conflicts":
        return Boolean(record.conflict_status);
      case "unreviewed":
        return !record.accepted && !record.ignored;
      default:
        return true;
    }
  });
  const selected = selectedRecord ?? filtered[0] ?? null;
  const targetName = targetFilename.trim() || selected?.detected_title || selected?.staged_file_path.split(/[\\/]/).pop()?.replace(/\.md$/i, "") || "imported-file";
  const resolvedTargetModule = selected ? moduleForImportRecord(selected, targetModule, mappingRule) : targetModule;
  const acceptedTargetModule = schemaConflictChoice === "note" ? "notes" : resolvedTargetModule;
  const targetPreview = `modules/${acceptedTargetModule}/data/${targetName.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "imported-file"}.md`;
  const recommendation = selected ? importReviewRecommendation(selected, targetModule, mappingRule, t) : null;
  const hiddenSystemFiles = index?.hidden_system_files ?? [];
  const hiddenSystemCount = index?.hidden_system_count ?? 0;

  return (
    <section className="grid gap-4" aria-label={t("settings.import.review.title")}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="text-base font-semibold">{t("settings.import.review.title")}</h3>
          <p className="mt-1 text-sm text-muted-foreground">{t("settings.import.review.description")}</p>
        </div>
        <Button disabled={busy !== null} onClick={onRefresh} size="sm" type="button" variant="outline">
          <Eye data-icon="inline-start" />
          {t("settings.import.review.refresh")}
        </Button>
      </div>
      <div className="rounded-md border border-border bg-muted/35 p-3 text-sm text-muted-foreground">
        <p>
          {records.length} {t("settings.import.review.userFilesReady")} {hiddenSystemCount} {t("settings.import.review.systemFilesHidden")}
        </p>
        {hiddenSystemFiles.length ? (
          <details className="mt-2 [&:not([open])>*:not(summary)]:hidden">
            <summary className="cursor-pointer text-xs font-medium text-foreground">
              {t("settings.import.review.hiddenSystemFiles")}
            </summary>
            <ul className="mt-2 grid max-h-32 gap-1 overflow-auto text-xs">
              {hiddenSystemFiles.slice(0, 40).map((path) => (
                <li className="break-all" key={path}>{path}</li>
              ))}
            </ul>
          </details>
        ) : null}
      </div>
      <div className="flex flex-wrap gap-2" role="toolbar" aria-label={t("settings.import.review.filters")}>
        {(["all", "unreviewed", "accepted", "ignored", "conflicts"] as const).map((candidate) => (
          <Button
            key={candidate}
            onClick={() => onFilterChange(candidate)}
            size="sm"
            type="button"
            variant={filter === candidate ? "default" : "outline"}
          >
            {filterLabel(candidate, t)}
          </Button>
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-2 rounded-md border border-border bg-muted/35 p-3" role="toolbar" aria-label={t("settings.import.review.bulkActions")}>
        <span className="text-xs text-muted-foreground">{selectedBulkPaths.length} {t("settings.import.review.selected")}</span>
        <Button disabled={busy !== null || !selectedBulkPaths.length} onClick={onBulkAccept} size="sm" type="button" variant="outline">
          {busy === "import-bulk" ? t("app.common.applying") : `${t("settings.import.review.accepted")} (${mappingRuleLabel(mappingRule, t)})`}
        </Button>
        <Button disabled={busy !== null || !selectedBulkPaths.length} onClick={onBulkIgnore} size="sm" type="button" variant="outline">
          {t("settings.import.review.ignoreSelected")}
        </Button>
        <span className="text-xs text-muted-foreground">{t("settings.import.review.mappingRuleNote")}</span>
      </div>
      {error ? <RepairNotice title={t("settings.import.review.failed")} message={error} /> : null}
      {message ? <p className="rounded-md border border-border bg-muted/55 p-3 text-sm text-muted-foreground">{message}</p> : null}
      <div className="grid gap-4 lg:grid-cols-[minmax(220px,0.8fr)_minmax(0,1.2fr)]">
        <div className="grid gap-2">
          {filtered.length ? (
            filtered.map((record) => (
              <div
                className={`grid gap-2 rounded-md border p-3 text-sm transition hover:border-primary ${
                  (selectedPath ?? selected?.staged_file_path) === record.staged_file_path ? "border-primary bg-muted/70" : "border-border bg-background"
                }`}
                key={record.staged_file_path}
              >
                <label className="flex items-start gap-2 text-xs text-muted-foreground">
                  <input
                    checked={selectedBulkPaths.includes(record.staged_file_path)}
                    className="mt-0.5"
                    disabled={record.accepted || record.ignored}
                    onChange={() => onToggleBulkPath(record.staged_file_path)}
                    type="checkbox"
                  />
                  {t("settings.import.review.selectBulk")}
                </label>
                <button className="text-left" onClick={() => onSelect(record.staged_file_path)} type="button">
                  <span className="block font-medium">{record.detected_title ?? record.staged_file_path}</span>
                  <span className="mt-1 block break-all text-xs text-muted-foreground">{record.staged_file_path}</span>
                  <span className="mt-2 flex flex-wrap gap-2">
                    <Badge variant={record.accepted ? "status" : record.ignored ? "outline" : "secondary"}>
                      {record.accepted ? t("settings.import.review.accepted") : record.ignored ? t("settings.import.review.ignored") : t("settings.import.review.unreviewed")}
                    </Badge>
                    <Badge variant="outline">{sourceKindLabel(record.source_kind, t)}</Badge>
                    <Badge variant="outline">{record.suggested_module}</Badge>
                    {record.conflict_status ? <Badge className="border-warning/40 text-foreground" variant="outline">{t("settings.import.review.conflict")}</Badge> : null}
                  </span>
                </button>
              </div>
            ))
          ) : (
            <p className="rounded-md border border-border bg-muted/55 p-3 text-sm text-muted-foreground">{t("settings.import.review.emptyFilter")}</p>
          )}
        </div>
        <div className="rounded-md border border-border bg-background p-4">
          {selected ? (
            <div className="grid gap-4">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <h4 className="font-medium">{selected.detected_title ?? "Markdown"}</h4>
                  <p className="mt-1 break-all text-xs text-muted-foreground">{selected.original_source_path ?? selected.staged_file_path}</p>
                </div>
                <Badge className={selected.conflict_status ? "border-warning/40 text-foreground" : undefined} variant={selected.conflict_status ? "outline" : "status"}>
                  {selected.conflict_status ? t("settings.import.review.needsReview") : t("app.common.ready")}
                </Badge>
              </div>
              <div className="grid gap-2 text-sm sm:grid-cols-2">
                <SummaryRow label={t("settings.import.review.suggestedModule")} value={selected.suggested_module} />
                <SummaryRow label={t("settings.import.review.sourceKind")} value={sourceKindLabel(selected.source_kind, t)} />
                <SummaryRow label={t("settings.import.review.checklists")} value={`${selected.detected_checklists}`} />
                <SummaryRow label={t("settings.import.review.tags")} value={selected.detected_tags.length ? selected.detected_tags.join(", ") : t("app.common.none")} />
                <SummaryRow label={t("settings.import.review.links")} value={selected.detected_links.length ? `${selected.detected_links.length}` : t("app.common.none")} />
              </div>
              {selected.conflict_status ? <RepairNotice title={t("settings.import.review.conflictWarning")} message={selected.conflict_status} /> : null}
              <div className="grid gap-2">
                <Label htmlFor="import-mapping-rule">{t("settings.import.review.mappingRule")}</Label>
                <select
                  className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                  id="import-mapping-rule"
                  onChange={(event) => onMappingRuleChange(event.target.value as ImportMappingRule)}
                  value={mappingRule}
                >
                  <option value="suggested">{t("settings.import.review.rule.suggested")}</option>
                  <option value="source-folder">{t("settings.import.review.rule.sourceFolder")}</option>
                  <option value="current">{t("settings.import.review.rule.current")}</option>
                </select>
                <p className="text-xs text-muted-foreground">{t("settings.import.review.sourceRule")}</p>
              </div>
              <div className="grid gap-2">
                {mappingRule === "suggested" ? (
                  <div className="rounded-md border border-border bg-muted/35 p-3">
                    <p className="text-xs font-medium uppercase text-muted-foreground">{t("settings.import.review.resolvedTarget")}</p>
                    <div className="mt-2 flex flex-wrap items-center gap-2">
                      <Badge variant="status">{moduleLabel(acceptedTargetModule, t)}</Badge>
                      {recommendation ? <span className="text-xs text-muted-foreground">{recommendation}</span> : null}
                    </div>
                  </div>
                ) : (
                  <>
                    <Label htmlFor="import-target-module">{t("settings.import.review.mapToModule")}</Label>
                    <select
                      className="h-10 rounded-md border border-input bg-background px-3 text-sm"
                      id="import-target-module"
                      onChange={(event) => onTargetModuleChange(event.target.value)}
                      value={targetModule}
                    >
                      <option value="notes">{t("nav.notes")}</option>
                      <option value="todos">{t("nav.todos")}</option>
                      <option value="contacts">{t("nav.contacts")}</option>
                      <option value="habits">{t("nav.habits")}</option>
                    </select>
                    <p className="text-xs text-muted-foreground">
                      {t("settings.import.review.resolvedTarget")}: {moduleLabel(acceptedTargetModule, t)}. {recommendation}
                    </p>
                  </>
                )}
              </div>
              <div className="grid gap-2 rounded-md border border-border bg-muted/35 p-3">
                <p className="text-sm font-medium">{t("settings.import.review.schemaConflictChoice")}</p>
                {(["accept", "note", "edit", "ignore"] as const).map((choice) => (
                  <label className="flex items-start gap-2 text-sm" key={choice}>
                    <input
                      checked={schemaConflictChoice === choice}
                      className="mt-1"
                      name="import-schema-conflict-choice"
                      onChange={() => onSchemaConflictChoiceChange(choice)}
                      type="radio"
                    />
                    <span>
                      <span className="block font-medium">{importConflictChoiceLabel(choice, t)}</span>
                      <span className="block text-xs text-muted-foreground">{importConflictChoiceDescription(choice, t)}</span>
                    </span>
                  </label>
                ))}
              </div>
              <div className="grid gap-2">
                <Label htmlFor="import-target-filename">{t("settings.import.review.targetFilename")}</Label>
                <Input
                  id="import-target-filename"
                  onChange={(event) => onTargetFilenameChange(event.target.value)}
                  placeholder={selected.detected_title ?? "imported-file"}
                  value={targetFilename}
                />
                <p className="break-all text-xs text-muted-foreground">{t("settings.import.review.targetFilenamePreview")}: {targetPreview}</p>
              </div>
              <div className="grid gap-2">
                <Label htmlFor="import-tags">{t("settings.import.review.tags")}</Label>
                <Input
                  id="import-tags"
                  onChange={(event) => onTagsChange(event.target.value)}
                  placeholder="imported obsidian"
                  value={tags}
                />
              </div>
              <div className="rounded-md border border-dashed border-border bg-muted/35 p-3 text-sm">
                <p className="font-medium">{t("settings.import.review.markdownPreview")}</p>
                <p className="mt-2 text-muted-foreground">
                  {selected.detected_title ? `# ${selected.detected_title}` : selected.staged_file_path}
                </p>
                <p className="mt-1 text-xs text-muted-foreground">{t("settings.import.review.preserved")}</p>
              </div>
              <div className="flex flex-wrap gap-3">
                <Button disabled={busy !== null || selected.accepted || selected.ignored} onClick={onAccept} type="button">
                  <Upload data-icon="inline-start" />
                  {busy === "import-accept" ? t("settings.import.review.accepting") : schemaConflictChoice === "ignore" ? t("settings.import.review.ignoreFromChoices") : t("settings.import.review.acceptIntoModule")}
                </Button>
                <Button disabled={busy !== null || selected.accepted} onClick={onIgnore} type="button" variant="outline">
                  {busy === "import-ignore" ? t("settings.import.review.ignoring") : t("settings.import.review.choice.ignore")}
                </Button>
                <Button disabled type="button" variant="outline">
                  {t("settings.import.review.keepStaged")}
                </Button>
                {selected.accepted ? (
                  <Button onClick={onOpenAcceptedModule} type="button" variant="outline">
                    {t("settings.import.review.openAcceptedModule")}
                  </Button>
                ) : null}
              </div>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">{t("settings.import.review.placeholder")}</p>
          )}
        </div>
      </div>
    </section>
  );
}

function filterLabel(filter: "accepted" | "all" | "conflicts" | "ignored" | "unreviewed", t: (key: string) => string) {
  switch (filter) {
    case "all":
      return t("settings.import.review.all");
    case "unreviewed":
      return t("settings.import.review.unreviewed");
    case "accepted":
      return t("settings.import.review.accepted");
    case "ignored":
      return t("settings.import.review.ignored");
    case "conflicts":
      return t("settings.import.review.conflicts");
  }
}

function moduleLabel(moduleId: string, t: (key: string) => string) {
  switch (moduleId) {
    case "todos":
      return t("nav.todos");
    case "contacts":
      return t("nav.contacts");
    case "habits":
      return t("nav.habits");
    default:
      return t("nav.notes");
  }
}

function moduleForImportRecord(record: StagedImportRecord, selectedModule: string, rule: ImportMappingRule) {
  if (rule === "current") return canonicalImportModule(selectedModule);
  if (rule === "suggested") return canonicalImportModule(record.suggested_module);
  const source = `${record.original_source_path ?? ""}/${record.staged_file_path}`.toLowerCase();
  if (/(workplace|people|person|contact|contacts|client|vendor)/.test(source)) return "contacts";
  if (/(tasks?|todos?|checklist|inbox)/.test(source)) return "todos";
  if (/(habits?|routines?|streaks?)/.test(source)) return "habits";
  return "notes";
}

function canonicalImportModule(moduleId: string) {
  return ["todos", "contacts", "habits", "notes"].includes(moduleId) ? moduleId : "notes";
}

function mappingRuleLabel(rule: ImportMappingRule, t: (key: string) => string) {
  switch (rule) {
    case "source-folder":
      return t("settings.import.review.rule.sourceFolder.short");
    case "current":
      return t("settings.import.review.rule.current.short");
    default:
      return t("settings.import.review.rule.suggested.short");
  }
}

function sourceKindLabel(sourceKind: string, t: (key: string) => string) {
  switch (sourceKind) {
    case "bentolife_vault":
      return t("settings.import.review.sourceKind.bentolifeVault");
    case "snapshot":
      return t("settings.import.review.sourceKind.snapshot");
    case "obsidian_markdown_folder":
      return t("settings.import.review.sourceKind.obsidian");
    case "markdown_folder":
      return t("settings.import.review.sourceKind.markdownFolder");
    default:
      return sourceKind.replace(/_/g, " ");
  }
}

function importReviewRecommendation(record: StagedImportRecord, selectedModule: string, rule: ImportMappingRule, t: (key: string) => string) {
  if (record.conflict_status?.toLowerCase().includes("enum")) {
    return t("settings.import.review.choice.edit.description");
  }
  const resolved = moduleForImportRecord(record, selectedModule, rule);
  if (record.conflict_status || record.detected_links.length > 8) {
    return t("settings.import.review.choice.note.description");
  }
  if (resolved === record.suggested_module || record.detected_checklists > 0 || record.detected_tags.length > 0) {
    return `${t("settings.import.review.choice.accept.description")} (${moduleLabel(resolved, t)}).`;
  }
  return t("settings.import.review.sourceRule");
}

function importConflictChoiceLabel(choice: ImportConflictChoice, t: (key: string) => string) {
  switch (choice) {
    case "note":
      return t("settings.import.review.choice.note");
    case "edit":
      return t("settings.import.review.choice.edit");
    case "ignore":
      return t("settings.import.review.choice.ignore");
    default:
      return t("settings.import.review.choice.accept");
  }
}

function importConflictChoiceDescription(choice: ImportConflictChoice, t: (key: string) => string) {
  switch (choice) {
    case "note":
      return t("settings.import.review.choice.note.description");
    case "edit":
      return t("settings.import.review.choice.edit.description");
    case "ignore":
      return t("settings.import.review.choice.ignore.description");
    default:
      return t("settings.import.review.choice.accept.description");
  }
}

function FolderImportSummary({ preview }: { preview: FolderImportPreview }) {
  const { t } = useI18n();
  const visibleFiles = preview.planned_files.slice(0, 4);
  return (
    <div className="rounded-md border border-border bg-muted/55 p-3 text-sm">
      <Badge variant={preview.conflicts.length ? "outline" : "status"}>{t("settings.import.folder.preview")}</Badge>
      <div className="mt-3 grid gap-2 sm:grid-cols-3">
        <SummaryRow label={t("settings.import.folder.markdown")} value={`${preview.scan.markdown_count}`} />
        <SummaryRow label={t("settings.import.folder.assets")} value={`${preview.scan.asset_count}`} />
        <SummaryRow label={t("settings.import.folder.ignored")} value={`${preview.scan.ignored_count}`} />
      </div>
      <p className="mt-3 break-all text-xs text-muted-foreground">{t("settings.import.folder.targetRoot")}: {preview.target_root}</p>
      {preview.conflicts.length ? <p className="mt-2 text-xs text-muted-foreground">{t("settings.import.folder.conflicts")}: {preview.conflicts.join("; ")}</p> : null}
      <div className="mt-3 flex flex-col gap-2">
        {visibleFiles.map((file) => (
          <div className="rounded-md bg-background px-3 py-2" key={`${file.source_relative_path}-${file.target_relative_path}`}>
            <p className="font-medium">{file.source_relative_path}</p>
            <p className="break-all text-xs text-muted-foreground">{file.target_relative_path}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function RepairNotice({ message, title }: { message: string; title: string }) {
  return (
    <div className="flex gap-3 rounded-md border border-border bg-muted/55 p-4 text-sm">
      <AlertTriangle aria-hidden="true" className="mt-0.5 shrink-0 text-amber-note-foreground" />
      <div className="min-w-0">
        <p className="font-medium">{title}</p>
        <p className="mt-1 text-muted-foreground">{message}</p>
      </div>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3 rounded-md border border-border bg-background px-3 py-2">
      <span className="truncate text-muted-foreground">{label}</span>
      <span className="truncate font-medium">{value}</span>
    </div>
  );
}

export function SnapshotPathDetails({ label, paths }: { label: string; paths?: string[] }) {
  const visiblePaths = paths ?? [];
  if (!visiblePaths.length) {
    return null;
  }
  return (
    <details className="rounded-md border border-border bg-background px-3 py-2 text-xs [&:not([open])>*:not(summary)]:hidden">
      <summary className="cursor-pointer font-semibold text-foreground">
        {label} ({visiblePaths.length})
      </summary>
      <ul className="mt-2 max-h-40 space-y-1 overflow-auto text-muted-foreground">
        {visiblePaths.map((path) => (
          <li className="break-all font-mono" key={path}>{path}</li>
        ))}
      </ul>
    </details>
  );
}

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
