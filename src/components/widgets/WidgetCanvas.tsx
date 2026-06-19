import { Move, Pencil, Plus } from "lucide-react";
import { startTransition, useCallback, useLayoutEffect, useMemo, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Empty } from "@/components/ui/empty";
import { WidgetCard } from "@/components/widgets/WidgetCard";
import type { RegistryState } from "@/services/backendCore";
import type { WidgetInteractionHandlers, WidgetRenderContext } from "@/services/widgetRendererRegistry";
import { useI18n } from "@/i18n";
import { type AppView, type FocusTarget } from "@/state/navigation";
import { buildDashboardRenderLayouts } from "@/domain/dashboard/layoutEngine";
import type { DashboardGridLayout } from "@/domain/dashboard/layout";
import {
  isWidgetActive,
  type DashboardWidgetInstance,
  type DashboardWidgetState,
  type UpdateDashboardWidgetInput,
  type WidgetSizeDefinition,
  type WidgetTypeDefinition,
} from "@/services/widgets";

export type WidgetActions = {
  addWidget: (widgetType: WidgetTypeDefinition) => void;
  compactWidgets: () => void;
  duplicateWidget: (instanceId: string) => void;
  moveWidget: (instance: DashboardWidgetInstance, direction: "up" | "down") => void;
  moveWidgetTo: (instance: DashboardWidgetInstance, layout: DashboardGridLayout) => void;
  removeWidget: (instanceId: string) => void;
  resizeWidget: (instanceId: string, size: WidgetSizeDefinition) => void;
  toggleCollapsed: (instance: DashboardWidgetInstance) => void;
  updateWidget: (instanceId: string, input: UpdateDashboardWidgetInput) => void;
};

export type WidgetNavigate = (view: AppView, label?: string, options?: Partial<FocusTarget>) => void;

type WidgetCanvasProps = {
  actions: WidgetActions;
  customMode: boolean;
  context: WidgetRenderContext;
  interactions: WidgetInteractionHandlers;
  moduleRegistry: RegistryState | null;
  onEnableModule: (moduleId: string) => void;
  onNavigate: WidgetNavigate;
  onOpenArchitect: () => void;
  onRetry?: () => void;
  setCustomMode: (customMode: boolean) => void;
  state: DashboardWidgetState | null;
  widgetTypes: WidgetTypeDefinition[];
};

export function WidgetCanvas({
  actions,
  customMode,
  context,
  interactions,
  moduleRegistry,
  onEnableModule,
  onNavigate,
  onOpenArchitect,
  onRetry,
  setCustomMode,
  state,
  widgetTypes,
}: WidgetCanvasProps) {
  const { t } = useI18n();
  const [pickerOpen, setPickerOpen] = useState(false);
  const instances = state?.instances ?? [];
  const activeInstances = useMemo(
    () => instances.filter((instance) => isWidgetActive(instance, moduleRegistry)),
    [instances, moduleRegistry],
  );
  const typeById = useMemo(() => new Map(widgetTypes.map((widgetType) => [widgetType.id, widgetType])), [widgetTypes]);
  const { gridRef, visibleColumns } = useMeasuredWidgetColumns();
  const renderInstances = useMemo(
    () => buildDashboardRenderLayouts(activeInstances, visibleColumns),
    [activeInstances, visibleColumns],
  );
  const neighborRenderLayoutsById = useMemo(() => {
    const byId = new Map<string, Array<{ instance_id: string; layout: DashboardGridLayout }>>();
    if (!customMode) {
      return byId;
    }
    for (const item of renderInstances) {
      byId.set(
        item.instance.instance_id,
        renderInstances
          .filter((candidate) => candidate.instance.instance_id !== item.instance.instance_id)
          .map((candidate) => ({
            instance_id: candidate.instance.instance_id,
            layout: candidate.renderLayout,
          })),
      );
    }
    return byId;
  }, [customMode, renderInstances]);

  if (!activeInstances.length) {
    return (
      <>
        <Empty
          className="min-h-[26rem]"
          title={t("dashboard.empty.title")}
          description={t("dashboard.empty.description")}
        >
          <Button onClick={() => setPickerOpen(true)}>
            <Plus data-icon="inline-start" />
            {t("dashboard.empty.addFirst")}
          </Button>
          <Button onClick={onOpenArchitect} variant="outline">
            <Pencil data-icon="inline-start" />
            {t("nav.architect")}
          </Button>
        </Empty>
        <WidgetPicker
          moduleRegistry={moduleRegistry}
          onAdd={(widgetType) => {
            actions.addWidget(widgetType);
            setPickerOpen(false);
          }}
          onEnableModule={onEnableModule}
          onOpenArchitect={onOpenArchitect}
          onRetry={onRetry}
          open={pickerOpen}
          setOpen={setPickerOpen}
          widgetTypes={widgetTypes}
        />
      </>
    );
  }

  return (
    <section className="flex flex-col gap-4" aria-label={t("dashboard.widgets.aria")}>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-xl font-semibold">{t("dashboard.title")}</h1>
          <p className="text-sm text-muted-foreground">{t("dashboard.description")}</p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            aria-pressed={customMode}
            onClick={() => setCustomMode(!customMode)}
            type="button"
            variant={customMode ? "default" : "outline"}
          >
            <Move data-icon="inline-start" />
            {t("dashboard.customMode")}
          </Button>
          <Button onClick={() => setPickerOpen(true)} variant="outline">
            <Plus data-icon="inline-start" />
            {t("dashboard.addWidget")}
          </Button>
          <Button disabled={!instances.length} onClick={actions.compactWidgets} variant="outline">
            <Move data-icon="inline-start" />
            {t("dashboard.compactLayout")}
          </Button>
        </div>
      </div>
      <div className="grid auto-rows-[minmax(12rem,auto)] gap-3 md:grid-cols-2 xl:grid-cols-4 2xl:grid-cols-7" data-widget-grid ref={gridRef}>
        {renderInstances.map(({ instance, renderLayout }) => (
          <WidgetCard
            actions={actions}
            customMode={customMode}
            context={context}
            interactions={interactions}
            instance={instance}
            key={instance.instance_id}
            neighborRenderLayouts={neighborRenderLayoutsById.get(instance.instance_id) ?? []}
            onNavigate={onNavigate}
            renderLayout={renderLayout}
            visibleColumns={visibleColumns}
            widgetType={typeById.get(instance.widget_type)}
          />
        ))}
      </div>
      <WidgetPicker
        moduleRegistry={moduleRegistry}
        onAdd={(widgetType) => {
          actions.addWidget(widgetType);
          setPickerOpen(false);
        }}
        onEnableModule={onEnableModule}
        onOpenArchitect={onOpenArchitect}
        onRetry={onRetry}
        open={pickerOpen}
        setOpen={setPickerOpen}
        widgetTypes={widgetTypes}
      />
    </section>
  );
}

function useMeasuredWidgetColumns() {
  const [gridElement, setGridElement] = useState<HTMLDivElement | null>(null);
  const [columns, setColumns] = useState(getViewportWidgetColumns);
  const gridRef = useCallback((element: HTMLDivElement | null) => {
    setGridElement(element);
  }, []);

  useLayoutEffect(() => {
    if (!gridElement || typeof window === "undefined") {
      return undefined;
    }

    const updateColumns = (next: number, deferred = false) => {
      const apply = () => setColumns((current) => (next === current ? current : next));
      if (deferred) {
        startTransition(apply);
        return;
      }
      apply();
    };

    updateColumns(measureGridColumns(gridElement));

    if (typeof window.matchMedia === "function") {
      const breakpointQueries = [
        { columns: 7, query: window.matchMedia("(min-width: 1536px)") },
        { columns: 4, query: window.matchMedia("(min-width: 1280px)") },
        { columns: 2, query: window.matchMedia("(min-width: 768px)") },
      ];
      let breakpointTimer = 0;
      const updateFromBreakpoints = () => {
        const nextColumns = breakpointQueries.find(({ query }) => query.matches)?.columns ?? 1;
        if (breakpointTimer) {
          window.clearTimeout(breakpointTimer);
        }
        breakpointTimer = window.setTimeout(() => {
          breakpointTimer = 0;
          updateColumns(nextColumns, true);
        }, 180);
      };

      breakpointQueries.forEach(({ query }) => {
        query.addEventListener("change", updateFromBreakpoints);
      });

      return () => {
        if (breakpointTimer) {
          window.clearTimeout(breakpointTimer);
        }
        breakpointQueries.forEach(({ query }) => {
          query.removeEventListener("change", updateFromBreakpoints);
        });
      };
    }

    let frame = 0;
    const measure = () => {
      frame = 0;
      const next = measureGridColumns(gridElement);
      updateColumns(next);
    };
    const scheduleMeasure = () => {
      if (frame) return;
      frame = window.requestAnimationFrame(measure);
    };

    window.addEventListener("resize", scheduleMeasure, { passive: true });

    return () => {
      window.removeEventListener("resize", scheduleMeasure);
      if (frame) {
        window.cancelAnimationFrame(frame);
      }
    };
  }, [gridElement]);

  return { gridRef, visibleColumns: columns };
}

function measureGridColumns(element: HTMLElement) {
  const measured = countGridTemplateColumns(window.getComputedStyle(element).gridTemplateColumns);
  if (measured > 0) {
    return Math.min(7, measured);
  }
  const width = element.getBoundingClientRect().width || window.innerWidth;
  return columnsForWidth(width);
}

function getViewportWidgetColumns() {
  if (typeof window === "undefined") return 7;
  return columnsForWidth(window.innerWidth);
}

function columnsForWidth(width: number) {
  if (width >= 1536) return 7;
  if (width >= 1280) return 4;
  if (width >= 768) return 2;
  return 1;
}

function countGridTemplateColumns(template: string) {
  const value = template.trim();
  if (!value || value === "none") return 0;
  const repeat = value.match(/^repeat\((\d+),/);
  if (repeat) return Number(repeat[1]);

  let columns = 0;
  let depth = 0;
  let hasToken = false;
  for (const character of value) {
    if (character === "(") depth += 1;
    if (character === ")") depth = Math.max(0, depth - 1);
    if (/\s/.test(character) && depth === 0) {
      if (hasToken) columns += 1;
      hasToken = false;
      continue;
    }
    hasToken = true;
  }
  return hasToken ? columns + 1 : columns;
}

export function WidgetPicker({
  moduleRegistry,
  onAdd,
  onEnableModule,
  onOpenArchitect,
  onRetry,
  open,
  setOpen,
  widgetTypes,
}: {
  moduleRegistry: RegistryState | null;
  onAdd: (widgetType: WidgetTypeDefinition) => void;
  onEnableModule?: (moduleId: string) => void;
  onOpenArchitect?: () => void;
  onRetry?: () => void;
  open: boolean;
  setOpen: (open: boolean) => void;
  widgetTypes: WidgetTypeDefinition[];
}) {
  const { t } = useI18n();
  const modules = moduleRegistry?.modules ?? [];
  const grouped = widgetTypes.reduce<Record<string, WidgetTypeDefinition[]>>((groups, widgetType) => {
    groups[widgetType.module_id] = [...(groups[widgetType.module_id] ?? []), widgetType];
    return groups;
  }, {});
  const groupedEntries = Object.entries(grouped);

  return (
    <Dialog onOpenChange={setOpen} open={open}>
      <DialogContent className="max-h-[86vh] overflow-auto">
        <DialogHeader>
          <DialogTitle>{t("widgets.picker.title")}</DialogTitle>
          <DialogDescription>{t("widgets.picker.description")}</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-4">
          {!groupedEntries.length ? (
            <div className="rounded-md border border-dashed border-border bg-muted/35 p-4 text-sm">
              <p className="font-medium">{t("widgets.picker.empty.title")}</p>
              <p className="mt-1 text-muted-foreground">{t("widgets.picker.empty.description")}</p>
              <div className="mt-3 flex flex-wrap gap-2">
                {onRetry ? (
                  <Button onClick={onRetry} size="sm" type="button" variant="outline">
                    {t("app.actions.rescan")}
                  </Button>
                ) : null}
                {onOpenArchitect ? (
                  <Button
                    onClick={() => {
                      setOpen(false);
                      onOpenArchitect();
                    }}
                    size="sm"
                    type="button"
                  >
                    {t("nav.architect")}
                  </Button>
                ) : null}
              </div>
            </div>
          ) : null}
          {groupedEntries.map(([moduleId, types]) => {
            const module = modules.find((candidate) => candidate.id === moduleId);
            const enabled = module ? module.available && module.installed && module.enabled : true;
            return (
              <div className="rounded-md border border-border p-3" data-widget-module={moduleId} key={moduleId}>
                <div className="mb-3 flex items-center justify-between gap-3">
                  <h3 className="text-sm font-semibold">{module?.display_name ?? moduleId}</h3>
                  {enabled ? (
                    <Badge variant="secondary">{t("widgets.picker.enabled")}</Badge>
                  ) : (
                    <Button
                      disabled={!onEnableModule || !module?.installed}
                      onClick={() => onEnableModule?.(moduleId)}
                      size="sm"
                      type="button"
                      variant="outline"
                    >
                      {t("widgets.picker.enableModule")}
                    </Button>
                  )}
                </div>
                <div className="grid gap-2">
                  {types.map((widgetType) => (
                    <Button
                      className="h-auto justify-between whitespace-normal px-3 py-2 text-left"
                      disabled={!enabled}
                      key={widgetType.id}
                      onClick={() => onAdd(widgetType)}
                      variant="outline"
                    >
                      <span className="min-w-0">
                        <span className="block font-medium">{widgetType.label}</span>
                        <span className="block text-xs text-muted-foreground">{widgetType.description}</span>
                      </span>
                      <Plus className="size-4" aria-hidden="true" />
                    </Button>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}
