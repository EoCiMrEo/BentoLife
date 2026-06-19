import { Move, RotateCcw } from "lucide-react";
import { useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Empty } from "@/components/ui/empty";
import { WidgetPicker, type WidgetActions } from "@/components/widgets/WidgetCanvas";
import { useI18n } from "@/i18n";
import type { RegistryState } from "@/services/backendCore";
import type { WidgetInteractionHandlers, WidgetRenderContext } from "@/services/widgetRendererRegistry";
import {
  dashboardWidgetWarningKey,
  dashboardWidgetWarningMessage,
  isWidgetActive,
  type DashboardWidgetState,
  type WidgetTypeDefinition,
} from "@/services/widgets";

export function WidgetManager({
  actions,
  moduleRegistry,
  onEnableModule,
  onReset,
  state,
  widgetTypes,
}: {
  actions: WidgetActions;
  context: WidgetRenderContext;
  interactions: WidgetInteractionHandlers;
  moduleRegistry: RegistryState | null;
  onEnableModule: (moduleId: string) => void;
  onReset: () => void;
  state: DashboardWidgetState | null;
  widgetTypes: WidgetTypeDefinition[];
}) {
  const { t } = useI18n();
  const [pickerOpen, setPickerOpen] = useState(false);
  const instances = state?.instances ?? [];
  const warnings = state?.warnings ?? [];
  const active = instances.filter((instance) => isWidgetActive(instance, moduleRegistry));
  const inactive = instances.filter((instance) => !isWidgetActive(instance, moduleRegistry));

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => setPickerOpen(true)}>{t("widgets.manager.add")}</Button>
        <Button disabled={!instances.length} onClick={actions.compactWidgets} variant="outline">
          <Move data-icon="inline-start" />
          {t("dashboard.compactLayout")}
        </Button>
        <Button disabled={!instances.length} onClick={onReset} variant="outline">
          <RotateCcw data-icon="inline-start" />
          {t("widgets.manager.reset")}
        </Button>
      </div>
      {warnings.length ? (
        <div className="rounded-md border border-border bg-muted/45 p-3 text-sm text-muted-foreground">
          {warnings.map((warning, index) => (
            <p key={dashboardWidgetWarningKey(warning, index)}>{dashboardWidgetWarningMessage(warning)}</p>
          ))}
        </div>
      ) : null}
      {!instances.length ? (
        <Empty title={t("widgets.manager.empty.title")} description={t("widgets.manager.empty.description")} />
      ) : null}
      {inactive.length ? (
        <div className="rounded-md border border-border bg-muted/35 p-3 text-sm text-muted-foreground">
          {inactive.length} {inactive.length === 1 ? t("widgets.manager.inactiveSingle") : t("widgets.manager.inactivePlural")}
        </div>
      ) : null}
      {active.length ? (
        <div className="rounded-md border border-border bg-background p-3 text-sm text-muted-foreground">
          {active.length} {active.length === 1 ? t("widgets.manager.activeSingle") : t("widgets.manager.activePlural")}
        </div>
      ) : null}
      <WidgetTypesByModule moduleRegistry={moduleRegistry} widgetTypes={widgetTypes} />
      <WidgetPicker
        moduleRegistry={moduleRegistry}
        onAdd={(widgetType) => {
          actions.addWidget(widgetType);
          setPickerOpen(false);
        }}
        onEnableModule={onEnableModule}
        open={pickerOpen}
        setOpen={setPickerOpen}
        widgetTypes={widgetTypes}
      />
    </div>
  );
}

function WidgetTypesByModule({ moduleRegistry, widgetTypes }: { moduleRegistry: RegistryState | null; widgetTypes: WidgetTypeDefinition[] }) {
  const { t } = useI18n();
  const modules = moduleRegistry?.modules ?? [];
  const counts = widgetTypes.reduce<Record<string, number>>((acc, widgetType) => {
    acc[widgetType.module_id] = (acc[widgetType.module_id] ?? 0) + 1;
    return acc;
  }, {});

  return (
    <Card className="shadow-none">
      <CardHeader>
        <CardTitle className="text-base">{t("widgets.manager.available.title")}</CardTitle>
        <CardDescription>{t("widgets.manager.available.description")}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-2 md:grid-cols-2">
        {Object.entries(counts).map(([moduleId, count]) => {
          const module = modules.find((candidate) => candidate.id === moduleId);
          const canAdd = module ? module.available && module.installed && module.enabled : true;
          return (
            <div className="rounded-md border border-border bg-background px-3 py-2" key={moduleId}>
              <div className="flex items-center justify-between gap-3">
                <span className="text-sm font-medium">{module?.display_name ?? moduleId}</span>
                <Badge variant={canAdd ? "secondary" : "outline"}>{count} {t("architect.modules.widgets")}</Badge>
              </div>
              {!canAdd ? (
                <p className="mt-2 text-xs text-muted-foreground">{t("widgets.manager.installOrEnable")}</p>
              ) : null}
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}
