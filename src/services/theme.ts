import { invoke } from "@tauri-apps/api/core";

import { isTauriRuntime } from "@/services/vault";

export type ThemeScope = "workspace" | "module";
export type ThemeTokenMap = Record<string, string>;

export type ThemeManifest = {
  schema_version: number;
  theme_id: string;
  scope: ThemeScope;
  module_id: string | null;
  source_path: string | null;
  tokens: ThemeTokenMap;
  active: boolean;
  updated_at: string;
};

export type ThemePreview = {
  safe: boolean;
  message: string;
  scope: ThemeScope;
  module_id: string | null;
  source_path: string;
  tokens: ThemeTokenMap;
  effective_tokens: ThemeTokenMap;
  rejected_tokens: string[];
};

export type ActiveThemeState = {
  schema_version: number;
  app_default_tokens: ThemeTokenMap;
  workspace_theme: ThemeManifest;
  module_default_tokens: Record<string, ThemeTokenMap>;
  module_themes: Record<string, ThemeManifest>;
  effective_tokens: ThemeTokenMap;
  effective_module_tokens: Record<string, ThemeTokenMap>;
  updated_at: string;
};

const mockActiveThemeKey = "bentolife:mockActiveTheme";
const mockThemeSourcePrefix = "bentolife:mockThemeSource:";

export function setMockThemeSource(sourcePath: string, content: string) {
  window.localStorage.setItem(`${mockThemeSourcePrefix}${sourcePath}`, content);
}

export async function readActiveTheme(vaultPath: string): Promise<ActiveThemeState> {
  if (!isTauriRuntime()) {
    void vaultPath;
    return readMockThemeState();
  }
  return invoke<ActiveThemeState>("read_active_theme", { vaultPath });
}

export async function previewThemeTokens(
  vaultPath: string,
  scope: ThemeScope,
  moduleId: string | null,
  sourcePath: string,
): Promise<ThemePreview> {
  if (!isTauriRuntime()) {
    void vaultPath;
    return mockPreviewTheme(scope, moduleId, sourcePath);
  }
  return invoke<ThemePreview>("preview_theme_tokens", { vaultPath, scope, moduleId, sourcePath });
}

export async function applyThemeTokens(
  vaultPath: string,
  scope: ThemeScope,
  moduleId: string | null,
  sourcePath: string,
): Promise<ActiveThemeState> {
  if (!isTauriRuntime()) {
    void vaultPath;
    const preview = mockPreviewTheme(scope, moduleId, sourcePath);
    let state = readMockThemeState();
    const manifest = createManifest(scope, moduleId, sourcePath, preview.tokens);
    if (scope === "workspace") {
      state.workspace_theme = manifest;
    } else if (moduleId) {
      state.module_themes[moduleId] = manifest;
    }
    state = hydrateThemeState(state);
    state.updated_at = new Date().toISOString();
    writeMockThemeState(state);
    return state;
  }
  return invoke<ActiveThemeState>("apply_theme_tokens", { vaultPath, scope, moduleId, sourcePath });
}

export async function rollbackTheme(
  vaultPath: string,
  scope: ThemeScope,
  moduleId: string | null,
): Promise<ActiveThemeState> {
  if (!isTauriRuntime()) {
    void vaultPath;
    let state = readMockThemeState();
    if (scope === "workspace") {
      state.workspace_theme = cleanSlateManifest("workspace", null);
    } else if (moduleId) {
      delete state.module_themes[moduleId];
    }
    state = hydrateThemeState(state);
    state.updated_at = new Date().toISOString();
    writeMockThemeState(state);
    return state;
  }
  return invoke<ActiveThemeState>("rollback_theme", { vaultPath, scope, moduleId });
}

export function effectiveTokens(state: ActiveThemeState, moduleId: string | null): ThemeTokenMap {
  return {
    ...state.app_default_tokens,
    ...state.workspace_theme.tokens,
    ...(moduleId ? state.module_default_tokens[moduleId] ?? {} : {}),
    ...(moduleId ? state.module_themes[moduleId]?.tokens ?? {} : {}),
  };
}

function mockPreviewTheme(scope: ThemeScope, moduleId: string | null, sourcePath: string): ThemePreview {
  if (scope === "module" && !moduleId) {
    throw new Error("Module theme scope requires a module ID.");
  }
  const content = window.localStorage.getItem(`${mockThemeSourcePrefix}${sourcePath}`);
  if (content === null) {
    throw new Error("Theme source file was not found.");
  }
  const tokens = sourcePath.toLowerCase().endsWith(".json") ? parseJsonTokens(content) : parseCssTokens(content);
  const state = hydrateThemeState(readMockThemeState());
  const effective = {
    ...effectiveTokens(state, scope === "module" ? moduleId : null),
    ...tokens,
  };
  return {
    safe: true,
    message: "Theme token preview is safe to apply.",
    scope,
    module_id: moduleId,
    source_path: sourcePath,
    tokens,
    effective_tokens: effective,
    rejected_tokens: [],
  };
}

function parseJsonTokens(content: string): ThemeTokenMap {
  const parsed = JSON.parse(content) as ThemeTokenMap;
  validateTokens(parsed);
  return parsed;
}

function parseCssTokens(content: string): ThemeTokenMap {
  const normalized = content.toLowerCase();
  for (const pattern of ["@import", "url(", "javascript:", "expression(", "<script", "</script", "behavior:", "-moz-binding"]) {
    if (normalized.includes(pattern)) {
      throw new Error(`CSS theme contains a rejected executable or remote-loading pattern: ${pattern}.`);
    }
  }
  const trimmed = content.trim();
  const source = trimmed.includes("{")
    ? trimmed.replace(/^:root\s*{\s*/i, "").replace(/\s*}\s*$/, "")
    : trimmed;
  if (trimmed.includes("{") && !trimmed.toLowerCase().startsWith(":root")) {
    throw new Error("CSS token themes may only use a :root custom-property block.");
  }
  const tokens: ThemeTokenMap = {};
  for (const declaration of source.split(";")) {
    const item = declaration.trim();
    if (!item) continue;
    const [name, ...valueParts] = item.split(":");
    const value = valueParts.join(":").trim();
    if (!name?.trim().startsWith("--") || !value) {
      throw new Error("CSS token declarations must use --token: value syntax.");
    }
    tokens[name.trim()] = value;
  }
  validateTokens(tokens);
  return tokens;
}

function validateTokens(tokens: ThemeTokenMap) {
  const allowed = new Set([
    "--background",
    "--foreground",
    "--card",
    "--card-foreground",
    "--popover",
    "--popover-foreground",
    "--primary",
    "--primary-foreground",
    "--secondary",
    "--secondary-foreground",
    "--muted",
    "--muted-foreground",
    "--accent",
    "--accent-foreground",
    "--destructive",
    "--destructive-foreground",
    "--border",
    "--input",
    "--ring",
    "--sage",
    "--sage-foreground",
    "--soft-blue",
    "--soft-blue-foreground",
    "--amber-note",
    "--amber-note-foreground",
    "--shadow-soft",
    "--shadow-lifted",
    "--habit-progress-height",
    "--habit-streak-emphasis",
    "--habit-completed-state",
    "--todo-overdue-state",
    "--todo-priority-emphasis",
    "--todo-completed-state",
    "--contact-relationship-chip",
  ]);
  for (const [token, value] of Object.entries(tokens)) {
    if (!allowed.has(token)) {
      throw new Error(`Theme token ${token} is not allowlisted.`);
    }
    if (/[{};<>@]|url\(|javascript:|expression\(/i.test(value)) {
      throw new Error("Theme token value contains a rejected pattern.");
    }
  }
}

function readMockThemeState(): ActiveThemeState {
  const serialized = window.localStorage.getItem(mockActiveThemeKey);
  if (serialized) {
    try {
      return hydrateThemeState(JSON.parse(serialized) as Partial<ActiveThemeState>);
    } catch {
      // Fall through to a clean state.
    }
  }
  return hydrateThemeState({
    schema_version: 1,
    workspace_theme: cleanSlateManifest("workspace", null),
    module_themes: {},
    updated_at: "mock",
  });
}

function writeMockThemeState(state: ActiveThemeState) {
  window.localStorage.setItem(mockActiveThemeKey, JSON.stringify(state));
}

function createManifest(scope: ThemeScope, moduleId: string | null, sourcePath: string | null, tokens: ThemeTokenMap): ThemeManifest {
  return {
    schema_version: 1,
    theme_id: moduleId ? `${scope}-${moduleId}` : scope,
    scope,
    module_id: moduleId,
    source_path: sourcePath,
    tokens,
    active: true,
    updated_at: new Date().toISOString(),
  };
}

function cleanSlateManifest(scope: ThemeScope, moduleId: string | null): ThemeManifest {
  return createManifest(scope, moduleId, null, {});
}

function hydrateThemeState(state: Partial<ActiveThemeState>): ActiveThemeState {
  const next: ActiveThemeState = {
    schema_version: 1,
    app_default_tokens: appDefaultTokens(),
    workspace_theme: state.workspace_theme ?? cleanSlateManifest("workspace", null),
    module_default_tokens: moduleDefaultTokens(),
    module_themes: state.module_themes ?? {},
    effective_tokens: {},
    effective_module_tokens: {},
    updated_at: state.updated_at ?? new Date().toISOString(),
  };
  next.effective_tokens = effectiveTokens(next, null);
  const moduleIds = new Set([...Object.keys(next.module_default_tokens), ...Object.keys(next.module_themes)]);
  next.effective_module_tokens = Object.fromEntries(
    [...moduleIds].map((moduleId) => [moduleId, effectiveTokens(next, moduleId)]),
  );
  return next;
}

function appDefaultTokens(): ThemeTokenMap {
  return {
    "--background": "#f6f7f4",
    "--foreground": "#202521",
    "--card": "#ffffff",
    "--card-foreground": "#202521",
    "--popover": "#ffffff",
    "--popover-foreground": "#202521",
    "--primary": "#335c4a",
    "--primary-foreground": "#f8fbf7",
    "--secondary": "#e8eee9",
    "--secondary-foreground": "#26342d",
    "--muted": "#edf0eb",
    "--muted-foreground": "#66716a",
    "--accent": "#e5edf4",
    "--accent-foreground": "#213346",
    "--destructive": "#9f3a3a",
    "--destructive-foreground": "#fff7f7",
    "--border": "#dfe4dc",
    "--input": "#d9e0d7",
    "--ring": "#6f927f",
    "--sage": "#7e9d86",
    "--sage-foreground": "#213528",
    "--soft-blue": "#7d98ad",
    "--soft-blue-foreground": "#203343",
    "--amber-note": "#d7b46a",
    "--amber-note-foreground": "#403112",
    "--shadow-soft": "0 16px 40px rgb(39 50 42 / 0.08)",
    "--shadow-lifted": "0 22px 55px rgb(39 50 42 / 0.12)",
  };
}

function moduleDefaultTokens(): Record<string, ThemeTokenMap> {
  return {
    habits: {
      "--habit-progress-height": "0.375rem",
      "--habit-streak-emphasis": "#335c4a",
      "--habit-completed-state": "#7e9d86",
    },
    todos: {
      "--todo-overdue-state": "#9f3a3a",
      "--todo-priority-emphasis": "#d7b46a",
      "--todo-completed-state": "#7e9d86",
    },
    contacts: {
      "--contact-relationship-chip": "#e5edf4",
    },
  };
}
