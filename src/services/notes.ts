import { invoke } from "@tauri-apps/api/core";

import {
  normalizeNoteDocument,
  normalizeNoteSummaries,
} from "@/services/contracts/notes.contract";
import {
  emptyWorkspaceRecoveryPreview,
  emptyWorkspaceScanResult,
  normalizeWorkspaceRecoveryPreview,
  normalizeWorkspaceScanResult,
} from "@/services/contracts/workspace.contract";
import { safeInvoke } from "@/services/contracts/invokeClient";
import type { ParsedEntityContract } from "@/services/backendCore";
import type { IndexSnapshot } from "@/types/bentolife-core";
import { isTauriRuntime } from "@/services/vault";

export type ScannedDocumentStatus =
  | "managed"
  | "plain_markdown"
  | "metadata_missing"
  | "layout_missing"
  | "metadata_path_mismatch"
  | "duplicate_identity";

export type LayoutCardMetadata = {
  section_match: string;
  card_id: string;
  width: "single" | "double" | "full" | string;
  order: number;
  widget: string;
};

export type LayoutMetadata = {
  schema_version: number;
  document_id: string;
  vault_relative: boolean;
  layout_mode: string;
  theme: string;
  cards: LayoutCardMetadata[];
  fallback_layout: {
    strategy: string;
    default_width: "single" | "double" | "full" | string;
    preserve_markdown_order: boolean;
  };
  updated_at: string;
};

export type ScannedDocument = {
  document_id: string | null;
  title: string;
  markdown_relative_path: string;
  metadata_path: string | null;
  layout_path: string | null;
  document_type: string;
  status: ScannedDocumentStatus;
  markdown: string;
  markdown_body: string;
  layout_metadata: LayoutMetadata | null;
  stale_layout_references: string[];
};

export type WorkspaceScanResult = {
  vault_path: string;
  documents: ScannedDocument[];
  issues: Array<{
    code: string;
    message: string;
    document_id: string | null;
    markdown_relative_path: string | null;
    classification: "recovery_issue" | "schema_warning" | "preserved_unknown_content";
    suggested_action?: string | null;
  }>;
  index?: IndexSnapshot;
};

export type RecoveryIssue = {
  code: string;
  message: string;
  document_id: string | null;
  markdown_relative_path: string | null;
  action: string | null;
};

export type WorkspaceRecoveryPreview = {
  vault_path: string;
  issues: RecoveryIssue[];
  scan: WorkspaceScanResult | null;
};

export type RecoveryResult = {
  action: string;
  document_id: string | null;
  markdown_relative_path: string | null;
  changed_paths: string[];
  message: string;
  scan: WorkspaceScanResult | null;
};

export type NoteSummary = {
  document_id: string;
  title: string;
  markdown_relative_path: string;
  excerpt: string;
  status: ScannedDocumentStatus;
  updated_at: string | null;
};

export type NoteDocument = {
  document_id: string;
  title: string;
  markdown_relative_path: string;
  markdown_body: string;
  layout_metadata: LayoutMetadata | null;
  parsed_entity: ParsedEntityContract;
  schema_warnings: string[];
};

export type MarkdownAsset = {
  module_id: string;
  document_id: string;
  vault_relative_path: string;
  markdown_link: string;
  content_hash: string;
  byte_len: number;
  mime_type: string;
};

export type MarkdownAssetRead = {
  module_id: string;
  document_id: string;
  vault_relative_path: string;
  mime_type: string;
  byte_len: number;
  bytes: number[];
};

const mockNotesStorageKey = "bentolife:mockNotes";
const mockLayoutsStorageKey = "bentolife:mockLayouts";

export async function scanWorkspace(vaultPath: string) {
  if (!isTauriRuntime()) {
    return mockScanWorkspace(vaultPath);
  }

  const result = await safeInvoke("scan_workspace", { vaultPath }, normalizeWorkspaceScanResult, emptyWorkspaceScanResult(vaultPath));
  return result.data;
}

export async function previewWorkspaceRecovery(vaultPath: string) {
  if (!isTauriRuntime()) {
    const scan = mockScanWorkspace(vaultPath);
    return {
      vault_path: vaultPath,
      issues: scan.issues
        .filter((issue) => issue.classification === "recovery_issue")
        .map((issue) => ({ ...issue, action: mockActionForIssue(issue.code) })),
      scan,
    } satisfies WorkspaceRecoveryPreview;
  }

  const result = await safeInvoke(
    "preview_workspace_recovery",
    { vaultPath },
    normalizeWorkspaceRecoveryPreview,
    emptyWorkspaceRecoveryPreview(vaultPath),
  );
  return result.data;
}

export async function recoverDocumentMetadata(vaultPath: string, markdownRelativePath: string) {
  if (!isTauriRuntime()) {
    return mockRecoveryResult("recover_document_metadata", vaultPath, null, markdownRelativePath);
  }

  return invoke<RecoveryResult>("recover_document_metadata", { vaultPath, markdownRelativePath });
}

export async function recoverLayoutMetadata(vaultPath: string, documentId: string) {
  if (!isTauriRuntime()) {
    const notes = readMockNotes();
    const note = notes.find((candidate) => candidate.document_id === documentId);
    if (note) {
      note.status = "managed";
      writeMockNotes(notes);
      writeMockLayout(documentId, mockLayoutMetadata(documentId, note.markdown_body));
    }
    return mockRecoveryResult("recover_layout_metadata", vaultPath, documentId, note?.markdown_relative_path ?? null);
  }

  return invoke<RecoveryResult>("recover_layout_metadata", { vaultPath, documentId });
}

export async function orphanMissingDocumentMetadata(vaultPath: string, documentId: string) {
  if (!isTauriRuntime()) {
    return mockRecoveryResult("orphan_missing_document_metadata", vaultPath, documentId, null);
  }

  return invoke<RecoveryResult>("orphan_missing_document_metadata", { vaultPath, documentId });
}

export async function restoreOrphanedDocumentMetadata(
  vaultPath: string,
  documentId: string,
  markdownRelativePath: string,
) {
  if (!isTauriRuntime()) {
    return mockRecoveryResult("restore_orphaned_document_metadata", vaultPath, documentId, markdownRelativePath);
  }

  return invoke<RecoveryResult>("restore_orphaned_document_metadata", {
    vaultPath,
    documentId,
    markdownRelativePath,
  });
}

export async function repairDocumentFrontmatterReference(vaultPath: string, documentId: string) {
  if (!isTauriRuntime()) {
    const note = readMockNotes().find((candidate) => candidate.document_id === documentId);
    return mockRecoveryResult(
      "repair_document_frontmatter_reference",
      vaultPath,
      documentId,
      note?.markdown_relative_path ?? null,
    );
  }

  return invoke<RecoveryResult>("repair_document_frontmatter_reference", { vaultPath, documentId });
}

export async function loadLayoutMetadata(vaultPath: string, documentId: string) {
  if (!isTauriRuntime()) {
    return readMockLayout(documentId) ?? mockLayoutMetadata(documentId);
  }

  return invoke<LayoutMetadata>("load_layout_metadata", { vaultPath, documentId });
}

export async function saveLayoutMetadata(vaultPath: string, documentId: string, layoutMetadata: LayoutMetadata) {
  if (!isTauriRuntime()) {
    if (layoutMetadata.document_id !== documentId) {
      throw new Error("Layout metadata document ID does not match the requested document ID.");
    }
    writeMockLayout(documentId, layoutMetadata);
    const notes = readMockNotes();
    const note = notes.find((candidate) => candidate.document_id === documentId);
    if (note && note.status === "layout_missing") {
      note.status = "managed";
      writeMockNotes(notes);
    }
    return layoutMetadata;
  }

  return invoke<LayoutMetadata>("save_layout_metadata", { vaultPath, documentId, layoutMetadata });
}

export async function listNotes(vaultPath: string) {
  if (!isTauriRuntime()) {
    return mockListNotes(vaultPath);
  }

  const result = await safeInvoke("list_notes", { vaultPath }, normalizeNoteSummaries, []);
  return result.data;
}

export async function readNote(vaultPath: string, documentId: string) {
  if (!isTauriRuntime()) {
    return mockReadNote(vaultPath, documentId);
  }

  return normalizeNoteDocument(await invoke<unknown>("read_note", { vaultPath, documentId }));
}

export async function createNote(vaultPath: string, title: string, markdownBody?: string) {
  if (!isTauriRuntime()) {
    return mockCreateNote(vaultPath, title, markdownBody);
  }

  return invoke<NoteDocument>("create_note", { vaultPath, title, markdownBody });
}

export async function updateNote(
  vaultPath: string,
  documentId: string,
  markdownBody: string,
  expectedContentHash?: string | null,
  overwriteConflict = false,
) {
  if (!isTauriRuntime()) {
    return mockUpdateNote(vaultPath, documentId, markdownBody, expectedContentHash, overwriteConflict);
  }

  return invoke<NoteDocument>("update_note", { vaultPath, documentId, markdownBody, expectedContentHash, overwriteConflict });
}

export async function renameNote(vaultPath: string, documentId: string, newTitle: string) {
  if (!isTauriRuntime()) {
    return mockRenameNote(vaultPath, documentId, newTitle);
  }

  return invoke<NoteDocument>("rename_note", { vaultPath, documentId, newTitle });
}

export async function saveMarkdownAsset(
  vaultPath: string,
  moduleId: string,
  documentId: string,
  fileName: string | null,
  mimeType: string,
  bytes: number[],
) {
  if (!isTauriRuntime()) {
    return mockSaveMarkdownAsset(vaultPath, moduleId, documentId, fileName, mimeType, bytes);
  }

  return invoke<MarkdownAsset>("save_markdown_asset", { vaultPath, moduleId, documentId, fileName, mimeType, bytes });
}

export async function readMarkdownAsset(
  vaultPath: string,
  moduleId: string,
  documentId: string,
  source: string,
) {
  if (!isTauriRuntime()) {
    return mockReadMarkdownAsset(moduleId, documentId, source);
  }

  return invoke<MarkdownAssetRead>("read_markdown_asset", { vaultPath, moduleId, documentId, source });
}

function mockScanWorkspace(vaultPath: string): WorkspaceScanResult {
  const documents = readMockNotes().map((note) => scannedDocumentFromNote(note));

  return {
    vault_path: vaultPath,
    documents,
    issues: documents.flatMap((document) => issueForMockDocument(document)),
  };
}

function issueForMockDocument(document: ScannedDocument): WorkspaceScanResult["issues"] {
  switch (document.status) {
    case "layout_missing":
      return [
        {
          code: "layout_missing",
          document_id: document.document_id,
          markdown_relative_path: document.markdown_relative_path,
          message: "Layout metadata is missing; a generated dashboard fallback will be used.",
          classification: "recovery_issue",
          suggested_action: "Open Recovery",
        },
      ];
    case "metadata_missing":
      return [
        {
          code: "metadata_missing",
          document_id: document.document_id,
          markdown_relative_path: document.markdown_relative_path,
          message: "Markdown content is safe, but document metadata is missing.",
          classification: "recovery_issue",
          suggested_action: "Open Recovery",
        },
      ];
    case "metadata_path_mismatch":
      return [
        {
          code: "metadata_path_mismatch",
          document_id: document.document_id,
          markdown_relative_path: document.markdown_relative_path,
          message: "Frontmatter points at stale metadata and can be repaired.",
          classification: "recovery_issue",
          suggested_action: "Open Recovery",
        },
      ];
    case "duplicate_identity":
      return [
        {
          code: "duplicate_identity",
          document_id: document.document_id,
          markdown_relative_path: document.markdown_relative_path,
          message: "Two Markdown files share a document identity and need review.",
          classification: "recovery_issue",
          suggested_action: "Open Recovery",
        },
      ];
    default:
      return [];
  }
}

function mockListNotes(_vaultPath: string): NoteSummary[] {
  return readMockNotes().map((note) => {
    const layoutMetadata = readMockLayout(note.document_id);

    return {
      document_id: note.document_id,
      title: note.title,
      markdown_relative_path: note.markdown_relative_path,
      excerpt: excerptFromMarkdown(note.markdown_body),
      status: layoutMetadata && note.status === "layout_missing" ? "managed" : note.status,
      updated_at: note.updated_at,
    };
  });
}

function mockReadNote(_vaultPath: string, documentId: string): NoteDocument {
  const note = readMockNotes().find((candidate) => candidate.document_id === documentId);
  if (!note) {
    throw new Error("Note was not found.");
  }
  return toNoteDocument(note);
}

function mockCreateNote(_vaultPath: string, title: string, markdownBody?: string): NoteDocument {
  const notes = readMockNotes();
  const cleanTitle = title.trim() || "Untitled Note";
  const documentId = `bl_doc_mock_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
  const note: MockNote = {
    document_id: documentId,
    title: cleanTitle,
    markdown_relative_path: uniqueMockPath(notes, cleanTitle),
    markdown_body: normalizeMarkdownBody(cleanTitle, markdownBody ?? ""),
    status: "layout_missing",
    updated_at: new Date().toISOString(),
    content_hash: "",
  };
  note.content_hash = mockContentHash(note.markdown_body);
  writeMockNotes([...notes, note]);
  return toNoteDocument(note);
}

function mockUpdateNote(
  _vaultPath: string,
  documentId: string,
  markdownBody: string,
  expectedContentHash?: string | null,
  overwriteConflict = false,
): NoteDocument {
  const notes = readMockNotes();
  const note = notes.find((candidate) => candidate.document_id === documentId);
  if (!note) {
    throw new Error("Note was not found.");
  }
  if (!overwriteConflict && expectedContentHash && expectedContentHash !== note.content_hash) {
    throw new Error("Note was changed outside BentoLife. Reload latest, save as copy, or choose overwrite before saving.");
  }
  note.markdown_body = markdownBody.trim() ? `${markdownBody.trim()}\n` : `# ${note.title}\n`;
  note.title = titleFromMarkdown(note.markdown_body, note.title);
  note.updated_at = new Date().toISOString();
  note.content_hash = mockContentHash(note.markdown_body);
  writeMockNotes(notes);
  return toNoteDocument(note);
}

function mockRenameNote(_vaultPath: string, documentId: string, newTitle: string): NoteDocument {
  const notes = readMockNotes();
  const note = notes.find((candidate) => candidate.document_id === documentId);
  if (!note) {
    throw new Error("Note was not found.");
  }
  const cleanTitle = newTitle.trim() || "Untitled Note";
  const nextPath = `modules/notes/data/${slugify(cleanTitle)}.md`;
  if (notes.some((candidate) => candidate.document_id !== documentId && candidate.markdown_relative_path === nextPath)) {
    throw new Error(`A note already exists at ${nextPath}.`);
  }
  note.title = cleanTitle;
  note.markdown_relative_path = nextPath;
  note.markdown_body = replaceOrInsertTitle(note.markdown_body, cleanTitle);
  note.updated_at = new Date().toISOString();
  note.content_hash = mockContentHash(note.markdown_body);
  writeMockNotes(notes);
  return toNoteDocument(note);
}

function mockSaveMarkdownAsset(
  _vaultPath: string,
  moduleId: string,
  documentId: string,
  fileName: string | null,
  mimeType: string,
  bytes: number[],
): MarkdownAsset {
  if (!["image/png", "image/jpeg", "image/jpg", "image/webp"].includes(mimeType.toLowerCase())) {
    throw new Error("Only pasted PNG, JPEG, and WEBP images are supported.");
  }
  if (!bytes.length) {
    throw new Error("Pasted image was empty.");
  }
  const extension = mimeType.toLowerCase().includes("webp") ? "webp" : mimeType.toLowerCase().includes("png") ? "png" : "jpg";
  const hash = mockContentHash(bytes.join(","));
  const safeName = (fileName ?? "pasted-image")
    .toLowerCase()
    .replace(/\.[a-z0-9]+$/i, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "pasted-image";
  const vaultRelativePath = `assets/${moduleId}/${documentId}/mock-${hash.slice(0, 8)}-${safeName}.${extension}`;
  return {
    module_id: moduleId,
    document_id: documentId,
    vault_relative_path: vaultRelativePath,
    markdown_link: `../../../${vaultRelativePath}`,
    content_hash: hash,
    byte_len: bytes.length,
    mime_type: mimeType,
  };
}

function mockReadMarkdownAsset(moduleId: string, documentId: string, source: string): MarkdownAssetRead {
  const normalized = source.trim().replace(/\\/g, "/");
  const lower = normalized.toLowerCase();
  if (
    !normalized ||
    lower.startsWith("http://") ||
    lower.startsWith("https://") ||
    lower.startsWith("data:") ||
    lower.startsWith("javascript:") ||
    lower.startsWith("file:") ||
    lower.endsWith(".svg") ||
    normalized.includes("..")
  ) {
    throw new Error("Markdown asset source must be a safe vault-relative image path.");
  }
  const vaultRelativePath = normalized.startsWith("assets/") ? normalized : `assets/${moduleId}/${documentId}/${normalized}`;
  if (!vaultRelativePath.startsWith(`assets/${moduleId}/${documentId}/`)) {
    throw new Error("Markdown asset source is outside the document asset folder.");
  }
  const extension = vaultRelativePath.toLowerCase().split(".").pop();
  const mimeType = extension === "webp" ? "image/webp" : extension === "jpg" || extension === "jpeg" ? "image/jpeg" : "image/png";
  return {
    module_id: moduleId,
    document_id: documentId,
    vault_relative_path: vaultRelativePath,
    mime_type: mimeType,
    byte_len: 8,
    bytes: [137, 80, 78, 71, 13, 10, 26, 10],
  };
}

type MockNote = {
  document_id: string;
  title: string;
  markdown_relative_path: string;
  markdown_body: string;
  status: ScannedDocumentStatus;
  updated_at: string | null;
  content_hash: string;
};

function readMockNotes(): MockNote[] {
  const serialized = window.localStorage.getItem(mockNotesStorageKey);
  if (!serialized) {
    return [];
  }

  try {
    return JSON.parse(serialized) as MockNote[];
  } catch {
    return [];
  }
}

function writeMockNotes(notes: MockNote[]) {
  window.localStorage.setItem(mockNotesStorageKey, JSON.stringify(notes));
}

function scannedDocumentFromNote(note: MockNote): ScannedDocument {
  const layout_metadata = readMockLayout(note.document_id);

  return {
    document_id: note.document_id,
    title: note.title,
    markdown_relative_path: note.markdown_relative_path,
    metadata_path: `.bentolifelayout/documents/${note.document_id}.json`,
    layout_path: `.bentolifelayout/layouts/${note.document_id}.layout.json`,
    document_type: "note",
    status: note.status,
    markdown: note.markdown_body,
    markdown_body: note.markdown_body,
    layout_metadata,
    stale_layout_references: layout_metadata ? staleLayoutReferences(layout_metadata, note.markdown_body) : [],
  };
}

function toNoteDocument(note: MockNote): NoteDocument {
  return {
    document_id: note.document_id,
    title: note.title,
    markdown_relative_path: note.markdown_relative_path,
    markdown_body: note.markdown_body,
      layout_metadata: null,
      parsed_entity: {
      module_id: "notes",
      entity_type: "note",
      fields: { title: note.title },
      field_descriptors: [
        { id: "title", label: "Title", type: "text", renderer_id: "text", value: note.title, editable: false, aliases: [], warnings: [] },
      ],
      blocks: parseMockMarkdownBlocks(note.markdown_body),
      unknown_blocks: [],
      relationships: [],
      tags: [],
      path: note.markdown_relative_path,
      content_hash: note.content_hash || mockContentHash(note.markdown_body),
    },
    schema_warnings: [],
  };
}

function parseMockMarkdownBlocks(markdownBody: string): ParsedEntityContract["blocks"] {
  return markdownBody.split(/\r?\n/).filter(Boolean).map((line) => {
    const trimmed = line.trim();
    const image = trimmed.match(/^!\[(.*)]\((.*)\)$/);
    if (image && !/^https?:\/\//i.test(image[2])) {
      return { type: "image", alt: image[1], source: image[2], raw: line };
    }
    if (trimmed.startsWith("# ")) {
      return { type: "heading", level: 1, text: trimmed.replace(/^#\s+/, "") };
    }
    if (trimmed.startsWith("## ")) {
      return { type: "heading", level: 2, text: trimmed.replace(/^##\s+/, "") };
    }
    return { type: "paragraph", text: line };
  });
}

function normalizeMarkdownBody(title: string, markdownBody: string) {
  const body = markdownBody.trim();
  if (!body) {
    return `# ${title}\n`;
  }
  if (body.split(/\r?\n/).some((line) => line.trimStart().startsWith("# "))) {
    return `${body}\n`;
  }
  return `# ${title}\n\n${body}\n`;
}

function replaceOrInsertTitle(markdownBody: string, title: string) {
  const lines = markdownBody.trim().split(/\r?\n/);
  const index = lines.findIndex((line) => line.trimStart().startsWith("# "));
  if (index >= 0) {
    lines[index] = `# ${title}`;
  } else {
    lines.unshift(`# ${title}`);
  }
  return `${lines.join("\n")}\n`;
}

function titleFromMarkdown(markdownBody: string, fallback: string) {
  return (
    markdownBody
      .split(/\r?\n/)
      .find((line) => line.trimStart().startsWith("# "))
      ?.replace(/^#\s+/, "")
      .trim() || fallback
  );
}

function excerptFromMarkdown(markdownBody: string) {
  return (
    markdownBody
      .split(/\r?\n/)
      .map((line) => line.trim())
      .find((line) => line && !line.startsWith("#") && !line.startsWith("- [")) || "No preview yet."
  );
}

function uniqueMockPath(notes: MockNote[], title: string) {
  const slug = slugify(title);
  let candidate = `modules/notes/data/${slug}.md`;
  let index = 1;
  while (notes.some((note) => note.markdown_relative_path === candidate)) {
    candidate = `modules/notes/data/${slug}-${index}.md`;
    index += 1;
  }
  return candidate;
}

function slugify(title: string) {
  const slug = title
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

  return slug || "untitled-note";
}

function mockContentHash(value: string) {
  let hash = 0;
  for (const character of value) {
    hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

function mockActionForIssue(code: string) {
  switch (code) {
    case "metadata_missing":
      return "recover_document_metadata";
    case "layout_missing":
      return "recover_layout_metadata";
    case "markdown_missing":
      return "orphan_missing_document_metadata";
    case "metadata_path_mismatch":
      return "repair_document_frontmatter_reference";
    default:
      return null;
  }
}

function mockRecoveryResult(
  action: string,
  vaultPath: string,
  documentId: string | null,
  markdownRelativePath: string | null,
): RecoveryResult {
  return {
    action,
    document_id: documentId,
    markdown_relative_path: markdownRelativePath,
    changed_paths: [],
    message: "Mock recovery action completed.",
    scan: mockScanWorkspace(vaultPath),
  };
}

function readMockLayout(documentId: string) {
  try {
    const layouts = JSON.parse(window.localStorage.getItem(mockLayoutsStorageKey) ?? "{}") as Record<string, LayoutMetadata>;
    return layouts[documentId] ?? null;
  } catch {
    return null;
  }
}

function writeMockLayout(documentId: string, layout: LayoutMetadata) {
  const layouts = (() => {
    try {
      return JSON.parse(window.localStorage.getItem(mockLayoutsStorageKey) ?? "{}") as Record<string, LayoutMetadata>;
    } catch {
      return {};
    }
  })();
  layouts[documentId] = layout;
  window.localStorage.setItem(mockLayoutsStorageKey, JSON.stringify(layouts));
}

function mockLayoutMetadata(documentId: string, markdownBody = ""): LayoutMetadata {
  const headings = markdownBody
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => /^(#|##)\s+.+$/.test(line));

  return {
    schema_version: 1,
    document_id: documentId,
    vault_relative: true,
    layout_mode: "bento-dashboard",
    theme: "clean-slate",
    cards: headings.map((heading, index) => ({
      section_match: heading,
      card_id: `card_${index}_${slugify(heading.replace(/^#+\s+/, ""))}`,
      width: "single",
      order: index,
      widget: "rich_text",
    })),
    fallback_layout: {
      strategy: "generate_cards_from_markdown_headings",
      default_width: "single",
      preserve_markdown_order: true,
    },
    updated_at: "mock",
  };
}

function staleLayoutReferences(layout: LayoutMetadata, markdownBody: string) {
  const headings = new Set(
    markdownBody
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => /^(#|##)\s+.+$/.test(line)),
  );

  return layout.cards.filter((card) => !headings.has(card.section_match)).map((card) => card.section_match);
}
