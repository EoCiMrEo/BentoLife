import { BookOpen, CheckSquare, Copy, Maximize2, MoreHorizontal, Move, Pencil, Trash2, ChevronsDown, ChevronsUp, Users, Flame } from "lucide-react";
import { memo, type PointerEvent, useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { WidgetIconHeader, WidgetModuleBadge } from "@/components/widgets/WidgetPrimitives";
import { WidgetErrorBoundary } from "@/components/system/WidgetErrorBoundary";
import { useI18n } from "@/i18n";
import { layoutsOverlap, type DashboardGridLayout } from "@/domain/dashboard/layout";
import {
  projectDashboardRenderLayout,
  resolveDashboardVisibleMove,
} from "@/domain/dashboard/layoutEngine";
import type { WidgetInteractionHandlers, WidgetRenderContext } from "@/services/widgetRendererRegistry";
import { renderWidget } from "@/services/widgetRendererRegistry";
import {
  dashboardWidgetTitle,
  type DashboardWidgetInstance,
  type WidgetConfigFieldDefinition,
  type WidgetSizeDefinition,
  type WidgetTypeDefinition,
} from "@/services/widgets";
import { viewForModule } from "@/state/navigation";
import type { WidgetActions, WidgetNavigate } from "@/components/widgets/WidgetCanvas";

export const WidgetCard = memo(function WidgetCard({
  actions,
  customMode = false,
  context,
  interactions,
  instance,
  neighborRenderLayouts = [],
  onNavigate,
  renderLayout,
  visibleColumns = 7,
  widgetType,
}: {
  actions: WidgetActions;
  customMode?: boolean;
  context: WidgetRenderContext;
  interactions: WidgetInteractionHandlers;
  instance: DashboardWidgetInstance;
  neighborRenderLayouts?: Array<{ instance_id: string; layout: DashboardGridLayout }>;
  onNavigate?: WidgetNavigate;
  renderLayout?: DashboardGridLayout;
  visibleColumns?: number;
  widgetType?: WidgetTypeDefinition;
}) {
  const { t } = useI18n();
  const projectedLayout = useMemo(
    () => renderLayout ?? projectDashboardRenderLayout(instance.layout, visibleColumns),
    [instance.layout, renderLayout, visibleColumns],
  );
  const columnSpan = projectedLayout.width;
  const rowSpan = projectedLayout.height;
  const columnStart = projectedLayout.column;
  const rowStart = projectedLayout.row;
  const [pointerState, setPointerState] = useState<WidgetPointerState | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [removeOpen, setRemoveOpen] = useState(false);
  const pointerStateRef = useRef<WidgetPointerState | null>(null);
  const pointerFrameRef = useRef(0);
  const pendingPointerRef = useRef<WidgetPointerPoint | null>(null);
  const title = dashboardWidgetTitle(instance, widgetType);
  const moduleId = instance.module_id || widgetType?.module_id;
  const extraHeaderBadges = widgetHeaderBadges(instance, context, t);
  const projectedNeighborLayouts = customMode ? neighborRenderLayouts : [];
  const startWidgetAction = (trigger: HTMLButtonElement, pointerId: number, clientX: number, clientY: number) => {
    if (!customMode || pointerStateRef.current) {
      return;
    }
    const card = trigger.closest<HTMLElement>("[data-widget-card]");
    const grid = card?.closest<HTMLElement>("[data-widget-grid]");
    const cardRect = card?.getBoundingClientRect();
    const gridRect = grid?.getBoundingClientRect();
    const nextPointerState = {
      cardHeight: Math.max(cardRect?.height ?? 192, 96),
      columnWidth: Math.max(gridRect ? gridRect.width / visibleColumns : cardRect?.width ?? 220, 1),
      pointerId,
      previewColumn: columnStart,
      previewRow: rowStart,
      previewBlocked: false,
      startColumn: columnStart,
      startRow: rowStart,
      startX: clientX,
      startY: clientY,
    };
    pointerStateRef.current = nextPointerState;
    setPointerState(nextPointerState);
  };

  const startPointerAction = (event: PointerEvent<HTMLButtonElement>) => {
    if (!customMode || event.button > 0) {
      return;
    }
    event.preventDefault();
    event.currentTarget.setPointerCapture?.(event.pointerId);
    startWidgetAction(event.currentTarget, event.pointerId, event.clientX, event.clientY);
  };

  const previewPointerAction = (pointerId: number, clientX: number, clientY: number) => {
    const activePointerState = pointerStateRef.current;
    if (!activePointerState || pointerId !== activePointerState.pointerId) {
      return;
    }
    const { blocked, visibleLayout } = resolvePointerPreview(activePointerState, clientX, clientY);
    const nextPointerState = {
      ...activePointerState,
      previewBlocked: blocked,
      previewColumn: visibleLayout.column,
      previewRow: visibleLayout.row,
    };
    if (
      nextPointerState.previewBlocked === activePointerState.previewBlocked &&
      nextPointerState.previewColumn === activePointerState.previewColumn &&
      nextPointerState.previewRow === activePointerState.previewRow
    ) {
      return;
    }
    pointerStateRef.current = nextPointerState;
    setPointerState(nextPointerState);
  };

  const schedulePointerPreview = (pointerId: number, clientX: number, clientY: number) => {
    pendingPointerRef.current = { clientX, clientY, pointerId };
    if (pointerFrameRef.current) {
      return;
    }
    pointerFrameRef.current = window.requestAnimationFrame(() => {
      pointerFrameRef.current = 0;
      const pendingPointer = pendingPointerRef.current;
      pendingPointerRef.current = null;
      if (pendingPointer) {
        previewPointerAction(pendingPointer.pointerId, pendingPointer.clientX, pendingPointer.clientY);
      }
    });
  };

  const finishPointerAction = (pointerId: number, clientX: number, clientY: number) => {
    const activePointerState = pointerStateRef.current;
    if (!activePointerState || pointerId !== activePointerState.pointerId) {
      return;
    }
    if (pointerFrameRef.current) {
      window.cancelAnimationFrame(pointerFrameRef.current);
      pointerFrameRef.current = 0;
    }
    pendingPointerRef.current = null;
    const { blocked, canonicalLayout, visibleLayout } = resolvePointerPreview(activePointerState, clientX, clientY);
    if (!blocked && (visibleLayout.column !== columnStart || visibleLayout.row !== rowStart)) {
      actions.moveWidgetTo(instance, canonicalLayout);
    }

    pointerStateRef.current = null;
    setPointerState(null);
  };

  const cancelPointerAction = () => {
    if (pointerFrameRef.current) {
      window.cancelAnimationFrame(pointerFrameRef.current);
      pointerFrameRef.current = 0;
    }
    pendingPointerRef.current = null;
    pointerStateRef.current = null;
    setPointerState(null);
  };

  useEffect(() => {
    if (!pointerState) {
      return undefined;
    }

    const onPointerMove = (event: globalThis.PointerEvent) => schedulePointerPreview(event.pointerId, event.clientX, event.clientY);
    const onPointerUp = (event: globalThis.PointerEvent) => finishPointerAction(event.pointerId, event.clientX, event.clientY);
    const onPointerCancel = () => cancelPointerAction();
    window.addEventListener("pointermove", onPointerMove, { passive: true });
    window.addEventListener("pointerup", onPointerUp, { passive: true });
    window.addEventListener("pointercancel", onPointerCancel);
    return () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("pointercancel", onPointerCancel);
    };
  }, [pointerState !== null]);

  function resolvePointerPreview(pointerState: WidgetPointerState, clientX: number, clientY: number) {
    const rawPreview = computePointerPreview(pointerState, clientX, clientY, columnSpan, visibleColumns);
    const rawVisibleLayout = { ...projectedLayout, column: rawPreview.column, row: rawPreview.row };
    const { canonicalLayout, renderLayout: visibleLayout } = resolveDashboardVisibleMove(
      rawVisibleLayout,
      instance.layout,
      visibleColumns,
    );
    const blocked = projectedNeighborLayouts.some((neighbor) => layoutsOverlap(visibleLayout, neighbor.layout));
    return { blocked, canonicalLayout, visibleLayout };
  }

  return (
    <>
      {pointerState ? (
        <div
          aria-hidden="true"
          className={`min-h-48 rounded-md border-2 border-dashed ${
            pointerState.previewBlocked ? "border-destructive bg-destructive/10" : "border-primary/60 bg-primary/10"
          }`}
          data-widget-collision={pointerState.previewBlocked ? "true" : "false"}
          data-widget-preview
          style={{
            gridColumn: `${pointerState.previewColumn} / span ${columnSpan}`,
            gridRow: `${pointerState.previewRow} / span ${rowSpan}`,
          }}
        />
      ) : null}
      <Card
        className={`relative flex min-h-48 flex-col overflow-hidden shadow-none ${pointerState?.previewBlocked ? "ring-2 ring-destructive" : ""}`}
        data-widget-card
        data-widget-column={columnStart}
        data-widget-height={rowSpan}
        data-widget-row={rowStart}
        data-widget-width={columnSpan}
        style={{ gridColumn: `${columnStart} / span ${columnSpan}`, gridRow: `${rowStart} / span ${rowSpan}` }}
      >
      <CardHeader className="shrink-0 pb-3">
        <WidgetIconHeader
          icon={widgetModuleIcon(moduleId)}
          subtitle={widgetType?.description ?? instance.widget_type}
          title={
            <span className="flex min-w-0 flex-wrap items-center gap-2 text-base">
              <span className="min-w-0 truncate">
              {moduleId && onNavigate ? (
                <button
                  className="max-w-full truncate text-left underline-offset-4 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  onClick={() => onNavigate(viewForModule(moduleId), title, { moduleId })}
                  type="button"
                >
                  {title}
                </button>
              ) : (
                title
              )}
              </span>
              {moduleId ? <WidgetModuleBadge label={widgetModuleLabel(moduleId, t)} /> : null}
              {extraHeaderBadges.map((badge) => (
                <WidgetModuleBadge key={badge} label={badge} />
              ))}
            </span>
          }
          action={
            <div className="flex shrink-0 items-center gap-1">
            {customMode ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    aria-label={formatTranslation(t("widgets.actions.moveWithMouse"), { title })}
                    className={pointerState ? "cursor-grabbing" : "cursor-grab"}
                    onPointerCancel={cancelPointerAction}
                    onPointerDown={startPointerAction}
                    onPointerUp={(event) => finishPointerAction(event.pointerId, event.clientX, event.clientY)}
                    size="icon"
                    type="button"
                    variant="ghost"
                  >
                    <Move className="size-4" aria-hidden="true" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t("widgets.actions.move")}</TooltipContent>
              </Tooltip>
            ) : null}
            <WidgetContextMenu
              actions={actions}
              instance={instance}
              onEdit={() => setSettingsOpen(true)}
              onRemove={() => setRemoveOpen(true)}
              widgetType={widgetType}
            />
            </div>
          }
        />
      </CardHeader>
      <CardContent className="min-h-0 flex-1 overflow-hidden pb-5" data-widget-body>
        <WidgetErrorBoundary widgetLabel={title}>
          {instance.collapsed ? (
            <p className="text-sm text-muted-foreground">{t("widgets.collapsed")}</p>
          ) : (
            <RenderedWidgetBody
              context={context}
              instance={instance}
              interactions={interactions}
              widgetType={widgetType}
            />
          )}
        </WidgetErrorBoundary>
      </CardContent>
      <WidgetSettingsDialog
        actions={actions}
        instance={instance}
        onOpenChange={setSettingsOpen}
        open={settingsOpen}
        widgetType={widgetType}
      />
      <Dialog onOpenChange={setRemoveOpen} open={removeOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("widgets.settings.removeTitle")}</DialogTitle>
            <DialogDescription>{t("widgets.settings.removeDescription")}</DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2">
            <Button onClick={() => setRemoveOpen(false)} type="button" variant="outline">
              {t("app.actions.cancel")}
            </Button>
            <Button
              className="border-destructive/40 text-destructive hover:bg-destructive/10 hover:text-destructive"
              onClick={() => {
                actions.removeWidget(instance.instance_id);
                setRemoveOpen(false);
              }}
              type="button"
              variant="outline"
            >
              {t("app.actions.remove")}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
      </Card>
    </>
  );
});

function RenderedWidgetBody({
  context,
  instance,
  interactions,
  widgetType,
}: {
  context: WidgetRenderContext;
  instance: DashboardWidgetInstance;
  interactions: WidgetInteractionHandlers;
  widgetType?: WidgetTypeDefinition;
}) {
  return useMemo(
    () => renderWidget(instance, widgetType, context, interactions),
    [context, instance, interactions, widgetType],
  );
}

type WidgetPointerState = {
  cardHeight: number;
  columnWidth: number;
  pointerId: number;
  previewBlocked: boolean;
  previewColumn: number;
  previewRow: number;
  startColumn: number;
  startRow: number;
  startX: number;
  startY: number;
};

type WidgetPointerPoint = {
  clientX: number;
  clientY: number;
  pointerId: number;
};

function computePointerPreview(
  pointerState: WidgetPointerState,
  clientX: number,
  clientY: number,
  columnSpan: number,
  visibleColumns: number,
) {
  const columnDelta = Math.round((clientX - pointerState.startX) / pointerState.columnWidth);
  const rowDelta = Math.round((clientY - pointerState.startY) / pointerState.cardHeight);
  return {
    column: clampInt(pointerState.startColumn + columnDelta, 1, Math.max(1, visibleColumns - columnSpan + 1)),
    row: Math.max(1, pointerState.startRow + rowDelta),
  };
}

function WidgetContextMenu({
  actions,
  instance,
  onEdit,
  onRemove,
  widgetType,
}: {
  actions: WidgetActions;
  instance: DashboardWidgetInstance;
  onEdit: () => void;
  onRemove: () => void;
  widgetType?: WidgetTypeDefinition;
}) {
  const { t } = useI18n();
  const nextSizes = widgetType?.allowed_sizes ?? [{ width: 1, height: 1 }, { width: 2, height: 1 }, { width: 2, height: 2 }];
  const title = dashboardWidgetTitle(instance, widgetType);

  return (
    <DropdownMenu>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuTrigger asChild>
            <Button aria-label={formatTranslation(t("widgets.actions.openActions"), { title })} size="icon" variant="ghost">
              <MoreHorizontal className="size-4" aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent>{t("widgets.actions.actions")}</TooltipContent>
      </Tooltip>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={onEdit}>
          <Pencil className="size-4" aria-hidden="true" />
          {t("widgets.actions.edit")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => actions.toggleCollapsed(instance)}>
          {instance.collapsed ? <ChevronsDown className="size-4" aria-hidden="true" /> : <ChevronsUp className="size-4" aria-hidden="true" />}
          {instance.collapsed ? t("widgets.actions.expand") : t("widgets.actions.collapse")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => actions.duplicateWidget(instance.instance_id)}>
          <Copy className="size-4" aria-hidden="true" />
          {t("widgets.actions.duplicate")}
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={() => actions.moveWidget(instance, "up")}>{t("widgets.actions.moveUp")}</DropdownMenuItem>
        <DropdownMenuItem onSelect={() => actions.moveWidget(instance, "down")}>{t("widgets.actions.moveDown")}</DropdownMenuItem>
        {nextSizes.map((size) => (
          <DropdownMenuItem key={`${size.width}x${size.height}`} onSelect={() => actions.resizeWidget(instance.instance_id, size)}>
            <Maximize2 className="size-4" aria-hidden="true" />
            {t("widgets.actions.resize")} {size.width}x{size.height}
          </DropdownMenuItem>
        ))}
        <DropdownMenuItem className="text-destructive focus:bg-destructive/10 focus:text-destructive" onSelect={onRemove}>
          <Trash2 className="size-4" aria-hidden="true" />
          {t("widgets.actions.remove")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function WidgetSettingsDialog({
  actions,
  instance,
  onOpenChange,
  open,
  widgetType,
}: {
  actions: WidgetActions;
  instance: DashboardWidgetInstance;
  onOpenChange: (open: boolean) => void;
  open: boolean;
  widgetType?: WidgetTypeDefinition;
}) {
  const { t } = useI18n();
  const [title, setTitle] = useState(instance.title ?? "");
  const [config, setConfig] = useState<Record<string, unknown>>(instance.config ?? {});
  const [size, setSize] = useState<WidgetSizeDefinition>(instance.layout);
  const configEntries = useMemo(() => Object.entries(widgetType?.config_schema ?? {}), [widgetType]);
  const allowedSizes = widgetType?.allowed_sizes ?? [{ width: 1, height: 1 }, { width: 2, height: 1 }, { width: 2, height: 2 }];

  useEffect(() => {
    if (!open) return;
    setTitle(instance.title ?? "");
    setConfig(instance.config ?? {});
    setSize({ width: instance.layout.width, height: instance.layout.height });
  }, [instance, open]);

  const save = () => {
    actions.updateWidget(instance.instance_id, { title, config });
    if (size.width !== instance.layout.width || size.height !== instance.layout.height) {
      actions.resizeWidget(instance.instance_id, size);
    }
    onOpenChange(false);
  };

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="max-h-[86vh] overflow-auto">
        <DialogHeader>
          <DialogTitle>{t("widgets.settings.title")}</DialogTitle>
          <DialogDescription>{t("widgets.settings.description")}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-4">
          <div className="grid gap-2">
            <Label htmlFor={`widget-title-${instance.instance_id}`}>{t("widgets.settings.widgetTitle")}</Label>
            <Input
              id={`widget-title-${instance.instance_id}`}
              onChange={(event) => setTitle(event.target.value)}
              placeholder={widgetType?.label ?? instance.widget_type}
              value={title}
            />
          </div>
          <div className="grid gap-2">
            <Label>{t("widgets.settings.size")}</Label>
            <div className="flex flex-wrap gap-2">
              {allowedSizes.map((candidate) => {
                const active = candidate.width === size.width && candidate.height === size.height;
                return (
                  <Button
                    aria-pressed={active}
                    key={`${candidate.width}x${candidate.height}`}
                    onClick={() => setSize(candidate)}
                    size="sm"
                    type="button"
                    variant={active ? "default" : "outline"}
                  >
                    {candidate.width}x{candidate.height}
                  </Button>
                );
              })}
            </div>
          </div>
          {configEntries.length ? (
            <div className="grid gap-3">
              <Label>{t("widgets.settings.config")}</Label>
              {configEntries.map(([key, definition]) => (
                <WidgetConfigField
                  definition={definition}
                  key={key}
                  name={key}
                  onChange={(value) => setConfig((current) => ({ ...current, [key]: value }))}
                  value={config[key] ?? definition.default ?? ""}
                />
              ))}
            </div>
          ) : null}
        </div>
        <div className="flex justify-end gap-2">
          <Button onClick={() => onOpenChange(false)} type="button" variant="outline">
            {t("app.actions.cancel")}
          </Button>
          <Button onClick={save} type="button">
            {t("app.actions.save")}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function WidgetConfigField({
  definition,
  name,
  onChange,
  value,
}: {
  definition: WidgetConfigFieldDefinition;
  name: string;
  onChange: (value: unknown) => void;
  value: unknown;
}) {
  const id = `widget-config-${name}`;
  if (definition.type === "boolean") {
    return (
      <label className="flex items-center gap-2 text-sm" htmlFor={id}>
        <input
          checked={Boolean(value)}
          id={id}
          onChange={(event) => onChange(event.target.checked)}
          type="checkbox"
        />
        {fieldLabel(name)}
      </label>
    );
  }
  if (definition.type === "enum") {
    return (
      <div className="grid gap-2">
        <Label htmlFor={id}>{fieldLabel(name)}</Label>
        <select
          className="h-10 rounded-md border border-input bg-background px-3 text-sm"
          id={id}
          onChange={(event) => onChange(event.target.value)}
          value={String(value ?? "")}
        >
          {(definition.options ?? []).map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </select>
      </div>
    );
  }
  return (
    <div className="grid gap-2">
      <Label htmlFor={id}>{fieldLabel(name)}</Label>
      <Input
        id={id}
        max={definition.max}
        min={definition.min}
        onChange={(event) => onChange(definition.type === "number" ? Number(event.target.value) : event.target.value)}
        type={definition.type === "number" ? "number" : "text"}
        value={String(value ?? "")}
      />
    </div>
  );
}

function fieldLabel(value: string) {
  return value.replace(/[_-]+/g, " ").replace(/\b\w/g, (character) => character.toUpperCase());
}

function formatTranslation(template: string, values: Record<string, string>) {
  return template.replace(/\{(\w+)\}/g, (_, key: string) => values[key] ?? "");
}

function widgetHeaderBadges(
  instance: DashboardWidgetInstance,
  context: WidgetRenderContext,
  t: (key: string) => string,
) {
  if (instance.widget_type !== "notes.by-tag") {
    return [];
  }
  const tag = String(instance.config.tag ?? "daily").toLowerCase().replace(/^#/, "");
  const matches = context.notes.filter((note) => `${note.title} ${note.excerpt}`.toLowerCase().includes(tag)).length;
  return [`#${tag}`, `${matches} ${t("widgets.labels.matches").toLowerCase()}`];
}

function widgetModuleLabel(moduleId: string, t: (key: string) => string) {
  if (moduleId === "notes") return t("nav.notes");
  if (moduleId === "todos") return t("nav.todos");
  if (moduleId === "contacts") return t("nav.contacts");
  if (moduleId === "habits") return t("nav.habits");
  return moduleId.replace(/[_-]+/g, " ");
}

function widgetModuleIcon(moduleId?: string) {
  if (moduleId === "notes") return <BookOpen className="size-4" aria-hidden="true" />;
  if (moduleId === "todos") return <CheckSquare className="size-4" aria-hidden="true" />;
  if (moduleId === "contacts") return <Users className="size-4" aria-hidden="true" />;
  if (moduleId === "habits") return <Flame className="size-4" aria-hidden="true" />;
  return null;
}

function clampInt(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, Math.round(value || min)));
}
