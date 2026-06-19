import {
  compactLayout,
  normalizeLayout,
  type DashboardGridInstance,
  type DashboardGridLayout,
} from "@/domain/dashboard/layout";
import { projectCanonicalLayout, unprojectVisibleLayout } from "@/domain/dashboard/responsiveLayout";

const DASHBOARD_MAX_COLUMNS = 7;
const DIAGNOSTIC_VISIBLE_COLUMNS = [1, 2, 4, 7] as const;

export type DashboardRenderLayout<T extends DashboardGridInstance> = {
  instance: T;
  renderLayout: DashboardGridLayout;
};

export type DashboardLayoutDiagnostics = {
  instance_count: number;
  max_canonical_row_before: number;
  max_canonical_row_after: number;
  max_projected_row_1_col: number;
  max_projected_row_2_col: number;
  max_projected_row_4_col: number;
  max_projected_row_7_col: number;
  repaired: boolean;
};

export function buildDashboardRenderLayouts<T extends DashboardGridInstance>(
  instances: T[],
  visibleColumns: number,
): DashboardRenderLayout<T>[] {
  return instances.map((instance) => ({
    instance,
    renderLayout: projectDashboardRenderLayout(instance.layout, visibleColumns),
  }));
}

export function projectDashboardRenderLayout(layout: DashboardGridLayout, visibleColumns: number): DashboardGridLayout {
  return projectCanonicalLayout(layout, visibleColumns, DASHBOARD_MAX_COLUMNS);
}

export function resolveDashboardVisibleMove(
  visibleLayout: DashboardGridLayout,
  originalCanonicalLayout: DashboardGridLayout,
  visibleColumns: number,
) {
  const canonicalLayout = unprojectVisibleLayout(visibleLayout, originalCanonicalLayout, visibleColumns, DASHBOARD_MAX_COLUMNS);
  return {
    canonicalLayout,
    renderLayout: projectDashboardRenderLayout(canonicalLayout, visibleColumns),
  };
}

export function diagnoseDashboardLayouts<T extends DashboardGridInstance>(instances: T[]): DashboardLayoutDiagnostics {
  const compacted = compactLayout(instances, DASHBOARD_MAX_COLUMNS);
  const projectedRows = Object.fromEntries(
    DIAGNOSTIC_VISIBLE_COLUMNS.map((columns) => [`max_projected_row_${columns}_col`, maxProjectedRow(compacted, columns)]),
  ) as Pick<
    DashboardLayoutDiagnostics,
    "max_projected_row_1_col" | "max_projected_row_2_col" | "max_projected_row_4_col" | "max_projected_row_7_col"
  >;

  return {
    instance_count: instances.length,
    max_canonical_row_before: maxCanonicalRow(instances),
    max_canonical_row_after: maxCanonicalRow(compacted),
    repaired: !layoutsMatchByInstance(instances, compacted),
    ...projectedRows,
  };
}

function maxCanonicalRow<T extends DashboardGridInstance>(instances: T[]) {
  return instances.reduce((maxRow, instance) => Math.max(maxRow, normalizeLayout(instance.layout).row), 0);
}

function maxProjectedRow<T extends DashboardGridInstance>(instances: T[], visibleColumns: number) {
  return instances.reduce(
    (maxRow, instance) => Math.max(maxRow, projectDashboardRenderLayout(instance.layout, visibleColumns).row),
    0,
  );
}

function layoutsMatchByInstance<T extends DashboardGridInstance>(left: T[], right: T[]) {
  if (left.length !== right.length) return false;
  return left.every((instance, index) => instance.instance_id === right[index]?.instance_id && layoutsEqual(instance.layout, right[index].layout));
}

function layoutsEqual(left: DashboardGridLayout, right: DashboardGridLayout) {
  return left.column === right.column && left.row === right.row && left.width === right.width && left.height === right.height;
}
