import { normalizeLayout, type DashboardGridLayout } from "@/domain/dashboard/layout";

const DEFAULT_MAX_COLUMNS = 7;

export function projectCanonicalLayout(
  layout: DashboardGridLayout,
  visibleColumns: number,
  maxColumns = DEFAULT_MAX_COLUMNS,
): DashboardGridLayout {
  const normalizedVisibleColumns = normalizeColumnCount(visibleColumns, maxColumns);
  const normalized = normalizeLayout(layout, maxColumns);
  const projectedIndex = (normalized.row - 1) * maxColumns + (normalized.column - 1);
  const width = Math.min(normalizedVisibleColumns, normalized.width);
  const projectedColumn = (projectedIndex % normalizedVisibleColumns) + 1;

  return {
    column: Math.min(projectedColumn, Math.max(1, normalizedVisibleColumns - width + 1)),
    height: normalized.height,
    row: Math.floor(projectedIndex / normalizedVisibleColumns) + 1,
    width,
  };
}

export function unprojectVisibleLayout(
  visibleLayout: DashboardGridLayout,
  originalCanonicalLayout: DashboardGridLayout,
  visibleColumns: number,
  maxColumns = DEFAULT_MAX_COLUMNS,
): DashboardGridLayout {
  const normalizedVisibleColumns = normalizeColumnCount(visibleColumns, maxColumns);
  const original = normalizeLayout(originalCanonicalLayout, maxColumns);
  const visible = normalizeLayout(
    {
      ...visibleLayout,
      width: Math.min(normalizedVisibleColumns, Math.max(1, visibleLayout.width)),
      height: original.height,
    },
    normalizedVisibleColumns,
  );
  const visibleIndex = (visible.row - 1) * normalizedVisibleColumns + (visible.column - 1);
  const rawColumn = (visibleIndex % maxColumns) + 1;
  const maxStartColumn = Math.max(1, maxColumns - original.width + 1);

  return normalizeLayout(
    {
      column: Math.min(rawColumn, maxStartColumn),
      height: original.height,
      row: Math.floor(visibleIndex / maxColumns) + 1,
      width: original.width,
    },
    maxColumns,
  );
}

function normalizeColumnCount(value: number, maxColumns: number) {
  return Math.min(maxColumns, Math.max(1, Math.round(value || maxColumns)));
}
