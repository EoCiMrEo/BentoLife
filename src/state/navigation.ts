import type { ModuleDefinition, RegistryState } from "@/services/backendCore";

export type AppView =
  | "dashboard"
  | "notes"
  | "todos"
  | "contacts"
  | "habits"
  | "navigator"
  | "architect"
  | "vault"
  | "settings"
  | "trash"
  | "archive"
  | "module";

export const appViews: AppView[] = [
  "dashboard",
  "notes",
  "todos",
  "contacts",
  "habits",
  "navigator",
  "architect",
  "vault",
  "settings",
  "trash",
  "archive",
  "module",
];

export const viewLabels: Record<AppView, string> = {
  dashboard: "Dashboard",
  notes: "Notes",
  todos: "Todos",
  contacts: "Contacts",
  habits: "Habits",
  navigator: "Navigator",
  architect: "Architect",
  vault: "Vault",
  settings: "Settings",
  trash: "Trash",
  archive: "Archive",
  module: "Module",
};

export type ModuleNavEntry = {
  id: string;
  label: string;
  moduleId: string;
  view: AppView;
  kind: string;
  defaultView: string;
  documentType: string;
  system: boolean;
};

export const fallbackModuleNavEntries: ModuleNavEntry[] = [
  { id: "notes", label: "Notes", moduleId: "notes", view: "notes", kind: "starter", defaultView: "cards", documentType: "note", system: false },
  { id: "todos", label: "Todos", moduleId: "todos", view: "todos", kind: "starter", defaultView: "cards", documentType: "todos", system: false },
];

export function buildModuleNavEntries(registry: RegistryState | null): ModuleNavEntry[] {
  if (!registry) return fallbackModuleNavEntries;
  return registry.modules
    .filter((module) => module.available && module.installed && module.enabled)
    .map(moduleToNavEntry)
    .filter((entry) => !entry.system);
}

export function buildSystemNavEntries(registry: RegistryState | null): ModuleNavEntry[] {
  const entries = registry ? registry.modules.filter((module) => module.available && module.installed && module.enabled).map(moduleToNavEntry) : [];
  const systemEntries = entries.filter((entry) => entry.system && entry.view !== "navigator");
  const byId = new Map(systemEntries.map((entry) => [entry.id, entry]));
  for (const entry of [
    { id: "architect", label: "Architect", moduleId: "architect", view: "architect" as AppView, kind: "system", defaultView: "system", documentType: "architect", system: true },
    { id: "settings", label: "Settings", moduleId: "settings", view: "settings" as AppView, kind: "system", defaultView: "system", documentType: "settings", system: true },
    { id: "trash", label: "Trash", moduleId: "trash", view: "trash" as AppView, kind: "system", defaultView: "system", documentType: "trash", system: true },
    { id: "archive", label: "Archive", moduleId: "archive", view: "archive" as AppView, kind: "system", defaultView: "system", documentType: "archive", system: true },
  ]) {
    if (!byId.has(entry.id)) byId.set(entry.id, entry);
  }
  return ["architect", "settings", "trash", "archive"].map((id) => byId.get(id)).filter(Boolean) as ModuleNavEntry[];
}

function moduleToNavEntry(module: ModuleDefinition): ModuleNavEntry {
  return {
    id: module.id,
    label: module.display_name || fallbackLabel(module.id),
    moduleId: module.id,
    view: viewForModule(module.id),
    kind: module.kind,
    defaultView: module.default_view,
    documentType: module.document_type,
    system: module.kind === "system",
  };
}

export function viewForModule(moduleId: string): AppView {
  switch (moduleId) {
    case "notes":
      return "notes";
    case "todos":
      return "todos";
    case "contacts":
      return "contacts";
    case "habits":
      return "habits";
    case "navigator":
      return "navigator";
    case "architect":
      return "architect";
    case "vault":
      return "vault";
    case "settings":
      return "settings";
    case "trash":
      return "trash";
    case "archive":
      return "archive";
    default:
      return "module";
  }
}

function fallbackLabel(moduleId: string) {
  return moduleId
    .replace(/[-_]+/g, " ")
    .replace(/\b\w/g, (character) => character.toUpperCase())
    .trim() || "Module";
}

export function isAppView(value: string): value is AppView {
  return appViews.includes(value as AppView);
}

export function getNextView(current: AppView, requested: string): AppView {
  return isAppView(requested) ? requested : current;
}

export function isDashboardCardView(view: AppView) {
  return (
    view === "notes" ||
    view === "todos" ||
    view === "contacts" ||
    view === "habits" ||
    view === "navigator" ||
    view === "architect" ||
    view === "vault" ||
    view === "settings" ||
    view === "trash" ||
    view === "archive" ||
    view === "module"
  );
}

export type FocusTarget = {
  architectTab?: "modules" | "dashboard" | "appearance" | "schemas" | "data_graph" | "recovery";
  documentId?: string;
  label: string;
  moduleId?: string;
  view: AppView;
};

export type NavigationTarget = {
  label: string;
  options?: Partial<FocusTarget>;
  view: AppView;
};

export function normalizeNavigationTarget(view: AppView, label = viewLabels[view], options?: Partial<FocusTarget>): NavigationTarget {
  if (view !== "navigator") {
    return { label, options, view };
  }

  return {
    label: "Data & Graph",
    options: { ...options, architectTab: "data_graph", moduleId: options?.moduleId ?? "navigator" },
    view: "architect",
  };
}

export function contextHeaderLabel(target: FocusTarget) {
  return `Dashboard / ${target.label.trim() || viewLabels[target.view]}`;
}
