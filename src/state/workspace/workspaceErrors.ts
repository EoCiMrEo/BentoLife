import type { WorkspaceResourceErrors, WorkspaceResourceKey } from "@/state/workspace/workspaceResources";

export function setResourceError(
  errors: WorkspaceResourceErrors,
  resource: WorkspaceResourceKey,
  error: string | null,
): WorkspaceResourceErrors {
  if (!error) {
    const { [resource]: _removed, ...rest } = errors;
    return rest;
  }
  return { ...errors, [resource]: error };
}

export function resourceErrorMessage(
  errors: WorkspaceResourceErrors,
  resource: WorkspaceResourceKey,
  workspaceError: string | null,
): string | null {
  return errors[resource] ?? workspaceError;
}
