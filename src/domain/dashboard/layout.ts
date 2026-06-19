export type DashboardGridLayout = {
  column: number;
  row: number;
  width: number;
  height: number;
};

export type DashboardGridInstance = {
  instance_id?: string;
  layout: DashboardGridLayout;
};

const DASHBOARD_MAX_COLUMNS = 7;
const DASHBOARD_MAX_HEIGHT = 3;

export function findNextAvailableLayout(
  instances: DashboardGridInstance[],
  width: number,
  height: number,
  maxColumns = DASHBOARD_MAX_COLUMNS,
): DashboardGridLayout {
  const normalizedWidth = clampInt(width, 1, maxColumns);
  const normalizedHeight = clampInt(height, 1, DASHBOARD_MAX_HEIGHT);
  const maxStartColumn = Math.max(1, maxColumns - normalizedWidth + 1);

  for (let row = 1; ; row += 1) {
    for (let column = 1; column <= maxStartColumn; column += 1) {
      const candidate = normalizeLayout({ column, row, width: normalizedWidth, height: normalizedHeight }, maxColumns);
      if (!instances.some((instance) => layoutsOverlap(candidate, instance.layout))) {
        return candidate;
      }
    }
  }
}

export function compactLayout<T extends DashboardGridInstance>(instances: T[], maxColumns = DASHBOARD_MAX_COLUMNS): T[] {
  const placed: T[] = [];
  const ordered = [...instances].sort((left, right) => {
    const rowDelta = left.layout.row - right.layout.row;
    if (rowDelta !== 0) return rowDelta;
    return left.layout.column - right.layout.column;
  });

  for (const instance of ordered) {
    const layout = findNextAvailableLayout(placed, instance.layout.width, instance.layout.height, maxColumns);
    placed.push({ ...instance, layout });
  }

  return placed;
}

export function normalizeLayout(layout: DashboardGridLayout, maxColumns = DASHBOARD_MAX_COLUMNS): DashboardGridLayout {
  const column = clampInt(layout.column, 1, maxColumns);
  const width = clampInt(layout.width, 1, maxColumns);
  return {
    column,
    row: Math.max(1, Math.round(layout.row || 1)),
    width: Math.min(width, maxColumns - column + 1),
    height: clampInt(layout.height, 1, DASHBOARD_MAX_HEIGHT),
  };
}

export function layoutsOverlap(left: DashboardGridLayout, right: DashboardGridLayout) {
  const normalizedLeft = normalizeLayout(left);
  const normalizedRight = normalizeLayout(right);
  const leftEndColumn = normalizedLeft.column + normalizedLeft.width - 1;
  const rightEndColumn = normalizedRight.column + normalizedRight.width - 1;
  const leftEndRow = normalizedLeft.row + normalizedLeft.height - 1;
  const rightEndRow = normalizedRight.row + normalizedRight.height - 1;

  return (
    normalizedLeft.column <= rightEndColumn &&
    leftEndColumn >= normalizedRight.column &&
    normalizedLeft.row <= rightEndRow &&
    leftEndRow >= normalizedRight.row
  );
}

export function overlappingInstanceIds<T extends DashboardGridInstance>(
  instances: T[],
  activeInstanceId?: string,
): string[] {
  const overlaps = new Set<string>();
  for (let leftIndex = 0; leftIndex < instances.length; leftIndex += 1) {
    for (let rightIndex = leftIndex + 1; rightIndex < instances.length; rightIndex += 1) {
      const left = instances[leftIndex];
      const right = instances[rightIndex];
      if (!layoutsOverlap(left.layout, right.layout)) continue;
      if (!activeInstanceId || left.instance_id === activeInstanceId) {
        overlaps.add(right.instance_id ?? `${rightIndex}`);
      }
      if (!activeInstanceId || right.instance_id === activeInstanceId) {
        overlaps.add(left.instance_id ?? `${leftIndex}`);
      }
    }
  }
  return [...overlaps].sort();
}

export function assertNoLayoutOverlap<T extends DashboardGridInstance>(
  instances: T[],
  activeInstanceId?: string,
): void {
  const affectedWidgetIds = overlappingInstanceIds(instances, activeInstanceId);
  if (affectedWidgetIds.length) {
    throw new DashboardLayoutBlockedError(affectedWidgetIds);
  }
}

export class DashboardLayoutBlockedError extends Error {
  affectedWidgetIds: string[];

  constructor(affectedWidgetIds: string[]) {
    super("This position overlaps another widget. Try another spot or use Compact layout.");
    this.name = "DashboardLayoutBlockedError";
    this.affectedWidgetIds = affectedWidgetIds;
  }
}

function clampInt(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, Math.round(value || min)));
}
