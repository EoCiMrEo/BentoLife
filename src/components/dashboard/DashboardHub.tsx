import { AlertTriangle, BookOpenText, CheckSquare, Leaf, Network, Plus, Sparkles, Users, type LucideIcon } from "lucide-react";
import { useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { WidgetCanvas, WidgetPicker, type WidgetActions, type WidgetNavigate } from "@/components/widgets/WidgetCanvas";
import type { RegistryState } from "@/services/backendCore";
import type { DashboardHubDocument, DashboardPinnedEntity } from "@/services/dashboard";
import type { WidgetInteractionHandlers, WidgetRenderContext } from "@/services/widgetRendererRegistry";
import { isWidgetActive, type DashboardWidgetState, type WidgetTypeDefinition } from "@/services/widgets";

export type DashboardHubProps = {
  dashboardHub: DashboardHubDocument | null;
  loading: boolean;
  moduleRegistry: RegistryState | null;
  onEnableModuleFromWidgetPicker: (moduleId: string) => void;
  onNavigate: WidgetNavigate;
  onOpenArchitect: () => void;
  onOpenPinnedEntity: (entity: DashboardPinnedEntity) => void;
  onRefresh: () => void;
  widgetActions: WidgetActions;
  widgetContext: WidgetRenderContext;
  widgetInteractions: WidgetInteractionHandlers;
  widgetState: DashboardWidgetState | null;
  widgetTypes: WidgetTypeDefinition[];
  workspaceError: string | null;
};

export function DashboardHub({
  dashboardHub,
  loading,
  moduleRegistry,
  onEnableModuleFromWidgetPicker,
  onNavigate,
  onOpenArchitect,
  onOpenPinnedEntity,
  onRefresh,
  widgetActions,
  widgetContext,
  widgetInteractions,
  widgetState,
  widgetTypes,
  workspaceError,
}: DashboardHubProps) {
  const pinnedCount = dashboardHub?.pinned_entities.length ?? 0;
  const activeWidgetCount = (widgetState?.instances ?? []).filter((instance) => isWidgetActive(instance, moduleRegistry)).length;
  const [widgetPickerOpen, setWidgetPickerOpen] = useState(false);
  const [dashboardCustomMode, setDashboardCustomMode] = useState(false);

  return (
    <section className="grid gap-5">
      <div className="flex min-w-0 flex-col gap-5">
        {workspaceError ? <RepairNotice title="Workspace scan failed" message={workspaceError} /> : null}
        {dashboardHub?.unresolved_pins.length ? (
          <RepairNotice
            title="Unresolved pinned entities"
            message={`Architect Mode can review unresolved pins: ${dashboardHub.unresolved_pins.join(", ")}.`}
          />
        ) : null}
        {dashboardHub?.pinned_entities.length ? (
          <section className="rounded-md border border-border bg-card p-4 shadow-soft" aria-label="Pinned entities">
            <div className="mb-3 flex items-center justify-between gap-3">
              <h1 className="text-xl font-semibold">{dashboardHub?.title ?? "Dashboard"}</h1>
              <Badge variant="secondary">{pinnedCount} pinned</Badge>
            </div>
            <div className="grid gap-2 md:grid-cols-2">
              {dashboardHub.pinned_entities.map((entity) => (
                <PinnedEntityRow
                  icon={iconForEntity(entity.entity_type)}
                  key={`${entity.document_id}-${entity.target}`}
                  onClick={() => onOpenPinnedEntity(entity)}
                  path={entity.markdown_relative_path}
                  title={entity.title}
                  type={entity.entity_type}
                />
              ))}
            </div>
          </section>
        ) : null}
        {!loading && (activeWidgetCount > 0 || !pinnedCount) ? (
          <WidgetCanvas
            actions={widgetActions}
            customMode={dashboardCustomMode}
            context={widgetContext}
            interactions={widgetInteractions}
            moduleRegistry={moduleRegistry}
            onEnableModule={onEnableModuleFromWidgetPicker}
            onNavigate={onNavigate}
            onOpenArchitect={onOpenArchitect}
            onRetry={onRefresh}
            setCustomMode={setDashboardCustomMode}
            state={widgetState}
            widgetTypes={widgetTypes}
          />
        ) : null}
        {!loading && pinnedCount > 0 && activeWidgetCount === 0 ? (
          <div className="flex justify-end">
            <Button onClick={() => setWidgetPickerOpen(true)} variant="outline">
              <Plus data-icon="inline-start" />
              Add Widget
            </Button>
            <WidgetPicker
              moduleRegistry={moduleRegistry}
              onAdd={(widgetType) => {
                widgetActions.addWidget(widgetType);
                setWidgetPickerOpen(false);
              }}
              onEnableModule={onEnableModuleFromWidgetPicker}
              onOpenArchitect={onOpenArchitect}
              onRetry={onRefresh}
              open={widgetPickerOpen}
              setOpen={setWidgetPickerOpen}
              widgetTypes={widgetTypes}
            />
          </div>
        ) : null}
        {loading ? <DashboardSkeleton /> : null}
      </div>
    </section>
  );
}

function RepairNotice({ message, title }: { message: string; title: string }) {
  return (
    <div className="flex gap-3 rounded-md border border-border bg-muted/55 p-4 text-sm">
      <AlertTriangle aria-hidden="true" className="mt-0.5 shrink-0 text-amber-note-foreground" />
      <div className="min-w-0">
        <p className="font-medium">{title}</p>
        <p className="mt-1 text-muted-foreground">{message}</p>
      </div>
    </div>
  );
}

function DashboardSkeleton() {
  return (
    <div className="grid gap-5 xl:grid-cols-2">
      <Skeleton className="h-64" />
      <Skeleton className="h-64" />
    </div>
  );
}

function PinnedEntityRow({
  icon: Icon,
  onClick,
  path,
  title,
  type,
}: {
  icon: LucideIcon;
  onClick: () => void;
  path: string;
  title: string;
  type: string;
}) {
  return (
    <button
      aria-label={`Open pinned ${title}`}
      className="group flex min-w-0 items-center gap-3 rounded-md border border-border bg-background px-3 py-2 text-left transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      onClick={onClick}
      type="button"
    >
      <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
        <Icon aria-hidden="true" className="size-4" />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">{title}</span>
        <span className="block truncate text-xs text-muted-foreground">{path}</span>
      </span>
      <Badge variant="secondary">{type}</Badge>
    </button>
  );
}

function iconForEntity(entityType: string): LucideIcon {
  switch (entityType) {
    case "todos":
      return CheckSquare;
    case "contact":
    case "contacts":
      return Users;
    case "habit":
    case "habits":
      return Leaf;
    case "navigator":
      return Network;
    case "dashboard":
      return Sparkles;
    default:
      return BookOpenText;
  }
}
