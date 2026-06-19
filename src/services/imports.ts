import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import { setMockThemeSource } from "@/services/theme";
import { isTauriRuntime } from "@/services/vault";
import type { DashboardWidgetState } from "@/services/widgets";

export type ImportValidation = {
  kind: string;
  safe: boolean;
  message: string;
  source_path: string;
  normalized_name: string | null;
};

export type ImportResult = {
  validation: ImportValidation;
  stored_relative_path: string;
  bytes_copied: number;
};

const mockImportSourcePrefix = "bentolife:mockImportSource:";
const mockWidgetsStorageKey = "bentolife:mockDashboardWidgets";

export function setMockImportSource(sourcePath: string, content: string) {
  window.localStorage.setItem(`${mockImportSourcePrefix}${sourcePath}`, content);
}

export async function selectImportFile(kind: "layout" | "theme") {
  if (!isTauriRuntime()) {
    return undefined;
  }

  const selected = await open({
    directory: false,
    multiple: false,
    title: kind === "layout" ? "Select layout JSON" : "Select theme tokens",
    filters: [
      kind === "layout"
        ? { name: "Layout JSON", extensions: ["json"] }
        : { name: "CSS theme", extensions: ["css"] },
    ],
  });

  return typeof selected === "string" ? selected : undefined;
}

export async function validateLayoutImport(sourcePath: string) {
  if (!isTauriRuntime()) {
    return mockValidateImport("layout", sourcePath);
  }

  return invoke<ImportValidation>("validate_layout_import", { sourcePath });
}

export async function importLayoutFile(vaultPath: string, sourcePath: string) {
  if (!isTauriRuntime()) {
    const validation = mockValidateImport("layout", sourcePath);
    if (!validation.safe) {
      throw new Error(validation.message);
    }
    return mockImportResult(vaultPath, validation, "imports/layouts", "json");
  }

  return invoke<ImportResult>("import_layout_file", { vaultPath, sourcePath });
}

export async function validateWidgetLayoutImport(vaultPath: string, sourcePath: string) {
  if (!isTauriRuntime()) {
    return mockValidateWidgetLayoutImport(vaultPath, sourcePath);
  }

  return invoke<ImportValidation>("validate_widget_layout_import", { vaultPath, sourcePath });
}

export async function importWidgetLayoutFile(vaultPath: string, sourcePath: string) {
  if (!isTauriRuntime()) {
    const validation = mockValidateWidgetLayoutImport(vaultPath, sourcePath);
    if (!validation.safe) {
      throw new Error(validation.message);
    }
    const result = mockImportResult(vaultPath, validation, "imports/dashboard-widgets", "json");
    const state = JSON.parse(window.localStorage.getItem(`${mockImportSourcePrefix}${sourcePath}`) ?? "{}") as DashboardWidgetState;
    const allState = JSON.parse(window.localStorage.getItem(mockWidgetsStorageKey) ?? "{}") as Record<string, DashboardWidgetState>;
    allState[vaultPath] = { schema_version: 1, warnings: [], instances: state.instances ?? [] };
    window.localStorage.setItem(mockWidgetsStorageKey, JSON.stringify(allState));
    return result;
  }

  return invoke<ImportResult>("import_widget_layout_file", { vaultPath, sourcePath });
}

export async function exportWidgetLayoutFile(vaultPath: string, outputPath: string) {
  if (!isTauriRuntime()) {
    const normalizedName = normalizedImportName(outputPath, "json");
    if (!normalizedName || outputPath.split(".").pop()?.toLowerCase() !== "json") {
      throw new Error("Dashboard widget layout exports must be .json files.");
    }
    const allState = JSON.parse(window.localStorage.getItem(mockWidgetsStorageKey) ?? "{}") as Record<string, DashboardWidgetState>;
    const state = allState[vaultPath] ?? { schema_version: 1, warnings: [], instances: [] };
    const validation: ImportValidation = {
      kind: "widget_layout_export",
      safe: true,
      message: "Dashboard widget layout export is valid and data-only.",
      source_path: outputPath,
      normalized_name: normalizedName,
    };
    setMockImportSource(outputPath, JSON.stringify({ schema_version: 1, warnings: [], instances: state.instances ?? [] }, null, 2));
    return {
      validation,
      stored_relative_path: outputPath,
      bytes_copied: window.localStorage.getItem(`${mockImportSourcePrefix}${outputPath}`)?.length ?? 0,
    } satisfies ImportResult;
  }

  return invoke<ImportResult>("export_widget_layout_file", { vaultPath, outputPath });
}

export async function validateThemeImport(sourcePath: string) {
  if (!isTauriRuntime()) {
    return mockValidateImport("theme", sourcePath);
  }

  return invoke<ImportValidation>("validate_theme_import", { sourcePath });
}

export async function importThemeFile(vaultPath: string, sourcePath: string) {
  if (!isTauriRuntime()) {
    const validation = mockValidateImport("theme", sourcePath);
    if (!validation.safe) {
      throw new Error(validation.message);
    }
    const result = mockImportResult(vaultPath, validation, "themes", "css");
    setMockThemeSource(result.stored_relative_path, window.localStorage.getItem(`${mockImportSourcePrefix}${sourcePath}`) ?? "");
    return result;
  }

  return invoke<ImportResult>("import_theme_file", { vaultPath, sourcePath });
}

function mockValidateImport(kind: "layout" | "theme", sourcePath: string): ImportValidation {
  const expectedExtension = kind === "layout" ? "json" : "css";
  const extension = sourcePath.split(".").pop()?.toLowerCase() ?? "";
  if (extension !== expectedExtension || isExecutableExtension(extension)) {
    return unsafeValidation(kind, sourcePath, "Imported layouts must be .json files and imported themes must be .css files.");
  }
  const content = window.localStorage.getItem(`${mockImportSourcePrefix}${sourcePath}`);
  if (content === null) {
    return unsafeValidation(kind, sourcePath, "Import source file was not found.");
  }
  const normalizedName = normalizedImportName(sourcePath, expectedExtension);
  if (!normalizedName) {
    return unsafeValidation(kind, sourcePath, "Import file name must include at least one ASCII letter or number.");
  }
  if (kind === "layout") {
    try {
      const parsed = JSON.parse(content) as { schema_version?: number; document_id?: string; vault_relative?: boolean };
      if (parsed.schema_version !== 1 || !parsed.document_id?.startsWith("bl_doc_") || parsed.vault_relative !== true) {
        return unsafeValidation(kind, sourcePath, "Layout metadata JSON is invalid.");
      }
    } catch {
      return unsafeValidation(kind, sourcePath, "Layout metadata JSON is invalid.");
    }
  } else {
    const cssError = cssSafetyError(content);
    if (cssError) {
      return unsafeValidation(kind, sourcePath, cssError);
    }
  }
  return {
    kind,
    safe: true,
    message: `${kind} import is valid and data-only.`,
    source_path: sourcePath,
    normalized_name: normalizedName,
  };
}

function mockValidateWidgetLayoutImport(vaultPath: string, sourcePath: string): ImportValidation {
  void vaultPath;
  const extension = sourcePath.split(".").pop()?.toLowerCase() ?? "";
  if (extension !== "json" || isExecutableExtension(extension)) {
    return unsafeValidation("widget_layout", sourcePath, "Dashboard widget layout imports must be .json files.");
  }
  const content = window.localStorage.getItem(`${mockImportSourcePrefix}${sourcePath}`);
  if (content === null) {
    return unsafeValidation("widget_layout", sourcePath, "Import source file was not found.");
  }
  const normalizedName = normalizedImportName(sourcePath, "json");
  if (!normalizedName) {
    return unsafeValidation("widget_layout", sourcePath, "Import file name must include at least one ASCII letter or number.");
  }
  try {
    const state = JSON.parse(content) as DashboardWidgetState;
    const error = widgetLayoutImportError(state);
    if (error) {
      return unsafeValidation("widget_layout", sourcePath, error);
    }
  } catch {
    return unsafeValidation("widget_layout", sourcePath, "Dashboard widget layout JSON is invalid.");
  }
  return {
    kind: "widget_layout",
    safe: true,
    message: "Dashboard widget layout import is valid and data-only.",
    source_path: sourcePath,
    normalized_name: normalizedName,
  };
}

function widgetLayoutImportError(state: DashboardWidgetState) {
  if (state.schema_version !== 1 || !Array.isArray(state.instances)) {
    return "Dashboard widget layout JSON is invalid.";
  }
  const ids = new Set<string>();
  for (const instance of state.instances) {
    if (!/^bl_widget_[A-Za-z0-9_-]+$/.test(instance.instance_id) || ids.has(instance.instance_id)) {
      return "Dashboard widget layout contains an unsafe or duplicate widget instance ID.";
    }
    ids.add(instance.instance_id);
    if (!instance.widget_type.startsWith(`${instance.module_id}.`)) {
      return "Dashboard widget layout contains a widget type that does not belong to its module.";
    }
    if (
      instance.layout.column < 1 ||
      instance.layout.row < 1 ||
      instance.layout.width < 1 ||
      instance.layout.height < 1 ||
      instance.layout.column + instance.layout.width - 1 > 7 ||
      instance.layout.height > 3
    ) {
      return "Dashboard widget layout must fit the 7-column by 3-row widget grid.";
    }
    for (const [key, value] of Object.entries(instance.config ?? {})) {
      if (!/^[A-Za-z0-9_ -]+$/.test(key) || (typeof value === "string" && /^(https?:|javascript:)/i.test(value.trim()))) {
        return "Dashboard widget layout contains unsafe widget configuration.";
      }
    }
  }
  return null;
}

function mockImportResult(vaultPath: string, validation: ImportValidation, folder: string, extension: string): ImportResult {
  const content = window.localStorage.getItem(`${mockImportSourcePrefix}${validation.source_path}`) ?? "";
  const stem = validation.normalized_name?.replace(new RegExp(`\\.${extension}$`), "") ?? "import";
  const stored_relative_path = `.bentolifelayout/${folder}/${stem}-mock.${extension}`;
  void vaultPath;
  return {
    validation,
    stored_relative_path,
    bytes_copied: content.length,
  };
}

function normalizedImportName(sourcePath: string, extension: string) {
  const fileName = sourcePath.split(/[\\/]/).pop() ?? "";
  if (fileName.includes("..")) {
    return null;
  }
  const stem = fileName.replace(/\.[^.]+$/, "");
  const slug = stem
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_]+/g, "-")
    .replace(/^-|-$/g, "");
  return slug ? `${slug}.${extension}` : null;
}

function cssSafetyError(content: string) {
  const normalized = content.toLowerCase();
  const rejected = ["@import", "url(", "javascript:", "expression(", "<script", "</script", "behavior:", "-moz-binding"];
  const pattern = rejected.find((candidate) => normalized.includes(candidate));
  return pattern ? `CSS theme contains a rejected executable or remote-loading pattern: ${pattern}.` : null;
}

function isExecutableExtension(extension: string) {
  return ["js", "mjs", "cjs", "html", "svg", "wasm", "exe", "bat", "cmd", "ps1"].includes(extension);
}

function unsafeValidation(kind: string, sourcePath: string, message: string): ImportValidation {
  return {
    kind,
    safe: false,
    message,
    source_path: sourcePath,
    normalized_name: null,
  };
}
