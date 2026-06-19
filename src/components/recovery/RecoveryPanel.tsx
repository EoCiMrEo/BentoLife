import { AlertTriangle, RefreshCw, RotateCcw } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useI18n } from "@/i18n";
import type { RecoveryIssue, WorkspaceRecoveryPreview } from "@/services/notes";

export type RecoveryPanelProps = {
  onResetWidgets: () => void;
  onRunRecovery: (issue: RecoveryIssue) => void;
  recoveryPreview: WorkspaceRecoveryPreview | null;
};

export function RecoveryPanel({ onResetWidgets, onRunRecovery, recoveryPreview }: RecoveryPanelProps) {
  const { t } = useI18n();
  const plainMarkdownDocs =
    recoveryPreview?.scan?.documents.filter((document) => document.status === "plain_markdown") ?? [];
  const duplicateDocs =
    recoveryPreview?.scan?.documents.filter((document) => document.status === "duplicate_identity") ?? [];
  const hasRecoveryPrompts = Boolean(recoveryPreview?.issues.length || plainMarkdownDocs.length || duplicateDocs.length);

  return (
    <div className="grid gap-4 text-sm">
      <Card className="shadow-none">
        <CardHeader>
          <CardTitle className="text-base">{t("recovery.title")}</CardTitle>
          <CardDescription>{t("recovery.description")}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3">
          {recoveryPreview?.issues.map((issue) => (
            <div className="rounded-md border border-border bg-background p-3" key={`${issue.code}-${issue.document_id}-${issue.markdown_relative_path}`}>
              <p className="text-sm font-medium">{recoveryTitle(issue.code, t)}</p>
              <p className="mt-1 text-sm text-muted-foreground">{issue.message}</p>
              <Button className="mt-3" disabled={!issue.action} onClick={() => onRunRecovery(issue)} size="sm" variant="outline">
                <RefreshCw data-icon="inline-start" />
                {issue.action ? recoveryActionLabel(issue.action, t) : t("recovery.manualReview")}
              </Button>
            </div>
          ))}
          {plainMarkdownDocs.map((document) => (
            <div className="rounded-md border border-border bg-background p-3" key={document.markdown_relative_path}>
              <p className="text-sm font-medium">{t("recovery.plainMarkdownPrompt")}</p>
              <p className="mt-1 text-sm text-muted-foreground">
                {document.markdown_relative_path} {t("recovery.plainMarkdownDescription")}
              </p>
              <Button
                className="mt-3"
                disabled={!document.markdown_relative_path}
                onClick={() =>
                  onRunRecovery({
                    action: "recover_document_metadata",
                    code: "plain_markdown",
                    document_id: document.document_id,
                    markdown_relative_path: document.markdown_relative_path,
                    message: "Plain Markdown can be managed by BentoLife.",
                  })
                }
                size="sm"
                variant="outline"
              >
                <RefreshCw data-icon="inline-start" />
                {t("recovery.addMetadata")}
              </Button>
            </div>
          ))}
          {duplicateDocs.map((document) => (
            <RecoveryWarning
              key={document.markdown_relative_path}
              title={t("recovery.duplicateUuid")}
              message={`${document.markdown_relative_path} shares a document identity with another file. Review before repair so Markdown content stays safe.`}
            />
          ))}
          {!hasRecoveryPrompts ? (
            <p className="text-sm text-muted-foreground">{t("recovery.noPrompts")}</p>
          ) : null}
        </CardContent>
      </Card>

      <Card className="shadow-none">
        <CardHeader>
          <CardTitle className="text-base">{t("recovery.widgetMetadata.title")}</CardTitle>
          <CardDescription>{t("recovery.widgetMetadata.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <Button className="w-fit" onClick={onResetWidgets} variant="outline">
            <RotateCcw data-icon="inline-start" />
            {t("recovery.widgetMetadata.reset")}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}

function RecoveryWarning({ message, title }: { message: string; title: string }) {
  return (
    <div className="rounded-md border border-amber-note/50 bg-amber-note/20 p-3 text-sm text-amber-note-foreground">
      <div className="flex items-start gap-2">
        <AlertTriangle className="mt-0.5 size-4" aria-hidden="true" />
        <div>
          <p className="font-semibold">{title}</p>
          <p className="mt-1">{message}</p>
        </div>
      </div>
    </div>
  );
}

function recoveryTitle(code: string, t: (key: string) => string) {
  switch (code) {
    case "layout_missing":
      return t("recovery.title.layoutMissing");
    case "metadata_missing":
    case "plain_markdown":
      return t("recovery.title.metadataMissing");
    case "metadata_path_mismatch":
      return t("recovery.title.pathMismatch");
    case "markdown_missing":
      return t("recovery.title.markdownMissing");
    case "duplicate_identity":
      return t("recovery.title.duplicateIdentity");
    default:
      return t("recovery.title.generic");
  }
}

function recoveryActionLabel(action: string, t: (key: string) => string) {
  switch (action) {
    case "recover_document_metadata":
      return t("recovery.action.documentMetadata");
    case "recover_layout_metadata":
      return t("recovery.action.layoutMetadata");
    case "orphan_missing_document_metadata":
      return t("recovery.action.preserveMissing");
    case "restore_orphaned_document_metadata":
      return t("recovery.action.restoreOrphan");
    case "repair_document_frontmatter_reference":
      return t("recovery.action.repairFrontmatter");
    default:
      return t("recovery.action.run");
  }
}
