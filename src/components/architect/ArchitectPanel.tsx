import { AlertTriangle, CheckCircle2, ChevronRight, FileJson, FolderOpen, Palette, RotateCcw, Upload, X } from "lucide-react";
import { useState, type ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { WidgetManager } from "@/components/widgets/WidgetManager";
import { useI18n } from "@/i18n";
import type { WidgetActions } from "@/components/widgets/WidgetCanvas";
import type { DashboardHubDocument } from "@/services/dashboard";
import type { EntityUpgradePreview, NavigatorSnapshot, RegistryState, SearchIndexSnapshot } from "@/services/backendCore";
import type { WidgetInteractionHandlers, WidgetRenderContext } from "@/services/widgetRendererRegistry";
import type { ActiveThemeState, ThemePreview, ThemeScope } from "@/services/theme";
import type { ArchitectTabId, DashboardWidgetState, WidgetTypeDefinition, WorkspaceUiState } from "@/services/widgets";
import {
  exportWidgetLayoutFile,
  importThemeFile,
  importWidgetLayoutFile,
  selectImportFile,
  validateThemeImport,
  validateWidgetLayoutImport,
  type ImportResult,
  type ImportValidation,
} from "@/services/imports";
import type { AppView, FocusTarget } from "@/state/navigation";
import { viewForModule } from "@/state/navigation";
import type { WorkspaceScanResult } from "@/services/notes";

type ArchitectTopTabId = "modules" | "dashboard" | "advanced";

const architectTabs: Array<{ descriptionKey: string; id: ArchitectTopTabId; labelKey: string }> = [
  { id: "modules", labelKey: "architect.tabs.modules", descriptionKey: "architect.tabs.modules.description" },
  { id: "dashboard", labelKey: "architect.tabs.dashboard", descriptionKey: "architect.tabs.dashboard.description" },
  { id: "advanced", labelKey: "architect.tabs.advanced", descriptionKey: "architect.tabs.advanced.description" },
];

const advancedArchitectTabs: Array<{ descriptionKey: string; id: ArchitectTabId; labelKey: string }> = [
  { id: "appearance", labelKey: "architect.tabs.appearance", descriptionKey: "architect.tabs.appearance.description" },
  { id: "schemas", labelKey: "architect.tabs.schemas", descriptionKey: "architect.tabs.schemas.description" },
  { id: "data_graph", labelKey: "architect.tabs.dataGraph", descriptionKey: "architect.tabs.dataGraph.description" },
  { id: "recovery", labelKey: "architect.tabs.recovery", descriptionKey: "architect.tabs.recovery.description" },
];

const defaultArchitectTab: ArchitectTabId = "modules";
const noopWidgetInteractions: WidgetInteractionHandlers = {
  openEntity: () => {},
};

export type ArchitectPanelProps = {
  activeArchitectTab?: ArchitectTabId;
  activeTheme: ActiveThemeState | null;
  dashboardHub: DashboardHubDocument | null;
  moduleRegistry: RegistryState | null;
  navigator: NavigatorSnapshot | null;
  onArchitectSectionChange: (section: string, expanded: boolean) => void;
  onArchitectTabChange: (tab: ArchitectTabId) => void;
  onApplyEntityUpgrade: () => void;
  onApplyTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  onCancelThemePreview: () => void;
  onNavigate: (view: AppView, label?: string, options?: Partial<FocusTarget>) => void;
  onPreviewEntityUpgrade: () => void;
  onPreviewTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  onRebuildNavigator: () => void;
  onSearchGraph: (query: string) => void;
  onRollbackTheme: (scope: ThemeScope, moduleId: string | null) => void;
  onResetWidgets: () => void;
  onToggleModule: (moduleId: string, enabled: boolean) => void;
  recoveryPanel: ReactNode;
  scan: WorkspaceScanResult | null;
  searchSnapshot: SearchIndexSnapshot | null;
  themePreview: ThemePreview | null;
  upgradePreview: EntityUpgradePreview | null;
  widgetActions: WidgetActions;
  widgetContext: WidgetRenderContext;
  widgetInteractions?: WidgetInteractionHandlers;
  widgetState: DashboardWidgetState | null;
  widgetTypes: WidgetTypeDefinition[];
  workspaceUiState: WorkspaceUiState | null;
  vaultPath?: string;
};

export function ArchitectPanel({
  activeArchitectTab,
  activeTheme,
  dashboardHub,
  moduleRegistry,
  navigator,
  onArchitectSectionChange,
  onArchitectTabChange,
  onApplyEntityUpgrade,
  onApplyTheme,
  onCancelThemePreview,
  onNavigate,
  onPreviewEntityUpgrade,
  onPreviewTheme,
  onRebuildNavigator,
  onRollbackTheme,
  onSearchGraph,
  onResetWidgets,
  onToggleModule,
  recoveryPanel,
  scan,
  searchSnapshot,
  themePreview,
  upgradePreview,
  widgetActions,
  widgetContext,
  widgetInteractions = noopWidgetInteractions,
  widgetState,
  widgetTypes,
  workspaceUiState,
  vaultPath,
}: ArchitectPanelProps) {
  const { t } = useI18n();
  const modules = moduleRegistry?.modules ?? [];
  const schemaWarningModules = modules.filter((module) => moduleWarnings(module).length);
  const requestedTab = isArchitectTab(activeArchitectTab)
    ? activeArchitectTab
    : isArchitectTab(workspaceUiState?.architect_active_tab)
      ? workspaceUiState.architect_active_tab
      : defaultArchitectTab;
  const activeTab: ArchitectTopTabId = isAdvancedArchitectTab(requestedTab) ? "advanced" : requestedTab;
  const activeAdvancedTab = isAdvancedArchitectTab(requestedTab) ? requestedTab : "appearance";
  const enableModuleFromWidgetPicker = (moduleId: string) => {
    const module = moduleRegistry?.modules.find((candidate) => candidate.id === moduleId);
    const label = module?.display_name ?? moduleId;
    if (window.confirm(`${t("architect.modules.enable")} ${label}?`)) {
      onToggleModule(moduleId, true);
    }
  };
  const configureModule = (moduleId: string, label: string) => {
    if (moduleId === "navigator") {
      onNavigate("architect", t("nav.dataGraph"), { architectTab: "data_graph", moduleId });
      return;
    }
    onNavigate(viewForModule(moduleId), label, { moduleId });
  };

  return (
    <Tabs
      className="flex flex-col gap-4"
      onValueChange={(value) => onArchitectTabChange(value === "advanced" ? activeAdvancedTab : asArchitectTab(value))}
      value={activeTab}
    >
      <div className="overflow-x-auto pb-1">
        <TabsList className="h-auto w-max justify-start">
          {architectTabs.map((tab) => (
            <TabsTrigger key={tab.id} value={tab.id}>
              {t(tab.labelKey)}
            </TabsTrigger>
          ))}
        </TabsList>
      </div>
      <p className="text-sm text-muted-foreground">{t(architectTabs.find((tab) => tab.id === activeTab)?.descriptionKey ?? "")}</p>
      <TabsContent value="modules">
        <ModulesSection
          moduleRegistry={moduleRegistry}
          onConfigure={configureModule}
          onSectionChange={onArchitectSectionChange}
          onToggleModule={onToggleModule}
          workspaceUiState={workspaceUiState}
          widgetTypes={widgetTypes}
        />
      </TabsContent>
      <TabsContent value="dashboard">
        <div className="grid gap-5">
          <WidgetManager
            actions={widgetActions}
            context={widgetContext}
            interactions={widgetInteractions}
            moduleRegistry={moduleRegistry}
            onEnableModule={enableModuleFromWidgetPicker}
            onReset={onResetWidgets}
            state={widgetState}
            widgetTypes={widgetTypes}
          />
          <WidgetLayoutImportPanel vaultPath={vaultPath} />
        </div>
      </TabsContent>
      <TabsContent value="advanced">
        <div className="grid gap-4">
          <div className="overflow-x-auto pb-1">
            <div className="inline-flex rounded-md bg-muted p-1" role="tablist" aria-label={t("architect.tabs.advanced")}>
              {advancedArchitectTabs.map((tab) => (
                <button
                  aria-selected={activeAdvancedTab === tab.id}
                  className={[
                    "rounded-sm px-3 py-1.5 text-sm font-medium transition-colors",
                    activeAdvancedTab === tab.id ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground",
                  ].join(" ")}
                  data-state={activeAdvancedTab === tab.id ? "active" : "inactive"}
                  key={tab.id}
                  onClick={() => onArchitectTabChange(tab.id)}
                  role="tab"
                  type="button"
                >
                  {t(tab.labelKey)}
                </button>
              ))}
            </div>
          </div>
          <p className="text-sm text-muted-foreground">{t(advancedArchitectTabs.find((tab) => tab.id === activeAdvancedTab)?.descriptionKey ?? "")}</p>
          {activeAdvancedTab === "appearance" ? (
            <ThemesSection
              activeTheme={activeTheme}
              onApplyTheme={onApplyTheme}
              onCancelThemePreview={onCancelThemePreview}
              onPreviewTheme={onPreviewTheme}
              onRollbackTheme={onRollbackTheme}
              themePreview={themePreview}
              vaultPath={vaultPath}
            />
          ) : null}
          {activeAdvancedTab === "schemas" ? <SchemasSection moduleRegistry={moduleRegistry} schemaWarningModules={schemaWarningModules} /> : null}
          {activeAdvancedTab === "data_graph" ? (
            <DataGraphSection
              dashboardHub={dashboardHub}
              navigator={navigator}
              onApplyEntityUpgrade={onApplyEntityUpgrade}
              onPreviewEntityUpgrade={onPreviewEntityUpgrade}
              onRebuildNavigator={onRebuildNavigator}
              onNavigate={onNavigate}
              onSearchGraph={onSearchGraph}
              scan={scan}
              searchSnapshot={searchSnapshot}
              upgradePreview={upgradePreview}
            />
          ) : null}
          {activeAdvancedTab === "recovery" ? recoveryPanel : null}
        </div>
      </TabsContent>
    </Tabs>
  );
}

function asArchitectTab(value: string): ArchitectTabId {
  return isArchitectTab(value) ? value : defaultArchitectTab;
}

function isArchitectTab(value: unknown): value is ArchitectTabId {
  return value === "modules" || value === "dashboard" || advancedArchitectTabs.some((tab) => tab.id === value);
}

function isAdvancedArchitectTab(value: ArchitectTabId): value is Extract<ArchitectTabId, "appearance" | "schemas" | "data_graph" | "recovery"> {
  return advancedArchitectTabs.some((tab) => tab.id === value);
}

export function ModulesSection({
  moduleRegistry,
  onConfigure,
  onSectionChange,
  onToggleModule,
  workspaceUiState,
  widgetTypes,
}: {
  moduleRegistry: RegistryState | null;
  onConfigure: (moduleId: string, label: string) => void;
  onSectionChange: (section: string, expanded: boolean) => void;
  onToggleModule: (moduleId: string, enabled: boolean) => void;
  workspaceUiState: WorkspaceUiState | null;
  widgetTypes: WidgetTypeDefinition[];
}) {
  const { t } = useI18n();
  const modules = moduleRegistry?.modules ?? [];
  const builtInModules = modules.filter((module) => module.kind === "system");
  const starterModules = modules.filter((module) => module.kind === "starter");
  const optionalModules = modules.filter((module) => module.kind === "optional");

  return (
    <div className="grid gap-5">
      <ModuleGroup
        defaultOpen={false}
        modules={builtInModules}
        onConfigure={onConfigure}
        onSectionChange={onSectionChange}
        sectionKey="modules_system_expanded"
        title={t("architect.modules.system")}
        widgetTypes={widgetTypes}
        workspaceUiState={workspaceUiState}
      />
      <ModuleGroup
        defaultOpen
        modules={starterModules}
        onConfigure={onConfigure}
        onSectionChange={onSectionChange}
        sectionKey="modules_starter_expanded"
        title={t("architect.modules.starter")}
        widgetTypes={widgetTypes}
        workspaceUiState={workspaceUiState}
      />
      <ModuleGroup
        defaultOpen
        modules={optionalModules}
        onConfigure={onConfigure}
        onSectionChange={onSectionChange}
        onToggleModule={onToggleModule}
        sectionKey="modules_optional_expanded"
        title={t("architect.modules.optional")}
        widgetTypes={widgetTypes}
        workspaceUiState={workspaceUiState}
      />
    </div>
  );
}

export function DashboardWidgetsSection() {
  return null;
}

export function DashboardLayoutSection({ children }: { children: ReactNode }) {
  return <>{children}</>;
}

export function ThemesSection({
  activeTheme,
  onApplyTheme,
  onCancelThemePreview,
  onPreviewTheme,
  onRollbackTheme,
  themePreview,
  vaultPath,
}: {
  activeTheme: ActiveThemeState | null;
  onApplyTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  onCancelThemePreview: () => void;
  onPreviewTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  onRollbackTheme: (scope: ThemeScope, moduleId: string | null) => void;
  themePreview: ThemePreview | null;
  vaultPath?: string;
}) {
  const { t } = useI18n();
  const workspaceTheme = activeTheme?.workspace_theme;
  const moduleEntries = Object.values(activeTheme?.module_themes ?? {});
  const moduleDefaults = Object.keys(activeTheme?.module_default_tokens ?? {});

  return (
    <div className="grid gap-4 text-sm">
      <div className="grid gap-2 rounded-md border border-border bg-background p-3">
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="secondary">{t("architect.theme.tokenOnly")}</Badge>
          <Badge variant={themePreview ? "status" : "outline"}>{themePreview ? t("architect.theme.previewActive") : t("architect.theme.noPreview")}</Badge>
        </div>
        <SummaryRow label={t("settings.theme.workspace")} value={workspaceTheme?.theme_id ?? "clean-slate"} />
        <SummaryRow label={t("architect.theme.workspaceTokens")} value={`${Object.keys(workspaceTheme?.tokens ?? {}).length}`} />
        <SummaryRow label={t("architect.theme.moduleDefaults")} value={`${moduleDefaults.length}`} />
        <SummaryRow label={t("settings.theme.moduleOverrides")} value={`${moduleEntries.length}`} />
        {themePreview ? (
          <>
            <SummaryRow label={t("architect.theme.previewScope")} value={themePreview.scope === "module" ? `module:${themePreview.module_id ?? "unknown"}` : "workspace"} />
            <SummaryRow label={t("architect.theme.previewSource")} value={themePreview.source_path} />
          </>
        ) : null}
      </div>
      <div className="rounded-md border border-border bg-muted/45 p-3">
        <p className="font-medium">{t("architect.theme.precedence")}</p>
        <ol className="mt-2 grid gap-1 text-muted-foreground">
          <li>1. {t("architect.theme.defaultTokens")}</li>
          <li>2. {t("architect.theme.workspaceThemeTokens")}</li>
          <li>3. {t("architect.theme.moduleDefaultTokens")}</li>
          <li>4. {t("architect.theme.moduleOverrideTokens")}</li>
          <li>5. {t("architect.theme.previewTokens")}</li>
        </ol>
      </div>
      <ThemeTokenImportRegistry
        onApplyTheme={onApplyTheme}
        onCancelThemePreview={onCancelThemePreview}
        onPreviewTheme={onPreviewTheme}
        themePreview={themePreview}
        vaultPath={vaultPath}
      />
      {moduleEntries.length ? (
        <div className="grid gap-2">
          <p className="font-medium">{t("architect.theme.activeOverrides")}</p>
          {moduleEntries.map((manifest) => (
            <div className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-border bg-background px-3 py-2" key={manifest.module_id ?? manifest.theme_id}>
              <span className="min-w-0 truncate">
                {manifest.module_id ?? "module"} · {Object.keys(manifest.tokens).length} tokens
              </span>
              <Button onClick={() => onRollbackTheme("module", manifest.module_id)} size="sm" variant="ghost">
                <RotateCcw data-icon="inline-start" />
                {t("architect.theme.rollBack")}
              </Button>
            </div>
          ))}
        </div>
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Button onClick={() => onRollbackTheme("workspace", null)} size="sm" variant="ghost">
          <RotateCcw data-icon="inline-start" />
          {t("architect.theme.rollBackWorkspace")}
        </Button>
      </div>
    </div>
  );
}

function ThemeTokenImportRegistry({
  onApplyTheme,
  onCancelThemePreview,
  onPreviewTheme,
  themePreview,
  vaultPath,
}: {
  onApplyTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  onCancelThemePreview: () => void;
  onPreviewTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  themePreview: ThemePreview | null;
  vaultPath?: string;
}) {
  const { t } = useI18n();
  const [sourcePath, setSourcePath] = useState("");
  const [result, setResult] = useState<ImportValidation | ImportResult | null>(null);
  const [busy, setBusy] = useState<"choose" | "validate" | "import" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const validation = result && "validation" in result ? result.validation : result;
  const importedPath = result && "stored_relative_path" in result ? result.stored_relative_path : null;
  const previewSource = importedPath ?? sourcePath;

  const chooseFile = async () => {
    setBusy("choose");
    setError(null);
    try {
      const selected = await selectImportFile("theme");
      if (selected) {
        setSourcePath(selected);
        setResult(null);
      }
    } catch (error) {
      setError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  const validateSource = async () => {
    setBusy("validate");
    setError(null);
    try {
      setResult(await validateThemeImport(sourcePath));
    } catch (error) {
      setError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  const importSource = async () => {
    if (!vaultPath) {
      setError("Select or create a vault before importing theme tokens.");
      return;
    }
    setBusy("import");
    setError(null);
    try {
      const imported = await importThemeFile(vaultPath, sourcePath);
      setResult(imported);
      setSourcePath(imported.stored_relative_path);
    } catch (error) {
      setError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="grid gap-3 rounded-md border border-border bg-background p-3">
      <div className="flex flex-wrap items-center gap-2">
        <Palette className="size-4 text-muted-foreground" aria-hidden="true" />
        <p className="font-medium">{t("architect.theme.registry")}</p>
        <Badge variant="secondary">{t("architect.theme.importTokens")}</Badge>
      </div>
      <p className="text-muted-foreground">{t("architect.theme.registryDescription")}</p>
      <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
        <div className="grid gap-2">
          <Label htmlFor="architect-theme-token-path">{t("architect.theme.source")}</Label>
          <Input
            id="architect-theme-token-path"
            onChange={(event) => {
              setSourcePath(event.target.value);
              setResult(null);
            }}
            placeholder="safe-theme.css"
            value={sourcePath}
          />
        </div>
        <Button className="self-end" disabled={busy !== null} onClick={chooseFile} variant="outline">
          <FolderOpen data-icon="inline-start" />
          {busy === "choose" ? t("app.common.choosing") : t("app.actions.chooseFile")}
        </Button>
      </div>
      {validation ? (
        <div className="rounded-md border border-border bg-muted/55 p-3">
          <Badge variant={validation.safe ? "status" : "outline"}>{validation.safe ? t("app.common.safe") : t("app.common.rejected")}</Badge>
          <p className="mt-2 text-muted-foreground">{validation.message}</p>
          {importedPath ? <p className="mt-2 break-all text-xs text-muted-foreground">Stored at {importedPath}</p> : null}
        </div>
      ) : null}
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      <div className="flex flex-wrap gap-2">
        <Button disabled={busy !== null || !sourcePath.trim()} onClick={validateSource} size="sm" variant="outline">
          <CheckCircle2 data-icon="inline-start" />
          {busy === "validate" ? t("app.common.validating") : t("architect.theme.validate")}
        </Button>
        <Button disabled={busy !== null || !sourcePath.trim()} onClick={importSource} size="sm" variant="outline">
          <Upload data-icon="inline-start" />
          {busy === "import" ? t("app.common.importing") : t("architect.theme.import")}
        </Button>
        <Button disabled={!previewSource.trim()} onClick={() => onPreviewTheme("workspace", null, previewSource)} size="sm" variant="outline">
          <FileJson data-icon="inline-start" />
          Preview workspace
        </Button>
        <Button disabled={!previewSource.trim()} onClick={() => onApplyTheme("workspace", null, previewSource)} size="sm">
          Apply workspace
        </Button>
        {themePreview ? (
          <Button onClick={onCancelThemePreview} size="sm" variant="ghost">
            <X data-icon="inline-start" />
            {t("app.actions.cancel")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function WidgetLayoutImportPanel({ vaultPath }: { vaultPath?: string }) {
  const { t } = useI18n();
  const [sourcePath, setSourcePath] = useState("");
  const [exportPath, setExportPath] = useState("dashboard-widgets-export.json");
  const [result, setResult] = useState<ImportValidation | ImportResult | null>(null);
  const [exportResult, setExportResult] = useState<ImportResult | null>(null);
  const [busy, setBusy] = useState<"choose" | "validate" | "import" | "export" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const validation = result && "validation" in result ? result.validation : result;

  const chooseFile = async () => {
    setBusy("choose");
    setError(null);
    try {
      const selected = await selectImportFile("layout");
      if (selected) {
        setSourcePath(selected);
        setResult(null);
      }
    } catch (error) {
      setError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  const validateSource = async () => {
    setBusy("validate");
    setError(null);
    try {
      if (!vaultPath) {
        setError("Select or create a vault before validating Dashboard widget layout metadata.");
        return;
      }
      setResult(await validateWidgetLayoutImport(vaultPath, sourcePath));
    } catch (error) {
      setError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  const importSource = async () => {
    if (!vaultPath) {
      setError("Select or create a vault before importing Dashboard widget layout metadata.");
      return;
    }
    setBusy("import");
    setError(null);
    try {
      setResult(await importWidgetLayoutFile(vaultPath, sourcePath));
    } catch (error) {
      setError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  const exportLayout = async () => {
    if (!vaultPath) {
      setError("Select or create a vault before exporting Dashboard widget layout metadata.");
      return;
    }
    setBusy("export");
    setError(null);
    try {
      setExportResult(await exportWidgetLayoutFile(vaultPath, exportPath));
    } catch (error) {
      setError(messageFromUnknown(error));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="grid gap-3 rounded-md border border-border bg-background p-3 text-sm">
      <div className="flex flex-wrap items-center gap-2">
        <FileJson className="size-4 text-muted-foreground" aria-hidden="true" />
        <p className="font-medium">{t("architect.layout.title")}</p>
        <Badge variant="outline">{t("architect.layout.metadataOnly")}</Badge>
      </div>
      <p className="text-muted-foreground">{t("architect.layout.description")}</p>
      <div className="grid gap-3 rounded-md border border-border bg-muted/35 p-3">
        <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
          <div className="grid gap-2">
            <Label htmlFor="architect-layout-export-path">{t("architect.layout.exportPath")}</Label>
            <Input
              id="architect-layout-export-path"
              onChange={(event) => {
                setExportPath(event.target.value);
                setExportResult(null);
              }}
              placeholder="dashboard-widgets-export.json"
              value={exportPath}
            />
          </div>
          <Button className="self-end" disabled={busy !== null || !exportPath.trim()} onClick={exportLayout} size="sm" variant="outline">
            <FileJson data-icon="inline-start" />
            {busy === "export" ? t("app.common.exporting") : t("architect.layout.export")}
          </Button>
        </div>
        {exportResult ? (
          <p className="break-all text-xs text-muted-foreground">
            {t("architect.layout.exported")} {exportResult.stored_relative_path}.
          </p>
        ) : null}
      </div>
      <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
        <div className="grid gap-2">
          <Label htmlFor="architect-layout-import-path">{t("architect.layout.importPath")}</Label>
          <Input
            id="architect-layout-import-path"
            onChange={(event) => {
              setSourcePath(event.target.value);
              setResult(null);
            }}
            placeholder="dashboard-widgets.json"
            value={sourcePath}
          />
        </div>
        <Button className="self-end" disabled={busy !== null} onClick={chooseFile} variant="outline">
          <FolderOpen data-icon="inline-start" />
          {busy === "choose" ? t("app.common.choosing") : t("app.actions.chooseFile")}
        </Button>
      </div>
      {validation ? (
        <div className="rounded-md border border-border bg-muted/55 p-3">
          <Badge variant={validation.safe ? "status" : "outline"}>{validation.safe ? t("app.common.safe") : t("app.common.rejected")}</Badge>
          <p className="mt-2 text-muted-foreground">{validation.message}</p>
          {"stored_relative_path" in (result ?? {}) ? (
            <p className="mt-2 break-all text-xs text-muted-foreground">Stored at {(result as ImportResult).stored_relative_path}</p>
          ) : null}
        </div>
      ) : null}
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      <div className="flex flex-wrap gap-2">
        <Button disabled={busy !== null || !sourcePath.trim()} onClick={validateSource} size="sm" variant="outline">
          <CheckCircle2 data-icon="inline-start" />
          {busy === "validate" ? t("app.common.validating") : t("architect.layout.validate")}
        </Button>
        <Button disabled={busy !== null || !sourcePath.trim()} onClick={importSource} size="sm">
          <Upload data-icon="inline-start" />
          {busy === "import" ? t("app.common.importing") : t("architect.layout.import")}
        </Button>
      </div>
    </div>
  );
}

export function SchemasSection({
  moduleRegistry,
  schemaWarningModules,
}: {
  moduleRegistry: RegistryState | null;
  schemaWarningModules: RegistryState["modules"];
}) {
  const { t } = useI18n();
  const modules = moduleRegistry?.modules.filter((module) => module.schema_path) ?? [];
  return (
    <div className="grid gap-3">
      {schemaWarningModules.map((module) => (
        <div className="rounded-md border border-border bg-muted/45 p-3" key={module.id}>
          <div className="flex items-center gap-2 text-sm font-medium">
            <AlertTriangle className="size-4 text-amber-note-foreground" aria-hidden="true" />
            {module.display_name} schema
          </div>
          <p className="mt-1 text-sm text-muted-foreground">{moduleWarnings(module).join(" ")}</p>
        </div>
      ))}
      {!schemaWarningModules.length ? <p className="text-sm text-muted-foreground">{t("architect.schema.noWarnings")}</p> : null}
      <Separator />
      <div className="grid gap-2 md:grid-cols-2">
        {modules.map((module) => (
          <div className="rounded-md border border-border bg-background p-3" key={module.id}>
            <p className="text-sm font-medium">{module.display_name}</p>
            <p className="mt-1 text-xs text-muted-foreground">{t("architect.schema.version")}{module.schema_version ?? "unknown"} - {t("architect.schema.migration")} {module.schema_migration_version ?? "none"}</p>
            <p className="mt-2 break-all text-xs text-muted-foreground">{t("architect.schema.runtimePath")}: {module.schema_path}</p>
            <p className="mt-1 break-all text-xs text-muted-foreground">{t("architect.schema.sourcePath")}: {schemaSourcePath(module.id)}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function schemaSourcePath(moduleId: string) {
  return ["notes", "todos", "contacts", "habits"].includes(moduleId)
    ? `schemas/modules/${moduleId}.schema.json`
    : "custom/runtime-only module schema";
}

export function DataGraphSection({
  dashboardHub,
  navigator,
  onApplyEntityUpgrade,
  onNavigate,
  onPreviewEntityUpgrade,
  onRebuildNavigator,
  onSearchGraph,
  scan,
  searchSnapshot,
  upgradePreview,
}: {
  dashboardHub: DashboardHubDocument | null;
  navigator: NavigatorSnapshot | null;
  onApplyEntityUpgrade: () => void;
  onNavigate: (view: AppView, label?: string, options?: Partial<FocusTarget>) => void;
  onPreviewEntityUpgrade: () => void;
  onRebuildNavigator: () => void;
  onSearchGraph: (query: string) => void;
  scan: WorkspaceScanResult | null;
  searchSnapshot: SearchIndexSnapshot | null;
  upgradePreview: EntityUpgradePreview | null;
}) {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const resolvedPins = dashboardHub?.pinned_entities ?? [];
  const unresolvedPins = dashboardHub?.unresolved_pins ?? [];
  const backlinks = navigator?.backlinks ?? [];
  const graphWarnings = navigator?.health_warnings ?? [];
  const searchEntries = searchSnapshot?.entries ?? [];
  const upgradeChanges = upgradePreview?.changes ?? [];
  return (
    <div className="grid gap-3 text-sm">
      <GraphDetailSection count={scan?.documents.length ?? 0} title={t("architect.graph.scanned")}>
        {scan?.documents.length ? (
          scan.documents.slice(0, 12).map((document) => (
            <DetailRow
              key={document.markdown_relative_path}
              title={document.title}
              meta={`${document.document_type} - ${document.status}`}
              path={document.markdown_relative_path}
            />
          ))
        ) : (
          <EmptyDetail>{t("architect.graph.noScanned")}</EmptyDetail>
        )}
      </GraphDetailSection>
      <GraphDetailSection count={scan?.issues.length ?? 0} title={t("architect.graph.workspaceIssues")}>
        {scan?.issues.length ? (
          scan.issues.map((issue, index) => (
            <DetailRow
              key={`${issue.code}-${issue.markdown_relative_path ?? index}`}
              actionLabel={issue.suggested_action ? t("architect.graph.openRecovery") : undefined}
              onAction={issue.suggested_action ? () => onNavigate("architect", t("nav.recovery"), { architectTab: "recovery" }) : undefined}
              title={issue.code.replace(/_/g, " ")}
              meta={issue.suggested_action ?? issue.message}
              path={issue.markdown_relative_path ?? issue.document_id ?? "workspace"}
            />
          ))
        ) : (
          <EmptyDetail>{t("architect.graph.noIssues")}</EmptyDetail>
        )}
      </GraphDetailSection>
      <GraphDetailSection count={resolvedPins.length} title={t("architect.graph.resolvedPins")}>
        {resolvedPins.length ? resolvedPins.map((pin) => <DetailRow key={pin.document_id} title={pin.title} meta={pin.entity_type} path={pin.markdown_relative_path} />) : <EmptyDetail>{t("architect.graph.noResolvedPins")}</EmptyDetail>}
      </GraphDetailSection>
      <GraphDetailSection count={unresolvedPins.length} title={t("architect.graph.unresolvedPins")}>
        {unresolvedPins.length ? unresolvedPins.map((pin) => <DetailRow key={pin} title={t("architect.graph.unresolvedPins")} meta={t("architect.modules.needsAttention")} path={pin} />) : <EmptyDetail>{t("architect.graph.noUnresolvedPins")}</EmptyDetail>}
      </GraphDetailSection>
      <GraphDetailSection count={backlinks.length} title={t("architect.graph.backlinks")}>
        {backlinks.length ? backlinks.map((link, index) => (
          <DetailRow key={`${link.source_path}-${link.target}-${index}`} title={link.raw} meta={`${link.link_type} - ${link.status}`} path={link.resolved_path ?? link.source_path} />
        )) : <EmptyDetail>{t("architect.graph.noBacklinks")}</EmptyDetail>}
      </GraphDetailSection>
      <GraphDetailSection count={graphWarnings.length} title={t("architect.graph.healthWarnings")}>
        {graphWarnings.length ? graphWarnings.map((warning, index) => (
          <DetailRow key={`${warning.code}-${warning.path ?? index}`} title={warning.code.replace(/_/g, " ")} meta={warning.message} path={warning.path ?? warning.document_id ?? "graph"} />
        )) : <EmptyDetail>{t("architect.graph.clean")}</EmptyDetail>}
      </GraphDetailSection>
      <div className="grid gap-2 rounded-md border border-border bg-background p-3">
        <p className="font-medium">{t("architect.graph.globalSearch")}</p>
        <div className="flex flex-wrap gap-2">
          <input
            aria-label={t("architect.graph.globalSearch")}
            className="h-9 min-w-0 flex-1 rounded-md border border-input bg-background px-3 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("architect.graph.searchPlaceholder")}
            value={query}
          />
          <Button disabled={!query.trim()} onClick={() => onSearchGraph(query)} size="sm" variant="outline">{t("app.actions.search")}</Button>
        </div>
      </div>
      <GraphDetailSection count={searchEntries.length} title={t("architect.graph.searchResults")}>
        {searchSnapshot ? (
          searchEntries.length ? searchEntries.map((entry) => (
            <DetailRow
              key={entry.path}
              actionLabel={t("architect.graph.open")}
              onAction={() => {
                const moduleId = moduleIdForSearchEntry(entry.entity_type, entry.path);
                onNavigate(viewForModule(moduleId), entry.title, {
                  documentId: entry.document_id ?? undefined,
                  moduleId,
                });
              }}
              title={entry.title}
              meta={`${entry.entity_type} - ${entry.excerpt}`}
              path={entry.path}
            />
          )) : <EmptyDetail>{t("architect.graph.noResults")}</EmptyDetail>
        ) : (
          <EmptyDetail>{t("architect.graph.runSearch")}</EmptyDetail>
        )}
      </GraphDetailSection>
      <GraphDetailSection count={upgradeChanges.length} title={t("architect.graph.entityUpgradePreview")}>
        {upgradePreview ? (
          upgradeChanges.length ? upgradeChanges.map((change) => (
            <DetailRow key={`${change.source_path}-${change.target_path}`} title={change.title} meta={`${change.entity_type} - ${change.action}`} path={`${change.source_path} -> ${change.target_path}`} />
          )) : <EmptyDetail>{t("architect.graph.noUpgrades")}</EmptyDetail>
        ) : (
          <EmptyDetail>{t("architect.graph.previewToInspect")}</EmptyDetail>
        )}
      </GraphDetailSection>
      <div className="flex flex-wrap gap-2">
        <Button onClick={onRebuildNavigator} size="sm" variant="outline">{t("architect.graph.rebuild")}</Button>
        <Button onClick={onPreviewEntityUpgrade} size="sm" variant="outline">{t("architect.graph.previewUpgrade")}</Button>
        <Button disabled={!upgradePreview?.changes.length} onClick={onApplyEntityUpgrade} size="sm">{t("architect.graph.applyUpgrade")}</Button>
      </div>
    </div>
  );
}

function GraphDetailSection({ children, count, title }: { children: ReactNode; count: number; title: string }) {
  return (
    <details className="rounded-md border border-border bg-background p-3">
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 text-sm font-medium">
        <span>{title}</span>
        <Badge variant={count ? "secondary" : "outline"}>{count}</Badge>
      </summary>
      <div className="mt-3 grid gap-2">{children}</div>
    </details>
  );
}

function moduleIdForSearchEntry(entityType: string, path: string) {
  const normalized = `${entityType} ${path}`.toLowerCase();
  if (normalized.includes("todo")) return "todos";
  if (normalized.includes("contact")) return "contacts";
  if (normalized.includes("habit")) return "habits";
  return "notes";
}

function DetailRow({
  actionLabel,
  meta,
  onAction,
  path,
  title,
}: {
  actionLabel?: string;
  meta: string;
  onAction?: () => void;
  path: string;
  title: string;
}) {
  return (
    <div className="rounded-md border border-border bg-muted/35 p-3">
      <div className="flex items-start justify-between gap-3">
        <p className="font-medium capitalize">{title}</p>
        {actionLabel && onAction ? (
          <Button onClick={onAction} size="sm" variant="ghost">
            {actionLabel}
          </Button>
        ) : null}
      </div>
      <p className="mt-1 text-xs text-muted-foreground">{meta}</p>
      <p className="mt-2 break-all text-xs text-muted-foreground">{path}</p>
    </div>
  );
}

function EmptyDetail({ children }: { children: ReactNode }) {
  return <p className="rounded-md border border-dashed border-border bg-muted/25 p-3 text-xs text-muted-foreground">{children}</p>;
}

function ModuleGroup({
  defaultOpen,
  modules,
  onConfigure,
  onSectionChange,
  onToggleModule,
  sectionKey,
  title,
  widgetTypes,
  workspaceUiState,
}: {
  defaultOpen: boolean;
  modules: RegistryState["modules"];
  onConfigure: (moduleId: string, label: string) => void;
  onSectionChange: (section: string, expanded: boolean) => void;
  onToggleModule?: (moduleId: string, enabled: boolean) => void;
  sectionKey: string;
  title: string;
  widgetTypes: WidgetTypeDefinition[];
  workspaceUiState: WorkspaceUiState | null;
}) {
  const { t } = useI18n();
  const needsAttention = modules.some(moduleNeedsAttention);
  const open = needsAttention || Boolean(workspaceUiState?.architect_sections?.[sectionKey] ?? defaultOpen);
  return (
    <details
      className="rounded-md border border-border bg-background p-3"
      onToggle={(event) => onSectionChange(sectionKey, event.currentTarget.open)}
      open={open}
    >
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 text-sm font-semibold">
        <span>{title}</span>
        <span className="flex items-center gap-2">
          {needsAttention ? <Badge variant="outline">{t("architect.modules.needsAttention")}</Badge> : null}
          <Badge variant={modules.length ? "secondary" : "outline"}>{modules.length}</Badge>
        </span>
      </summary>
      <div className="mt-3 grid gap-3 md:grid-cols-2">
        {modules.map((module) => (
          <ArchitectModuleCard
            key={module.id}
            module={module}
            onConfigure={() => onConfigure(module.id, module.display_name)}
            onToggle={onToggleModule ? () => onToggleModule(module.id, !module.enabled) : undefined}
            widgetCount={widgetTypes.filter((widgetType) => widgetType.module_id === module.id).length}
          />
        ))}
        {!modules.length ? <p className="text-sm text-muted-foreground">{t("architect.modules.noGroup")}</p> : null}
      </div>
    </details>
  );
}

function ArchitectModuleCard({
  module,
  onConfigure,
  onToggle,
  widgetCount,
}: {
  module: RegistryState["modules"][number];
  onConfigure: () => void;
  onToggle?: () => void;
  widgetCount: number;
}) {
  const { t } = useI18n();
  const needsAttention = moduleWarnings(module).length > 0 || !module.available || !module.installed;
  const canUseModule = module.available && module.installed;

  return (
    <div className="rounded-md border border-border bg-background p-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{module.display_name}</p>
          <p className="mt-1 break-all text-xs text-muted-foreground">{module.index_path}</p>
          <p className="mt-2 text-xs text-muted-foreground">
            {t("architect.modules.disablingNote")}
          </p>
        </div>
        <div className="flex flex-col items-end gap-1.5">
          {module.kind === "system" || module.kind === "starter" ? <Badge variant="secondary">{t("architect.modules.builtIn")}</Badge> : null}
          {module.installed ? <Badge variant="secondary">{t("architect.modules.installed")}</Badge> : null}
          {needsAttention ? <Badge variant="outline">{t("architect.modules.needsAttention")}</Badge> : null}
          <Badge variant={module.enabled ? "default" : "secondary"}>{module.enabled ? t("app.common.enabled") : t("app.common.disabled")}</Badge>
          {widgetCount ? <Badge variant="outline">{widgetCount} {t("architect.modules.widgets")}</Badge> : null}
        </div>
      </div>
      <div className="mt-3 flex gap-2">
        <Button
          aria-label={`${t("architect.graph.open")} ${module.display_name}`}
          disabled={!canUseModule}
          onClick={onConfigure}
          size="icon"
          title={`${t("architect.graph.open")} ${module.display_name}`}
          variant="outline"
        >
          <ChevronRight aria-hidden="true" />
        </Button>
        {onToggle ? (
          <Button disabled={!canUseModule} onClick={onToggle} size="sm" variant={module.enabled ? "outline" : "default"}>
            {module.enabled ? t("architect.modules.disable") : t("architect.modules.enable")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function moduleWarnings(module: RegistryState["modules"][number]) {
  return Array.isArray(module.schema_warnings) ? module.schema_warnings : [];
}

function moduleNeedsAttention(module: RegistryState["modules"][number]) {
  return moduleWarnings(module).length > 0 || !module.available || !module.installed;
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3 rounded-md border border-border bg-background px-3 py-2">
      <span className="truncate text-muted-foreground">{label}</span>
      <span className="truncate font-medium">{value}</span>
    </div>
  );
}

function messageFromUnknown(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
