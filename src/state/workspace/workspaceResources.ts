export type WorkspaceResourceKey =
  | "scan"
  | "dashboardHub"
  | "notes"
  | "todos"
  | "contacts"
  | "habits"
  | "navigator"
  | "recovery"
  | "theme"
  | "moduleRegistry"
  | "widgetRegistry"
  | "dashboardWidgets"
  | "workspaceUiState";

export type ResourceState<T> =
  | {
      status: "idle" | "loading" | "ready";
      data: T;
      error: null;
      warnings: string[];
    }
  | {
      status: "degraded";
      data: T;
      error: string;
      warnings: string[];
    };

export type WorkspaceResourceErrors = Partial<Record<WorkspaceResourceKey, string>>;

export function readyResource<T>(data: T, warnings: string[] = []): ResourceState<T> {
  return { status: "ready", data, error: null, warnings };
}

export function degradedResource<T>(data: T, error: string, warnings: string[] = []): ResourceState<T> {
  return { status: "degraded", data, error, warnings };
}

export async function loadWorkspaceResource<T>(
  load: () => Promise<T>,
  fallback: T,
): Promise<ResourceState<T>> {
  try {
    return readyResource(await load());
  } catch (error) {
    return degradedResource(fallback, error instanceof Error ? error.message : String(error));
  }
}
