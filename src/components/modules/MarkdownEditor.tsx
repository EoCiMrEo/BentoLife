import React, { useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangle, Columns2, Eye, FilePlus2, Pencil, RefreshCw, Save } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useToast } from "@/components/ui/toast";
import { useI18n } from "@/i18n";
import { parseMarkdownPreview } from "@/services/markdownPreview";
import { renderFieldValue } from "@/services/rendererRegistry";

type SaveResult = { ok: true } | { ok: false; conflict?: boolean; message: string };
type MarkdownBacklink = { source_path: string; target: string; link_type: string; status: string; raw: string };
type MarkdownPresentationMode = "view" | "edit" | "split";

export type MarkdownEditorProps = {
  backlinks?: MarkdownBacklink[];
  documentId: string;
  title: string;
  markdownPath: string;
  vaultPath: string | null;
  value: string;
  expectedContentHash: string;
  onChange: (value: string) => void;
  onReloadLatest: () => void;
  onSave: (overwriteConflict?: boolean) => Promise<SaveResult>;
  onSaveAsCopy: () => void;
  onPasteImage: (file: File) => Promise<string>;
};

export function MarkdownEditor({
  backlinks = [],
  documentId,
  title,
  markdownPath,
  vaultPath,
  value,
  expectedContentHash,
  onChange,
  onReloadLatest,
  onSave,
  onSaveAsCopy,
  onPasteImage,
}: MarkdownEditorProps) {
  const { t } = useI18n();
  const { showToast } = useToast();
  const editorRef = useRef<HTMLTextAreaElement | null>(null);
  const [saving, setSaving] = useState(false);
  const [conflict, setConflict] = useState<string | null>(null);
  const [pasteMessage, setPasteMessage] = useState<string | null>(null);
  const [mode, setMode] = useState<MarkdownPresentationMode>("view");
  const blocks = useMemo(() => parseMarkdownPreview(value), [value]);
  const renderContext = { documentId, moduleId: "notes", vaultPath, markdownPath };

  useEffect(() => {
    setMode("view");
    setConflict(null);
    setPasteMessage(null);
  }, [documentId]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "s") return;
      const target = event.target as HTMLElement | null;
      if (!(target instanceof HTMLTextAreaElement) || target.dataset.markdownEditor !== documentId) return;
      event.preventDefault();
      void save(false);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [value, expectedContentHash]);

  async function save(overwriteConflict: boolean) {
    setSaving(true);
    setConflict(null);
    const result = await onSave(overwriteConflict);
    setSaving(false);
    if (result.ok) {
      showToast({ kind: "success", message: t("modules.editor.saved"), title: t("toast.updated") });
      setMode("view");
      return;
    }
    if (result.conflict) {
      setConflict(result.message);
      return;
    }
    showToast({ kind: "error", message: result.message, title: t("toast.actionFailed") });
  }

  async function handlePaste(event: React.ClipboardEvent<HTMLTextAreaElement>) {
    const imageFiles = Array.from(event.clipboardData.files).filter((item) => item.type.startsWith("image/"));
    const file = imageFiles.find((item) => isAllowedClipboardImage(item.type));
    if (!file && imageFiles.length) {
      event.preventDefault();
      setPasteMessage(t("modules.editor.pasteImageTypes"));
      return;
    }
    if (!file) return;
    event.preventDefault();
    setPasteMessage(null);
    try {
      const markdownLink = await onPasteImage(file);
      insertAtCursor(`![${t("modules.editor.pastedImageAlt")}](${markdownLink})`, event.currentTarget);
      showToast({ kind: "success", message: t("modules.editor.imagePasted"), title: t("toast.updated") });
    } catch (error) {
      setPasteMessage(error instanceof Error ? error.message : t("modules.editor.imagePasteFailed"));
    }
  }

  function insertAtCursor(text: string, activeEditor?: HTMLTextAreaElement | null) {
    const editor = activeEditor ?? editorRef.current;
    if (!editor) {
      onChange(`${value.trimEnd()}\n\n${text}\n`);
      return;
    }
    const start = editor.selectionStart;
    const end = editor.selectionEnd;
    const before = value.slice(0, start);
    const after = value.slice(end);
    const prefix = before.endsWith("\n") || before.length === 0 ? "" : "\n";
    const suffix = after.startsWith("\n") || after.length === 0 ? "" : "\n";
    const next = `${before}${prefix}${text}${suffix}${after}`;
    onChange(next);
    requestAnimationFrame(() => {
      editor.focus();
      const cursor = before.length + prefix.length + text.length;
      editor.setSelectionRange(cursor, cursor);
    });
  }

  function enterMode(nextMode: MarkdownPresentationMode) {
    setMode(nextMode);
    if (nextMode === "edit" || nextMode === "split") {
      requestAnimationFrame(() => editorRef.current?.focus());
    }
  }

  const editor = (
    <div className="flex min-w-0 flex-col gap-2">
      <Label htmlFor={`markdown-editor-${documentId}`}>{t("modules.editor.markdownSource")}</Label>
      <textarea
        aria-label={t("modules.editor.markdownSource")}
        className="min-h-[28rem] w-full resize-y rounded-md border border-input bg-background px-3 py-3 font-mono text-sm leading-6 outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
        data-markdown-editor={documentId}
        id={`markdown-editor-${documentId}`}
        onChange={(event) => onChange(event.target.value)}
        onFocus={(event) => {
          editorRef.current = event.currentTarget;
        }}
        onPaste={handlePaste}
        ref={editorRef}
        spellCheck
        value={value}
      />
    </div>
  );

  const renderPreview = (showLabel: boolean) => (
    <div className="min-h-[28rem] rounded-md border border-border bg-card p-4">
      {showLabel ? (
        <div className="mb-3 flex items-center gap-2 text-xs font-semibold uppercase text-muted-foreground">
          <Eye className="size-4" />
          {t("modules.editor.preview")}
        </div>
      ) : null}
      {renderFieldValue("markdown", blocks, "markdown-editor-preview", undefined, renderContext)}
    </div>
  );

  return (
    <section className="rounded-md border border-border bg-card p-5 shadow-soft">
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold uppercase text-muted-foreground">{t("modules.editor.markdown")}</h2>
          <p className="mt-1 truncate text-sm font-medium">{title}</p>
          <p className="mt-1 truncate text-xs text-muted-foreground">{markdownPath}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          {mode === "view" ? (
            <Button onClick={() => enterMode("edit")} variant="outline">
              <Pencil data-icon="inline-start" />
              {t("app.actions.edit")}
            </Button>
          ) : (
            <>
              <Button disabled={saving} onClick={() => void save(false)}>
                <Save data-icon="inline-start" />
                {saving ? t("modules.editor.saving") : t("app.actions.save")}
              </Button>
              <Button onClick={() => setMode("view")} type="button" variant="outline">
                <Eye data-icon="inline-start" />
                {t("modules.editor.done")}
              </Button>
            </>
          )}
          {mode !== "split" ? (
            <Button onClick={() => enterMode("split")} type="button" variant="ghost">
              <Columns2 data-icon="inline-start" />
              {t("modules.editor.split")}
            </Button>
          ) : (
            <Button onClick={() => setMode("view")} type="button" variant="ghost">
              <Eye data-icon="inline-start" />
              {t("modules.editor.view")}
            </Button>
          )}
        </div>
      </div>

      {mode === "view" ? (
        <div
          aria-label={t("modules.editor.editMarkdownContent")}
          className="cursor-text focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
          onClick={() => enterMode("edit")}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              enterMode("edit");
            }
          }}
          role="button"
          tabIndex={0}
        >
          {renderPreview(false)}
        </div>
      ) : mode === "split" ? (
        <div className="grid gap-4 lg:grid-cols-2">
          {editor}
          {renderPreview(true)}
        </div>
      ) : (
        editor
      )}

      {pasteMessage ? <p className="mt-3 text-sm text-muted-foreground">{pasteMessage}</p> : null}

      {conflict ? (
        <div className="mt-4 rounded-md border border-amber-note/50 bg-amber-note/20 p-4 text-sm text-amber-note-foreground">
          <div className="flex items-start gap-2">
            <AlertTriangle className="mt-0.5 size-4" />
            <div>
              <p className="font-semibold">{t("modules.editor.externalEdit")}</p>
              <p className="mt-1">{conflict}</p>
            </div>
          </div>
          <div className="mt-3 flex flex-wrap gap-2">
            <Button onClick={onReloadLatest} type="button" variant="outline">
              <RefreshCw data-icon="inline-start" />
              {t("modules.editor.reloadLatest")}
            </Button>
            <Button onClick={onSaveAsCopy} type="button" variant="outline">
              <FilePlus2 data-icon="inline-start" />
              {t("modules.editor.saveAsCopy")}
            </Button>
            <Button onClick={() => void save(true)} type="button" variant="outline">
              {t("modules.editor.overwriteAnyway")}
            </Button>
            <Button onClick={() => setConflict(null)} type="button" variant="ghost">
              {t("app.actions.cancel")}
            </Button>
          </div>
        </div>
      ) : null}

      <details className="mt-4 rounded-md border border-border bg-muted p-3 text-sm text-muted-foreground">
        <summary className="cursor-pointer font-semibold text-foreground">{t("modules.editor.advancedRaw")}</summary>
        <div className="mt-3 rounded-md border border-border bg-background p-3">
          <p className="text-xs font-semibold uppercase text-muted-foreground">{t("modules.editor.backlinks")}</p>
          {backlinks.length ? (
            <div className="mt-2 grid gap-2">
              {backlinks.map((link) => (
                <div className="rounded-md border border-border bg-muted/50 p-2" key={`${link.source_path}-${link.raw}`}>
                  <p className="break-all text-sm font-medium text-foreground">{link.source_path}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{link.link_type} - {link.status}</p>
                </div>
              ))}
            </div>
          ) : (
            <p className="mt-2 text-sm text-muted-foreground">{t("modules.editor.noBacklinksNote")}</p>
          )}
        </div>
        <p className="mt-2 leading-5">
          {t("modules.editor.rawReadOnlyDescription")}
        </p>
        <pre className="mt-3 max-h-72 overflow-auto whitespace-pre-wrap break-words rounded-md bg-background p-3 text-xs">
          {value}
        </pre>
      </details>
    </section>
  );
}

function isAllowedClipboardImage(mimeType: string) {
  return mimeType === "image/png" || mimeType === "image/jpeg" || mimeType === "image/webp";
}
