import { Archive, Copy, MoreHorizontal, Pin, PinOff, Plus, Save, Search, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { MarkdownEditor } from "@/components/modules/MarkdownEditor";
import {
  FocusSurfaceHeader,
  ModuleBrowsePanel,
  ModuleEmptyState,
  OperationMessage,
} from "@/components/modules/shared/ModuleSurface";
import { Button } from "@/components/ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Empty } from "@/components/ui/empty";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useToast } from "@/components/ui/toast";
import { useI18n } from "@/i18n";
import { archiveManagedEntity, trashManagedEntity } from "@/services/backendCore";
import type { NoteDocument, NoteSummary } from "@/services/notes";

type ModuleBacklink = { source_path: string; target: string; link_type: string; status: string; raw: string };

export type NotesPanelProps = {
  backlinks?: ModuleBacklink[];
  loading: boolean;
  notes: NoteSummary[];
  onCreateNote: (title?: string, markdownBody?: string) => Promise<NoteDocument | null>;
  onPasteNoteImage: (documentId: string, file: File) => Promise<string>;
  onRenameNote: (documentId: string, newTitle: string) => Promise<void>;
  onRefreshWorkspace: () => Promise<void> | void;
  onSaveNote: (
    documentId: string,
    markdownBody: string,
    expectedContentHash?: string | null,
    overwriteConflict?: boolean,
  ) => Promise<NoteDocument>;
  onSelectNote: (documentId: string) => Promise<NoteDocument | null>;
  onToggleNotePin: (documentId: string, pinned: boolean) => Promise<void>;
  pinnedNoteIds: string[];
  selectedNote: NoteDocument | null;
  selectedNotePinned: boolean;
  vaultPath: string | null;
  workspaceError: string | null;
};

export function NotesPanel({
  backlinks = [],
  loading,
  notes,
  onCreateNote,
  onPasteNoteImage,
  onRenameNote,
  onRefreshWorkspace,
  onSaveNote,
  onSelectNote,
  onToggleNotePin,
  pinnedNoteIds,
  selectedNote,
  selectedNotePinned,
  vaultPath,
  workspaceError,
}: NotesPanelProps) {
  const { t } = useI18n();
  const { showToast } = useToast();
  const titleEditorRef = useRef<HTMLDivElement | null>(null);
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const [query, setQuery] = useState("");
  const [draftTitle, setDraftTitle] = useState("");
  const [draftMarkdown, setDraftMarkdown] = useState("");
  const [renaming, setRenaming] = useState(false);
  const [renameSaving, setRenameSaving] = useState(false);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [pinSaving, setPinSaving] = useState(false);
  const [lifecycleSaving, setLifecycleSaving] = useState<string | null>(null);
  const [openNoteMenuId, setOpenNoteMenuId] = useState<string | null>(null);

  useEffect(() => {
    setDraftTitle(selectedNote?.title ?? "");
    setDraftMarkdown(selectedNote?.markdown_body ?? "");
    setRenaming(false);
    setRenameError(null);
  }, [selectedNote]);

  const filteredNotes = notes.filter((note) =>
    `${note.title} ${note.markdown_relative_path} ${note.excerpt}`.toLowerCase().includes(query.toLowerCase()),
  );

  function beginRename(note = selectedNote) {
    setDraftTitle(note?.title ?? "");
    setRenameError(null);
    setRenaming(true);
    requestAnimationFrame(() => {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    });
  }

  function cancelRename() {
    setDraftTitle(selectedNote?.title ?? "");
    setRenameError(null);
    setRenaming(false);
  }

  async function saveRename(note = selectedNote) {
    if (!note) return;
    const nextTitle = draftTitle.trim();
    if (!nextTitle) {
      setRenameError(t("modules.notes.titleRequired"));
      requestAnimationFrame(() => renameInputRef.current?.focus());
      return;
    }
    if (nextTitle === note.title) {
      setRenaming(false);
      setRenameError(null);
      return;
    }
    setRenameSaving(true);
    setRenameError(null);
    try {
      await onRenameNote(note.document_id, nextTitle);
      setRenaming(false);
      showToast({ kind: "success", message: t("modules.notes.renamed"), title: t("toast.updated") });
    } catch (error) {
      const message = getErrorMessage(error);
      setRenameError(message);
      showToast({ kind: "error", message, title: t("modules.notes.renameFailed") });
    } finally {
      setRenameSaving(false);
    }
  }

  function handleTitleBlur() {
    window.setTimeout(() => {
      if (titleEditorRef.current?.contains(document.activeElement)) return;
      if (!renaming || renameSaving) return;
      void saveRename();
    }, 0);
  }

  async function startRenameForNote(note: NoteSummary) {
    setOpenNoteMenuId(null);
    const selected = selectedNote?.document_id === note.document_id ? selectedNote : await onSelectNote(note.document_id);
    window.setTimeout(() => {
      beginRename(selected ?? ({ title: note.title } as NoteDocument));
    }, 0);
  }

  async function togglePin(documentId: string, pinned: boolean) {
    setPinSaving(true);
    try {
      await onToggleNotePin(documentId, pinned);
      showToast({
        kind: "success",
        message: pinned ? t("modules.notes.unpinned") : t("modules.notes.pinned"),
        title: t("toast.updated"),
      });
    } catch (error) {
      const message = getErrorMessage(error);
      showToast({ kind: "error", message, title: pinned ? t("modules.notes.unpinFailed") : t("modules.notes.pinFailed") });
    } finally {
      setPinSaving(false);
      setOpenNoteMenuId(null);
    }
  }

  async function copyNotePath(path: string) {
    await navigator.clipboard?.writeText(path);
    showToast({ kind: "success", message: t("modules.notes.pathCopied"), title: t("toast.updated") });
    setOpenNoteMenuId(null);
  }

  async function archiveNote(note: NoteSummary) {
    if (!vaultPath) return;
    setLifecycleSaving(`archive:${note.document_id}`);
    try {
      await archiveManagedEntity(vaultPath, note.markdown_relative_path);
      await onRefreshWorkspace();
      showToast({ kind: "success", message: t("modules.notes.archived"), title: t("toast.updated") });
    } catch (error) {
      const message = getErrorMessage(error);
      showToast({ kind: "error", message, title: t("modules.notes.archiveFailed") });
    } finally {
      setLifecycleSaving(null);
      setOpenNoteMenuId(null);
    }
  }

  async function trashNote(note: NoteSummary) {
    if (!vaultPath) return;
    setLifecycleSaving(`trash:${note.document_id}`);
    try {
      await trashManagedEntity(vaultPath, note.markdown_relative_path);
      await onRefreshWorkspace();
      showToast({ kind: "success", message: t("modules.notes.trashed"), title: t("toast.updated") });
    } catch (error) {
      const message = getErrorMessage(error);
      showToast({ kind: "error", message, title: t("modules.notes.trashFailed") });
    } finally {
      setLifecycleSaving(null);
      setOpenNoteMenuId(null);
    }
  }

  return (
    <div className="grid gap-5 xl:grid-cols-[18rem_minmax(0,1fr)]">
      <ModuleBrowsePanel>
        <div className="flex gap-2">
          <div className="relative min-w-0 flex-1">
            <Search aria-hidden="true" className="pointer-events-none absolute left-3 top-3 text-muted-foreground" data-icon="inline-start" />
            <Input
              aria-label={t("modules.notes.search")}
              className="pl-9"
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("modules.notes.search")}
              value={query}
            />
          </div>
          <Button aria-label={t("modules.notes.create")} onClick={() => void onCreateNote(t("modules.notes.untitled"))} size="icon">
            <Plus aria-hidden="true" />
          </Button>
        </div>

        <div className="flex max-h-[30rem] min-w-0 flex-col gap-2 overflow-auto pr-1">
          {filteredNotes.map((note) => {
            const pinned = pinnedNoteIds.includes(note.document_id);
            return (
            <div
              className="flex min-w-0 items-start gap-2 rounded-md border border-border bg-background p-2 transition-colors hover:bg-accent"
              key={note.document_id}
              onContextMenu={(event) => {
                event.preventDefault();
                setOpenNoteMenuId(note.document_id);
              }}
            >
              <button
                className="min-w-0 flex-1 rounded-sm p-1 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                onClick={() => void onSelectNote(note.document_id)}
                type="button"
              >
                <span className="flex min-w-0 items-center gap-2">
                  {pinned ? <Pin aria-hidden="true" className="size-3.5 shrink-0 fill-current text-primary" /> : null}
                  <span className="truncate text-sm font-medium">{note.title}</span>
                </span>
                <span className="mt-1 block truncate text-xs text-muted-foreground">{note.markdown_relative_path}</span>
                <span className="mt-2 line-clamp-2 text-xs leading-5 text-muted-foreground">{note.excerpt}</span>
              </button>
              <NoteRowMenu
                note={note}
                onCopyPath={() => void copyNotePath(note.markdown_relative_path)}
                onArchive={() => void archiveNote(note)}
                onOpenChange={(open) => setOpenNoteMenuId(open ? note.document_id : null)}
                onPin={() => void togglePin(note.document_id, pinned)}
                onRename={() => void startRenameForNote(note)}
                onTrash={() => void trashNote(note)}
                open={openNoteMenuId === note.document_id}
                pinned={pinned}
                lifecycleBusy={lifecycleSaving !== null}
                lifecycleDisabled={!vaultPath}
              />
            </div>
            );
          })}
          {!filteredNotes.length ? (
            <Empty
              className="min-h-48"
              title={notes.length ? t("modules.notes.noMatching") : t("modules.notes.noneYet")}
              description={notes.length ? t("modules.notes.tryDifferentSearch") : t("modules.notes.createDescription")}
            />
          ) : null}
        </div>
      </ModuleBrowsePanel>

      <div className="min-w-0">
        {workspaceError ? <OperationMessage title={t("modules.notes.loadFailed")} message={workspaceError} /> : null}
        {loading ? <Skeleton className="h-96" /> : null}
        {!loading && selectedNote ? (
          <div className="flex min-w-0 flex-col gap-4">
            <FocusSurfaceHeader>
              {renaming ? (
                <div className="flex min-w-0 flex-1 flex-col gap-2" ref={titleEditorRef}>
                  <Label htmlFor="note-title">{t("modules.notes.title")}</Label>
                  <div className="flex flex-wrap items-start gap-2">
                    <Input
                      aria-describedby={renameError ? "note-title-error" : undefined}
                      id="note-title"
                      onChange={(event) => {
                        setDraftTitle(event.target.value);
                        if (event.target.value.trim()) setRenameError(null);
                      }}
                      onKeyDown={(event) => {
                        if (event.key === "Escape") {
                          event.preventDefault();
                          cancelRename();
                        }
                        if (event.key === "Enter" || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s")) {
                          event.preventDefault();
                          void saveRename();
                        }
                      }}
                      onBlur={handleTitleBlur}
                      ref={renameInputRef}
                      value={draftTitle}
                    />
                    <Button disabled={renameSaving || !draftTitle.trim()} onClick={() => void saveRename()} onMouseDown={(event) => event.preventDefault()}>
                      <Save data-icon="inline-start" />
                      {renameSaving ? t("modules.editor.saving") : t("app.actions.save")}
                    </Button>
                    <Button disabled={renameSaving} onClick={cancelRename} onMouseDown={(event) => event.preventDefault()} type="button" variant="outline">
                      <X data-icon="inline-start" />
                      {t("app.actions.cancel")}
                    </Button>
                  </div>
                  {renameError ? <p className="text-sm text-destructive" id="note-title-error">{renameError}</p> : null}
                  {!draftTitle.trim() && !renameError ? <p className="text-sm text-muted-foreground">{t("modules.notes.titleRequired")}</p> : null}
                </div>
              ) : (
                <>
                  <div className="min-w-0 flex-1">
                    <button
                      className="block max-w-full truncate rounded-sm text-left text-2xl font-semibold leading-tight text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      onClick={() => beginRename()}
                      type="button"
                    >
                      {selectedNote.title}
                    </button>
                    <p className="mt-1 truncate text-sm text-muted-foreground">{selectedNote.markdown_relative_path}</p>
                  </div>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        aria-label={selectedNotePinned ? t("modules.notes.unpin") : t("modules.notes.pin")}
                        className="self-start"
                        disabled={pinSaving}
                        onClick={() => void togglePin(selectedNote.document_id, selectedNotePinned)}
                        size="icon"
                        variant={selectedNotePinned ? "default" : "outline"}
                      >
                        {selectedNotePinned ? <Pin aria-hidden="true" className="fill-current" /> : <Pin aria-hidden="true" />}
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent>{selectedNotePinned ? t("modules.notes.unpin") : t("modules.notes.pin")}</TooltipContent>
                  </Tooltip>
                </>
              )}
            </FocusSurfaceHeader>
            <MarkdownEditor
              backlinks={backlinks}
              documentId={selectedNote.document_id}
              expectedContentHash={selectedNote.parsed_entity.content_hash}
              markdownPath={selectedNote.markdown_relative_path}
              onChange={setDraftMarkdown}
              onPasteImage={(file) => onPasteNoteImage(selectedNote.document_id, file)}
              onReloadLatest={() => {
                void onSelectNote(selectedNote.document_id);
              }}
              onSave={async (overwriteConflict = false) => {
                try {
                  const saved = await onSaveNote(
                    selectedNote.document_id,
                    draftMarkdown,
                    selectedNote.parsed_entity.content_hash,
                    overwriteConflict,
                  );
                  setDraftMarkdown(saved.markdown_body);
                  return { ok: true };
                } catch (error) {
                  const message = getErrorMessage(error);
                  return { ok: false, conflict: isStaleWriteMessage(message), message };
                }
              }}
              onSaveAsCopy={() => {
                void onCreateNote(`${selectedNote.title} ${t("modules.notes.copySuffix")}`, draftMarkdown);
              }}
              title={selectedNote.title}
              value={draftMarkdown}
              vaultPath={vaultPath}
            />
          </div>
        ) : null}
        {!loading && !selectedNote ? (
          <ModuleEmptyState title={t("modules.notes.selectOrCreate")} description={t("modules.notes.storageDescription")} />
        ) : null}
      </div>
    </div>
  );
}

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function isStaleWriteMessage(message: string) {
  const normalized = message.toLowerCase();
  return normalized.includes("changed outside bentolife") || normalized.includes("stale") || normalized.includes("conflict");
}

function NoteRowMenu({
  note,
  onCopyPath,
  onArchive,
  onOpenChange,
  onPin,
  onRename,
  onTrash,
  open,
  pinned,
  lifecycleBusy,
  lifecycleDisabled,
}: {
  note: NoteSummary;
  onCopyPath: () => void;
  onArchive: () => void;
  onOpenChange: (open: boolean) => void;
  onPin: () => void;
  onRename: () => void;
  onTrash: () => void;
  open: boolean;
  pinned: boolean;
  lifecycleBusy: boolean;
  lifecycleDisabled: boolean;
}) {
  const { t } = useI18n();
  return (
    <DropdownMenu open={open} onOpenChange={onOpenChange}>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuTrigger asChild>
            <Button aria-label={`${t("modules.notes.actions")} ${note.title}`} size="icon" variant="ghost">
              <MoreHorizontal aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent>{t("modules.notes.actions")}</TooltipContent>
      </Tooltip>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={onPin}>
          {pinned ? <PinOff aria-hidden="true" className="size-4" /> : <Pin aria-hidden="true" className="size-4" />}
          {pinned ? t("modules.notes.unpin") : t("modules.notes.pin")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={onRename}>
          <Save aria-hidden="true" className="size-4" />
          {t("modules.notes.rename")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={onCopyPath}>
          <Copy aria-hidden="true" className="size-4" />
          {t("modules.notes.copyPath")}
        </DropdownMenuItem>
        <div className="my-1 h-px bg-border" role="separator" />
        <DropdownMenuItem disabled={lifecycleBusy || lifecycleDisabled} onSelect={onArchive}>
          <Archive aria-hidden="true" className="size-4" />
          {t("modules.notes.archive")}
        </DropdownMenuItem>
        <DropdownMenuItem disabled={lifecycleBusy || lifecycleDisabled} onSelect={onTrash}>
          <Trash2 aria-hidden="true" className="size-4" />
          {t("modules.notes.moveToTrash")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
