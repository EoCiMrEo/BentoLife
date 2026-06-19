import type { VaultInspection } from "@/services/vault";

export type VaultStage =
  | "checking"
  | "missing"
  | "ready"
  | "needs_repair"
  | "needs_reset_guidance"
  | "invalid"
  | "error";

export type VaultSnapshot = {
  defaultPath: string;
  error?: string;
  inspection?: VaultInspection;
  selectedPath?: string;
  stage: VaultStage;
};

export function deriveVaultStage(inspection?: VaultInspection): VaultStage {
  if (!inspection) {
    return "missing";
  }

  if (inspection.state === "ready") {
    return "ready";
  }

  if (inspection.state === "layout_missing" || inspection.state === "scaffold_incomplete") {
    return "needs_repair";
  }

  if (inspection.state === "older_version_detected") {
    return "needs_reset_guidance";
  }

  if (inspection.state === "invalid_path" || inspection.state === "blocked") {
    return "invalid";
  }

  return "missing";
}

export function createVaultSnapshot(params: {
  defaultPath: string;
  error?: string;
  inspection?: VaultInspection;
  selectedPath?: string;
}): VaultSnapshot {
  return {
    ...params,
    stage: params.error ? "error" : deriveVaultStage(params.inspection),
  };
}
