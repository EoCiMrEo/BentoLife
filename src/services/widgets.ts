import { invoke } from "@tauri-apps/api/core";

import { assertNoLayoutOverlap, compactLayout, findNextAvailableLayout, normalizeLayout } from "@/domain/dashboard/layout";
import {
  emptyDashboardWidgetState,
  normalizeDashboardWidgetState,
  normalizeWidgetTypeRegistry,
} from "@/services/contracts/widgets.contract";
import {
  defaultWorkspaceUiState as contractDefaultWorkspaceUiState,
  normalizeWorkspaceUiState,
} from "@/services/contracts/architect.contract";
import { safeInvoke } from "@/services/contracts/invokeClient";
import type { RegistryState } from "@/services/backendCore";
import { isTauriRuntime } from "@/services/vault";

export type WidgetConfigFieldType = "text" | "number" | "boolean" | "enum" | "tags" | "date_range";

export type WidgetSizeDefinition = {
  width: number;
  height: number;
};

export type WidgetConfigFieldDefinition = {
  type: WidgetConfigFieldType;
  default?: unknown;
  min?: number;
  max?: number;
  options?: string[];
};

export type WidgetTypeDefinition = {
  id: string;
  module_id: string;
  label: string;
  description: string;
  renderer_id: string;
  default_size: WidgetSizeDefinition;
  allowed_sizes: WidgetSizeDefinition[];
  config_schema: Record<string, WidgetConfigFieldDefinition>;
};

export type DashboardWidgetLayout = {
  column: number;
  row: number;
  width: number;
  height: number;
};

export type DashboardWidgetInstance = {
  instance_id: string;
  widget_type: string;
  module_id: string;
  title?: string | null;
  config: Record<string, unknown>;
  layout: DashboardWidgetLayout;
  collapsed: boolean;
};

export type DashboardWidgetWarning =
  | string
  | {
      instance_id?: string | null;
      module_id?: string | null;
      widget_type?: string | null;
      code: string;
      message: string;
    };

export type DashboardWidgetState = {
  schema_version: number;
  instances: DashboardWidgetInstance[];
  warnings: DashboardWidgetWarning[];
  recovery_backup_path?: string | null;
  last_layout_operation?: DashboardWidgetLayoutOperation | null;
};

export type DashboardWidgetLayoutOperation = {
  moved_widget_id?: string | null;
  resized_widget_id?: string | null;
  affected_widget_ids: string[];
  reason: "moved" | "resized" | "compacted";
};

export type WorkspaceUiState = {
  schema_version: number;
  workspace_name: string;
  default_theme: string;
  language?: "en" | "vi";
  architect_active_tab?: ArchitectTabId;
  architect_sections: Record<string, boolean>;
  updated_at: string;
  [key: string]: unknown;
};

export type ArchitectTabId = "modules" | "dashboard" | "appearance" | "schemas" | "data_graph" | "recovery";

export type CreateDashboardWidgetInput = {
  widget_type: string;
  module_id: string;
  title?: string | null;
  config?: Record<string, unknown>;
};

export type UpdateDashboardWidgetInput = {
  title?: string | null;
  config?: Record<string, unknown>;
};

const mockWidgetsStorageKey = "bentolife:mockDashboardWidgets";
const mockWorkspaceStateStorageKey = "bentolife:mockWorkspaceUiState";
const sparseRepairMinRow = 5;
const sparseRepairMargin = 3;
export const DASHBOARD_WIDGET_SPARSE_REPAIR_WARNING =
  "Dashboard widget layout was repaired after detecting sparse rows from an older layout bug.";

export async function readWidgetTypeRegistry(vaultPath: string): Promise<WidgetTypeDefinition[]> {
  if (!isTauriRuntime()) {
    return mockWidgetTypes();
  }
  const result = await safeInvoke("load_widget_types", { vaultPath }, normalizeWidgetTypeRegistry, []);
  return result.data;
}

export async function readDashboardWidgets(vaultPath: string): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    return readMockWidgetState(vaultPath);
  }
  const result = await safeInvoke(
    "load_dashboard_widgets",
    { vaultPath },
    normalizeDashboardWidgetState,
    emptyDashboardWidgetState("Dashboard widget metadata failed to load."),
  );
  return result.data;
}

export async function createDashboardWidget(
  vaultPath: string,
  input: CreateDashboardWidgetInput,
): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    const widgetType = getMockWidgetType(input.widget_type);
    const state = readMockWidgetState(vaultPath);
    const instance: DashboardWidgetInstance = {
      instance_id: `bl_widget_${Date.now().toString(36)}_${state.instances.length}`,
      widget_type: widgetType.id,
      module_id: widgetType.module_id,
      title: input.title?.trim() || widgetType.label,
      config: { ...defaultConfig(widgetType), ...(input.config ?? {}) },
      layout: findNextAvailableLayout(state.instances, widgetType.default_size.width, widgetType.default_size.height),
      collapsed: false,
    };
    return writeMockWidgetState(vaultPath, { ...state, instances: [...state.instances, instance] });
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("create_dashboard_widget", { vaultPath, input }));
}

export async function updateDashboardWidget(
  vaultPath: string,
  instanceId: string,
  input: UpdateDashboardWidgetInput,
): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    return updateMockInstance(vaultPath, instanceId, (instance) => ({
      ...instance,
      title: input.title?.trim() || instance.title,
      config: input.config ? { ...instance.config, ...input.config } : instance.config,
    }));
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("update_dashboard_widget", { vaultPath, instanceId, input }));
}

export async function removeDashboardWidget(vaultPath: string, instanceId: string): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    const state = readMockWidgetState(vaultPath);
    return writeMockWidgetState(vaultPath, {
      ...state,
      instances: state.instances.filter((instance) => instance.instance_id !== instanceId),
    });
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("remove_dashboard_widget", { vaultPath, instanceId }));
}

export async function duplicateDashboardWidget(vaultPath: string, instanceId: string): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    const state = readMockWidgetState(vaultPath);
    const source = findInstance(state, instanceId);
    const sourceTitle = dashboardWidgetTitle(source, getMockWidgetType(source.widget_type));
    const duplicate: DashboardWidgetInstance = {
      ...source,
      instance_id: `bl_widget_${Date.now().toString(36)}_${state.instances.length}`,
      title: `${sourceTitle} copy`,
      layout: findNextAvailableLayout(state.instances, source.layout.width, source.layout.height),
    };
    return writeMockWidgetState(vaultPath, { ...state, instances: [...state.instances, duplicate] });
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("duplicate_dashboard_widget", { vaultPath, instanceId }));
}

export async function moveDashboardWidget(
  vaultPath: string,
  instanceId: string,
  layout: DashboardWidgetLayout,
): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    const state = readMockWidgetState(vaultPath);
    findInstance(state, instanceId);
    const instances = state.instances.map((instance) =>
      instance.instance_id === instanceId ? { ...instance, layout: normalizeLayout(layout) } : instance,
    );
    assertNoLayoutOverlap(instances, instanceId);
    return writeMockWidgetState(
      vaultPath,
      { ...state, instances },
      { affected_widget_ids: [], moved_widget_id: instanceId, resized_widget_id: null, reason: "moved" },
    );
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("move_dashboard_widget", { vaultPath, instanceId, layout }));
}

export async function resizeDashboardWidget(
  vaultPath: string,
  instanceId: string,
  size: WidgetSizeDefinition,
): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    const state = readMockWidgetState(vaultPath);
    const resizedIndex = state.instances.findIndex((instance) => instance.instance_id === instanceId);
    if (resizedIndex < 0) {
      throw new Error("Dashboard widget was not found.");
    }
    const instances = state.instances.map((instance, index) => {
      if (index !== resizedIndex) return instance;
      const widgetType = getMockWidgetType(instance.widget_type);
      const nextSize = widgetType.allowed_sizes.some((allowed) => sameSize(allowed, size)) ? size : widgetType.default_size;
      return { ...instance, layout: normalizeLayout({ ...instance.layout, ...nextSize }) };
    });
    assertNoLayoutOverlap(instances, instanceId);
    return writeMockWidgetState(
      vaultPath,
      { ...state, instances },
      { affected_widget_ids: [], moved_widget_id: null, resized_widget_id: instanceId, reason: "resized" },
    );
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("resize_dashboard_widget", { vaultPath, instanceId, size }));
}

export async function setDashboardWidgetCollapsed(
  vaultPath: string,
  instanceId: string,
  collapsed: boolean,
): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    return updateMockInstance(vaultPath, instanceId, (instance) => ({ ...instance, collapsed }));
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("set_dashboard_widget_collapsed", { vaultPath, instanceId, collapsed }));
}

export async function resetDashboardWidgets(vaultPath: string): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    return writeMockWidgetState(vaultPath, emptyWidgetState());
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("reset_dashboard_widgets", { vaultPath }));
}

export async function compactDashboardWidgets(vaultPath: string): Promise<DashboardWidgetState> {
  if (!isTauriRuntime()) {
    const state = readMockWidgetState(vaultPath);
    const compacted = compactLayout(state.instances);
    const movedWidgetIds = compacted
      .filter((instance, index) => JSON.stringify(instance.layout) !== JSON.stringify(state.instances[index]?.layout))
      .map((instance) => instance.instance_id);
    return writeMockWidgetState(
      vaultPath,
      {
        ...state,
        instances: compacted,
      },
      {
        affected_widget_ids: movedWidgetIds,
        moved_widget_id: null,
        resized_widget_id: null,
        reason: "compacted",
      },
    );
  }
  return normalizeDashboardWidgetState(await invoke<unknown>("compact_dashboard_widgets", { vaultPath }));
}

export async function loadWorkspaceUiState(vaultPath: string): Promise<WorkspaceUiState> {
  if (!isTauriRuntime()) {
    return readMockWorkspaceUiState(vaultPath);
  }
  const result = await safeInvoke("load_workspace_ui_state", { vaultPath }, normalizeWorkspaceUiState, contractDefaultWorkspaceUiState());
  return result.data;
}

export async function saveWorkspaceUiState(vaultPath: string, state: WorkspaceUiState): Promise<WorkspaceUiState> {
  if (!isTauriRuntime()) {
    return writeMockWorkspaceUiState(vaultPath, state);
  }
  return normalizeWorkspaceUiState(await invoke<unknown>("save_workspace_ui_state", { vaultPath, state }));
}

export function isWidgetActive(instance: DashboardWidgetInstance, registry: RegistryState | null) {
  const module = registry?.modules.find((candidate) => candidate.id === instance.module_id);
  return module ? module.available && module.installed && module.enabled : true;
}

export function dashboardWidgetTitle(instance: DashboardWidgetInstance, widgetType?: WidgetTypeDefinition) {
  return instance.title?.trim() || widgetType?.label || instance.widget_type;
}

export function dashboardWidgetWarningMessage(warning: DashboardWidgetWarning) {
  return typeof warning === "string" ? warning : warning.message;
}

export function dashboardWidgetWarningKey(warning: DashboardWidgetWarning, index: number) {
  if (typeof warning === "string") {
    return `${index}-${warning}`;
  }
  return `${warning.code}-${warning.instance_id ?? warning.module_id ?? warning.widget_type ?? "all"}-${index}`;
}

export function isDashboardWidgetSparseRepairWarning(warning: DashboardWidgetWarning) {
  return dashboardWidgetWarningMessage(warning) === DASHBOARD_WIDGET_SPARSE_REPAIR_WARNING;
}

function mockWidgetTypes(): WidgetTypeDefinition[] {
  return [
    widgetType("notes.recent", "notes", "Recent Notes", "Shows recently edited notes.", "recent_notes", { width: 2, height: 1 }, {}, [{ width: 4, height: 2 }, { width: 7, height: 2 }]),
    widgetType("notes.pinned", "notes", "Pinned Notes", "Shows notes pinned from the root dashboard.", "pinned_notes", { width: 2, height: 1 }),
    widgetType("notes.by-tag", "notes", "Notes By Tag", "Shows notes matching a tag.", "notes_by_tag", { width: 1, height: 1 }, { tag: { type: "text", default: "daily" } }),
    widgetType("todos.today", "todos", "Today", "Shows open tasks for today.", "todo_list", { width: 1, height: 1 }),
    widgetType("todos.upcoming", "todos", "Upcoming Tasks", "Shows incomplete tasks due soon.", "todo_list", { width: 2, height: 1 }, { range_days: { type: "number", default: 7, min: 1, max: 90 }, show_completed: { type: "boolean", default: false } }, [{ width: 4, height: 2 }, { width: 7, height: 3 }]),
    widgetType("todos.overdue", "todos", "Overdue Tasks", "Shows tasks that need attention.", "todo_list", { width: 1, height: 1 }),
    widgetType("habits.daily-checkin", "habits", "Daily Check-in", "Shows habits due today.", "habit_checkin", { width: 1, height: 1 }),
    widgetType("habits.weekly-progress", "habits", "Weekly Progress", "Shows habit progress for the week.", "progress", { width: 2, height: 1 }, {}, [{ width: 4, height: 2 }]),
    widgetType("contacts.recent", "contacts", "Recent Contacts", "Shows recently edited contacts.", "recent_contacts", { width: 1, height: 1 }),
  ];
}

function widgetType(
  id: string,
  moduleId: string,
  label: string,
  description: string,
  rendererId: string,
  defaultSize: WidgetSizeDefinition,
  configSchema: Record<string, WidgetConfigFieldDefinition> = {},
  extraSizes: WidgetSizeDefinition[] = [],
): WidgetTypeDefinition {
  return {
    id,
    module_id: moduleId,
    label,
    description,
    renderer_id: rendererId,
    default_size: defaultSize,
    allowed_sizes: [
      { width: 1, height: 1 },
      { width: 2, height: 1 },
      { width: 2, height: 2 },
      ...extraSizes,
    ],
    config_schema: configSchema,
  };
}

function emptyWidgetState(): DashboardWidgetState {
  return { schema_version: 1, instances: [], warnings: [], last_layout_operation: null };
}

function readMockWidgetState(vaultPath: string): DashboardWidgetState {
  try {
    const allState = JSON.parse(window.localStorage.getItem(mockWidgetsStorageKey) ?? "{}") as Record<string, DashboardWidgetState>;
    const state = allState[vaultPath] ?? emptyWidgetState();
    const normalized = {
      ...state,
      last_layout_operation: state.last_layout_operation ?? null,
      schema_version: 1,
      warnings: state.warnings ?? [],
      instances: (state.instances ?? []).map(normalizeInstance),
    };
    const repaired = repairSparseLayoutIfNeeded(normalized);
    if (!repaired) {
      return normalized;
    }

    allState[vaultPath] = {
      ...repaired,
      warnings: normalized.warnings,
      recovery_backup_path: normalized.recovery_backup_path ?? null,
    };
    window.localStorage.setItem(mockWidgetsStorageKey, JSON.stringify(allState));
    return {
      ...repaired,
      recovery_backup_path: "browser-local-storage:previous-dashboard-widgets",
      warnings: [...normalized.warnings, DASHBOARD_WIDGET_SPARSE_REPAIR_WARNING],
    };
  } catch {
    return {
      ...emptyWidgetState(),
      warnings: [{ code: "malformed_widgets_json", message: "Browser widget metadata was malformed and has been ignored." }],
    };
  }
}

function writeMockWidgetState(
  vaultPath: string,
  state: DashboardWidgetState,
  operation: DashboardWidgetLayoutOperation | null = null,
): DashboardWidgetState {
  let allState: Record<string, DashboardWidgetState>;
  try {
    allState = JSON.parse(window.localStorage.getItem(mockWidgetsStorageKey) ?? "{}") as Record<string, DashboardWidgetState>;
  } catch {
    allState = {};
    state = {
      ...state,
      warnings: [
        ...(state.warnings ?? []),
        { code: "malformed_widgets_json", message: "Browser widget metadata was malformed and has been replaced by reset." },
      ],
      recovery_backup_path: "browser-local-storage:previous-dashboard-widgets",
    };
  }
  const nextState = {
    schema_version: 1,
    warnings: state.warnings ?? [],
    recovery_backup_path: state.recovery_backup_path ?? null,
    last_layout_operation: operation,
    instances: state.instances.map(normalizeInstance),
  };
  allState[vaultPath] = nextState;
  window.localStorage.setItem(mockWidgetsStorageKey, JSON.stringify(allState));
  return nextState;
}

function updateMockInstance(
  vaultPath: string,
  instanceId: string,
  update: (instance: DashboardWidgetInstance) => DashboardWidgetInstance,
) {
  const state = readMockWidgetState(vaultPath);
  findInstance(state, instanceId);
  return writeMockWidgetState(vaultPath, {
    ...state,
    instances: state.instances.map((instance) => (instance.instance_id === instanceId ? update(instance) : instance)),
  });
}

function findInstance(state: DashboardWidgetState, instanceId: string) {
  const instance = state.instances.find((candidate) => candidate.instance_id === instanceId);
  if (!instance) {
    throw new Error("Dashboard widget was not found.");
  }
  return instance;
}

function getMockWidgetType(widgetTypeId: string) {
  const widgetType = mockWidgetTypes().find((candidate) => candidate.id === widgetTypeId);
  if (!widgetType) {
    throw new Error(`Widget type '${widgetTypeId}' is unavailable.`);
  }
  return widgetType;
}

function defaultConfig(widgetType: WidgetTypeDefinition) {
  return Object.fromEntries(
    Object.entries(widgetType.config_schema)
      .filter(([, definition]) => "default" in definition)
      .map(([key, definition]) => [key, definition.default]),
  );
}

function normalizeInstance(instance: DashboardWidgetInstance): DashboardWidgetInstance {
  return {
    ...instance,
    config: instance.config ?? {},
    collapsed: Boolean(instance.collapsed),
    layout: normalizeLayout(instance.layout),
  };
}

function repairSparseLayoutIfNeeded(state: DashboardWidgetState): DashboardWidgetState | null {
  if (!state.instances.length) return null;

  const maxRow = Math.max(...state.instances.map((instance) => instance.layout.row));
  if (maxRow <= sparseRepairMinRow) return null;

  const compacted = compactLayout(state.instances);
  const compactedMaxRow = Math.max(...compacted.map((instance) => instance.layout.row));
  const suspicious = maxRow > compactedMaxRow + sparseRepairMargin || maxRow > state.instances.length + sparseRepairMargin;
  if (!suspicious || layoutsMatchByInstance(state.instances, compacted)) return null;

  return {
    ...state,
    instances: compacted,
    last_layout_operation: state.last_layout_operation ?? null,
  };
}

function layoutsMatchByInstance(left: DashboardWidgetInstance[], right: DashboardWidgetInstance[]) {
  return (
    left.length === right.length &&
    left.every((leftInstance) => {
      const rightInstance = right.find((candidate) => candidate.instance_id === leftInstance.instance_id);
      return rightInstance ? JSON.stringify(rightInstance.layout) === JSON.stringify(leftInstance.layout) : false;
    })
  );
}

function sameSize(left: WidgetSizeDefinition, right: WidgetSizeDefinition) {
  return left.width === right.width && left.height === right.height;
}

function readMockWorkspaceUiState(vaultPath: string): WorkspaceUiState {
  try {
    const allState = JSON.parse(window.localStorage.getItem(mockWorkspaceStateStorageKey) ?? "{}") as Record<string, WorkspaceUiState>;
    return allState[vaultPath] ?? defaultWorkspaceUiState();
  } catch {
    return defaultWorkspaceUiState();
  }
}

function writeMockWorkspaceUiState(vaultPath: string, state: WorkspaceUiState): WorkspaceUiState {
  const allState = JSON.parse(window.localStorage.getItem(mockWorkspaceStateStorageKey) ?? "{}") as Record<string, WorkspaceUiState>;
  const nextState = { ...state, updated_at: new Date().toISOString() };
  allState[vaultPath] = nextState;
  window.localStorage.setItem(mockWorkspaceStateStorageKey, JSON.stringify(allState));
  return nextState;
}

function defaultWorkspaceUiState(): WorkspaceUiState {
  return {
    schema_version: 1,
    workspace_name: "BentoLife",
    default_theme: "clean-slate",
    architect_active_tab: "modules",
    architect_sections: {
      appearance_expanded: false,
      dashboard_customization_expanded: true,
      modules_expanded: true,
      dashboard_widgets_expanded: true,
      dashboard_layout_expanded: false,
      data_graph_expanded: false,
      themes_expanded: false,
      schemas_expanded: false,
      diagnostics_expanded: false,
      recovery_expanded: false,
      modules_system_expanded: false,
      modules_starter_expanded: true,
      modules_optional_expanded: true,
    },
    updated_at: new Date().toISOString(),
  };
}
