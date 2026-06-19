import { invoke } from "@tauri-apps/api/core";

import {
  emptyDashboardHubDocument,
  normalizeDashboardHubDocument,
} from "@/services/contracts/dashboard.contract";
import { safeInvoke } from "@/services/contracts/invokeClient";
import type { LayoutMetadata } from "@/services/notes";
import { isTauriRuntime } from "@/services/vault";

export type DashboardPinnedEntity = {
  label: string;
  target: string;
  document_id: string;
  title: string;
  markdown_relative_path: string;
  entity_type: string;
};

export type DashboardModuleSummary = {
  module_id: string;
  display_name: string;
  status: string;
  entity_count: number;
  index_path: string;
};

export type DashboardHubDocument = {
  document_id: string | null;
  layout_metadata?: LayoutMetadata | null;
  markdown_relative_path: string;
  title: string;
  markdown_body: string;
  pinned_entities: DashboardPinnedEntity[];
  unresolved_pins: string[];
  module_summaries: DashboardModuleSummary[];
  warnings: string[];
};

const mockDashboardHubKey = "bentolife:mockDashboardHub";
const mockLayoutsStorageKey = "bentolife:mockLayouts";
const mockNotesStorageKey = "bentolife:mockNotes";

type MockPinnedNote = {
  document_id: string;
  markdown_relative_path: string;
  title: string;
};

export async function readDashboardHub(vaultPath: string) {
  if (!isTauriRuntime()) {
    return mockDashboardHub(vaultPath);
  }

  const result = await safeInvoke("read_dashboard_hub", { vaultPath }, normalizeDashboardHubDocument, emptyDashboardHubDocument());
  return result.data;
}

export async function pinDashboardEntity(vaultPath: string, documentId: string) {
  if (!isTauriRuntime()) {
    return mockSetPinned(vaultPath, documentId, true);
  }

  return normalizeDashboardHubDocument(await invoke<unknown>("pin_dashboard_entity", { vaultPath, documentId }));
}

export async function unpinDashboardEntity(vaultPath: string, documentId: string) {
  if (!isTauriRuntime()) {
    return mockSetPinned(vaultPath, documentId, false);
  }

  return normalizeDashboardHubDocument(await invoke<unknown>("unpin_dashboard_entity", { vaultPath, documentId }));
}

export function setMockDashboardHub(markdown: string) {
  window.localStorage.setItem(mockDashboardHubKey, markdown);
}

function mockDashboardHub(_vaultPath: string): DashboardHubDocument {
  const markdown =
    window.localStorage.getItem(mockDashboardHubKey) ??
    "# Dashboard\n";

  return {
    document_id: "bl_doc_mock_dashboard",
    layout_metadata: readMockLayout("bl_doc_mock_dashboard"),
    markdown_relative_path: "INDEX.md",
    title: titleFromMarkdown(markdown),
    markdown_body: markdown,
    pinned_entities: mockPinnedEntities(markdown),
    unresolved_pins: markdown.includes("[[Missing]]") ? ["Missing"] : [],
    module_summaries: [
      { display_name: "Navigator", entity_count: 1, index_path: "modules/navigator/INDEX.md", module_id: "navigator", status: "implemented" },
      { display_name: "Notes", entity_count: 1, index_path: "modules/notes/INDEX.md", module_id: "notes", status: "implemented" },
      { display_name: "Todos", entity_count: 0, index_path: "modules/todos/INDEX.md", module_id: "todos", status: "implemented" },
      { display_name: "Contacts", entity_count: 0, index_path: "modules/contacts/INDEX.md", module_id: "contacts", status: "implemented" },
      { display_name: "Habits", entity_count: 0, index_path: "modules/habits/INDEX.md", module_id: "habits", status: "implemented" },
    ],
    warnings: [],
  };
}

function mockSetPinned(_vaultPath: string, documentId: string, pinned: boolean): DashboardHubDocument {
  const hub = mockDashboardHub(_vaultPath);
  const knownPin =
    hub.pinned_entities.find((pin) => pin.document_id === documentId) ??
    mockPinnedNote(documentId) ??
    {
      document_id: documentId,
      entity_type: "note",
      label: "Pinned note",
      markdown_relative_path: documentId,
      target: documentId,
      title: "Pinned note",
    };
  const markdown = window.localStorage.getItem(mockDashboardHubKey) ?? "# Dashboard\n";
  const nextMarkdown = pinned
    ? ensureMockPin(markdown, knownPin)
    : removeMockPin(markdown, knownPin);
  window.localStorage.setItem(mockDashboardHubKey, nextMarkdown);
  return mockDashboardHub(_vaultPath);
}

function mockPinnedNote(documentId: string): DashboardPinnedEntity | null {
  try {
    const notes = JSON.parse(window.localStorage.getItem(mockNotesStorageKey) ?? "[]") as MockPinnedNote[];
    const note = notes.find((candidate) => candidate.document_id === documentId);
    if (!note) return null;
    return {
      document_id: note.document_id,
      entity_type: "note",
      label: note.title,
      markdown_relative_path: note.markdown_relative_path,
      target: note.markdown_relative_path,
      title: note.title,
    };
  } catch {
    return null;
  }
}

function mockPinnedEntities(markdown: string): DashboardPinnedEntity[] {
  try {
    const notes = JSON.parse(window.localStorage.getItem(mockNotesStorageKey) ?? "[]") as MockPinnedNote[];
    const resolved = notes
      .filter((note) => markdown.includes(note.markdown_relative_path) || markdown.includes(note.title))
      .map((note) => ({
        document_id: note.document_id,
        entity_type: "note",
        label: note.title,
        markdown_relative_path: note.markdown_relative_path,
        target: note.markdown_relative_path,
        title: note.title,
      }));
    if (resolved.length) return resolved;
  } catch {
    // Fall through to the legacy static browser fixture below.
  }
  return markdown.includes("modules/notes/data/daily-note.md") || markdown.includes("Daily Note")
    ? [{
        document_id: "bl_doc_mock_daily_note",
        entity_type: "note",
        label: "Daily Note",
        markdown_relative_path: "modules/notes/data/daily-note.md",
        target: "modules/notes/data/daily-note.md",
        title: "Daily Note",
      }]
    : [];
}

function ensureMockPin(markdown: string, pin: DashboardPinnedEntity) {
  if (markdown.includes(pin.target) || markdown.includes(pin.title)) return markdown;
  const pinLine = `- [${pin.title}](${pin.target})`;
  if (/^##\s+Pinned(?: Entities)?\s*$/im.test(markdown)) {
    return markdown.replace(/(^##\s+Pinned(?: Entities)?\s*$)/im, `$1\n\n${pinLine}`);
  }
  return `${markdown.trimEnd()}\n\n## Pinned Entities\n\n${pinLine}\n`;
}

function removeMockPin(markdown: string, pin: DashboardPinnedEntity) {
  return markdown
    .split(/\r?\n/)
    .filter((line) => !(line.includes(pin.target) || line.includes(pin.title)))
    .join("\n")
    .trimEnd() + "\n";
}

function readMockLayout(documentId: string) {
  try {
    const layouts = JSON.parse(window.localStorage.getItem(mockLayoutsStorageKey) ?? "{}") as Record<string, LayoutMetadata>;
    return layouts[documentId] ?? null;
  } catch {
    return null;
  }
}

function titleFromMarkdown(markdown: string) {
  return (
    markdown
      .split(/\r?\n/)
      .find((line) => line.trim().startsWith("# "))
      ?.replace(/^#\s+/, "")
      .trim() || "Today"
  );
}
