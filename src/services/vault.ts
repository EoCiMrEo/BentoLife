import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export type VaultState =
  | "missing"
  | "invalid_path"
  | "older_version_detected"
  | "layout_missing"
  | "scaffold_incomplete"
  | "ready"
  | "blocked";

export type VaultInspection = {
  path: string;
  state: VaultState;
  exists: boolean;
  is_bentolife_vault: boolean;
  layout_exists: boolean;
  older_version_detected: boolean;
  missing_paths: string[];
  message: string;
};

const selectedVaultStorageKey = "bentolife:selectedVaultPath";
const mockVaultStateStorageKey = "bentolife:mockVaultState";
export const browserMockDefaultVaultPath =
  requireEnv(import.meta.env.VITE_BENTOLIFE_BROWSER_MOCK_VAULT_PATH, "VITE_BENTOLIFE_BROWSER_MOCK_VAULT_PATH");

function requireEnv(value: string | undefined, name: string) {
  if (!value) {
    throw new Error(`${name} must be configured for browser fallback mode.`);
  }

  return value;
}

export function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function getStoredVaultPath() {
  return window.localStorage.getItem(selectedVaultStorageKey);
}

export function storeVaultPath(path: string) {
  window.localStorage.setItem(selectedVaultStorageKey, path);
}

export function clearStoredVaultPath() {
  window.localStorage.removeItem(selectedVaultStorageKey);
}

export function setMockVaultState(state: VaultState) {
  window.localStorage.setItem(mockVaultStateStorageKey, state);
}

export async function getDefaultVaultPath() {
  if (!isTauriRuntime()) {
    return browserMockDefaultVaultPath;
  }

  return invoke<string>("get_default_vault_path");
}

export async function inspectVault(path: string) {
  if (!isTauriRuntime()) {
    return mockInspection(path, getMockVaultState(path));
  }

  return invoke<VaultInspection>("inspect_vault", { path });
}

export async function createDefaultVault() {
  if (!isTauriRuntime()) {
    const inspection = mockInspection(browserMockDefaultVaultPath, "ready");
    setMockVaultState("ready");
    storeVaultPath(inspection.path);
    return inspection;
  }

  return invoke<VaultInspection>("create_default_vault");
}

export async function createVaultAt(path: string) {
  if (!isTauriRuntime()) {
    const inspection = mockInspection(path, "ready");
    setMockVaultState("ready");
    storeVaultPath(inspection.path);
    return inspection;
  }

  return invoke<VaultInspection>("create_vault_at", { path });
}

export async function repairVaultStructure(path: string) {
  if (!isTauriRuntime()) {
    const inspection = mockInspection(path, "ready");
    setMockVaultState("ready");
    return inspection;
  }

  return invoke<VaultInspection>("repair_vault_structure", {
    path,
    confirmationToken: "repair-vault-structure",
  });
}

export async function selectVaultFolder() {
  if (!isTauriRuntime()) {
    return browserMockDefaultVaultPath;
  }

  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select .bentolifevault",
  });

  return typeof selected === "string" ? selected : undefined;
}

export async function selectFolder(title: string) {
  if (!isTauriRuntime()) {
    return browserMockDefaultVaultPath;
  }

  const selected = await open({
    directory: true,
    multiple: false,
    title,
  });

  return typeof selected === "string" ? selected : undefined;
}

function getMockVaultState(path: string): VaultState {
  const state = window.localStorage.getItem(mockVaultStateStorageKey) as VaultState | null;

  if (state) {
    return state;
  }

  return path.endsWith(".bentolifevault") ? "missing" : "invalid_path";
}

function mockInspection(path: string, state: VaultState): VaultInspection {
  const isVaultPath = path.endsWith(".bentolifevault");
  const ready = state === "ready";
  const layoutMissing = state === "layout_missing";
  const scaffoldIncomplete = state === "scaffold_incomplete";
  const olderVersionDetected = state === "older_version_detected";
  const missing_paths =
    state === "missing"
      ? ["assets", ".bentolifelayout"]
      : layoutMissing
        ? [".bentolifelayout"]
        : scaffoldIncomplete
          ? [".bentolifelayout/index.json"]
          : [];

  return {
    path,
    state: isVaultPath ? state : "invalid_path",
    exists: ready || layoutMissing || scaffoldIncomplete || olderVersionDetected,
    is_bentolife_vault: isVaultPath,
    layout_exists: ready || scaffoldIncomplete,
    older_version_detected: olderVersionDetected,
    missing_paths,
    message: messageForState(isVaultPath ? state : "invalid_path"),
  };
}

function messageForState(state: VaultState) {
  switch (state) {
    case "ready":
      return "Vault is ready.";
    case "layout_missing":
      return "Markdown content may be safe, but .bentolifelayout is missing.";
    case "older_version_detected":
      return "Older BentoLife vault paths were detected. Back up or snapshot this vault, then create a fresh V3 vault and import copied content explicitly.";
    case "scaffold_incomplete":
      return "Vault exists but required metadata files are missing.";
    case "invalid_path":
      return "Select or create the .bentolifevault folder itself.";
    case "blocked":
      return "BentoLife cannot use this path.";
    case "missing":
    default:
      return "Vault folder has not been created yet.";
  }
}
