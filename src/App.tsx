import {
  AlertTriangle,
  Archive,
  ArrowLeft,
  BookOpenText,
  CalendarCheck,
  CheckCircle2,
  CheckSquare,
  ChevronRight,
  FolderPlus,
  FolderOpen,
  Leaf,
  Menu,
  Network,
  Palette,
  Pencil,
  Plus,
  Package,
  RefreshCw,
  Search,
  Settings,
  SlidersHorizontal,
  Sparkles,
  Trash2,
  Users,
} from "lucide-react";
import { type CSSProperties, useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  buildModuleNavEntries,
  buildSystemNavEntries,
  normalizeNavigationTarget,
  type AppView,
  type FocusTarget,
  type ModuleNavEntry,
  viewForModule,
  viewLabels,
} from "@/state/navigation";
import {
  emptyContactDocument,
  emptyDashboardHubDocument,
  emptyDashboardWidgetState,
  emptyHabitDocument,
  emptyNavigatorSnapshot,
  emptyRegistryState,
  emptyWorkspaceRecoveryPreview,
  defaultWorkspaceUiState,
} from "@/state/workspace/workspaceDefaults";
import {
  loadWorkspaceResource,
  type WorkspaceResourceErrors,
} from "@/state/workspace/workspaceResources";
import {
  resolveSelectedContactId,
  resolveSelectedHabitId,
  resolveSelectedNoteId,
  resolveSelectedTodoId,
  selectActiveModuleId,
  selectContactById,
  selectHabitById,
  selectModuleErrors,
} from "@/state/workspace/workspaceSelectors";
import { applyPerfToggles, parsePerfToggles } from "@/state/performance/perfToggles";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ChecklistField } from "@/components/forms/ChecklistField";
import { DateField } from "@/components/forms/DateField";
import { EntityLinksField } from "@/components/forms/EntityLinksField";
import { SelectField } from "@/components/forms/SelectField";
import { TagsField } from "@/components/forms/TagsField";
import { TextAreaField } from "@/components/forms/TextAreaField";
import { TextField } from "@/components/forms/TextField";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Empty } from "@/components/ui/empty";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { useToast, type AppNoticeKind } from "@/components/ui/toast";
import { I18nProvider, normalizeLanguage, useI18n, type AppLanguage, type TranslationKey } from "@/i18n";
import { en } from "@/i18n/locales/en";
import { vi } from "@/i18n/locales/vi";
import {
  CONTACT_RELATIONSHIP_OPTIONS,
  HABIT_FREQUENCY_OPTIONS,
  TODO_PRIORITY_OPTIONS,
  TODO_STATUS_OPTIONS,
} from "@/domain/entities";
import {
  clearStoredVaultPath,
  createDefaultVault,
  getDefaultVaultPath,
  getStoredVaultPath,
  inspectVault,
  repairVaultStructure,
  selectVaultFolder,
  storeVaultPath,
  type VaultInspection,
} from "@/services/vault";
import {
  createNote,
  listNotes,
  orphanMissingDocumentMetadata,
  previewWorkspaceRecovery,
  readNote,
  recoverDocumentMetadata,
  recoverLayoutMetadata,
  renameNote,
  repairDocumentFrontmatterReference,
  restoreOrphanedDocumentMetadata,
  saveMarkdownAsset,
  scanWorkspace,
  updateNote,
  type NoteDocument,
  type NoteSummary,
  type RecoveryIssue,
  type WorkspaceRecoveryPreview,
  type WorkspaceScanResult,
} from "@/services/notes";
import { createTodo, readTodo, listTodos, updateTodo, renameTodo, type TodoDocument, type TodoSummary } from "@/services/todo";
import { TodosGeneratedUI } from "@/components/modules/TodosGeneratedUI";
import { ContactsGeneratedUI } from "@/components/modules/ContactsGeneratedUI";
import { HabitsGeneratedUI } from "@/components/modules/HabitsGeneratedUI";
import { EntityEditDrawer, type RawConflictChoice } from "@/components/modules/EntityEditDrawer";
import { BentoLifeBrandMark } from "@/components/brand/BentoLifeBrandMark";
import {
  createContact,
  readContacts,
  updateContact,
  type ContactDocument,
  type ContactEntry,
  type ContactInput,
} from "@/services/contacts";
import {
  createHabit,
  readHabits,
  recordHabitCheckin,
  updateHabit,
  type HabitDocument,
  type HabitEntry,
  type HabitInput,
} from "@/services/habits";
import {
  applyEntityUpgrade,
  loadModuleRegistry,
  setModuleEnabled,
  previewEntityUpgrade,
  readNavigator,
  scanAndRebuildNavigator,
  searchEntities,
  type EntityUpgradePreview,
  type NavigatorSnapshot,
  type RegistryState,
  type SearchIndexSnapshot,
} from "@/services/backendCore";
import { createVaultSnapshot, type VaultSnapshot } from "@/state/vault-onboarding";
import {
  pinDashboardEntity,
  readDashboardHub,
  unpinDashboardEntity,
  type DashboardHubDocument,
  type DashboardPinnedEntity,
} from "@/services/dashboard";
import {
  applyThemeTokens,
  effectiveTokens,
  previewThemeTokens,
  readActiveTheme,
  rollbackTheme,
  type ActiveThemeState,
  type ThemePreview,
  type ThemeScope,
  type ThemeTokenMap,
} from "@/services/theme";
import { GeneratedModuleUI } from "@/components/modules/GeneratedModuleUI";
import { ModuleErrorBoundary } from "@/components/system/ModuleErrorBoundary";
import { NotesPanel } from "@/components/modules/NotesPanel";
import { ArchitectPanel as ArchitectControlPanel } from "@/components/architect/ArchitectPanel";
import { ArchivePanel } from "@/components/archive/ArchivePanel";
import { DashboardHub } from "@/components/dashboard/DashboardHub";
import { RecoveryPanel } from "@/components/recovery/RecoveryPanel";
import { SettingsPanel } from "@/components/settings/SettingsPanel";
import { TrashPanel } from "@/components/trash/TrashPanel";
import { VaultStatusBlock, VaultStatusPanel } from "@/components/vault/VaultStatusPanel";
import { type WidgetActions } from "@/components/widgets/WidgetCanvas";
import type { WidgetInteractionHandlers } from "@/services/widgetRendererRegistry";
import {
  createDashboardWidget,
  compactDashboardWidgets,
  duplicateDashboardWidget,
  isDashboardWidgetSparseRepairWarning,
  loadWorkspaceUiState,
  moveDashboardWidget,
  readDashboardWidgets,
  readWidgetTypeRegistry,
  removeDashboardWidget,
  resetDashboardWidgets,
  resizeDashboardWidget,
  saveWorkspaceUiState,
  setDashboardWidgetCollapsed,
  updateDashboardWidget,
  type ArchitectTabId,
  type DashboardWidgetState,
  type WorkspaceUiState,
  type WidgetTypeDefinition,
} from "@/services/widgets";

const iconByView: Record<AppView, typeof BookOpenText> = {
  dashboard: Sparkles,
  notes: BookOpenText,
  todos: CheckSquare,
  contacts: Users,
  habits: Leaf,
  navigator: Network,
  architect: SlidersHorizontal,
  vault: FolderOpen,
  settings: Settings,
  trash: Trash2,
  archive: Archive,
  module: Package,
};

const navLabelKeys: Record<AppView, TranslationKey> = {
  dashboard: "nav.dashboard",
  notes: "nav.notes",
  todos: "nav.todos",
  contacts: "nav.contacts",
  habits: "nav.habits",
  navigator: "nav.navigator",
  architect: "nav.architect",
  vault: "nav.vault",
  settings: "nav.settings",
  trash: "nav.trash",
  archive: "nav.archive",
  module: "nav.module",
};

const appDictionaries = { en, vi };

function translateApp(language: AppLanguage, key: TranslationKey, values: Record<string, string | number> = {}) {
  const template = appDictionaries[language]?.[key] ?? en[key] ?? key;
  return Object.entries(values).reduce((message, [name, value]) => message.split(`{${name}}`).join(`${value}`), template);
}

type ModuleBacklink = NavigatorSnapshot["backlinks"][number];

const RAIL_STORAGE_KEY = "bentolife:calm-shell:rail-expanded";

function App() {
  const [activeView, setActiveView] = useState<AppView>("dashboard");
  const [vaultSnapshot, setVaultSnapshot] = useState<VaultSnapshot>({
    defaultPath: "",
    stage: "checking",
  });
  const [vaultAction, setVaultAction] = useState<"create" | "select" | "repair" | "reset" | null>(null);
  const [workspaceScan, setWorkspaceScan] = useState<WorkspaceScanResult | null>(null);
  const [recoveryPreview, setRecoveryPreview] = useState<WorkspaceRecoveryPreview | null>(null);
  const [noteSummaries, setNoteSummaries] = useState<NoteSummary[]>([]);
  const [todoSummaries, setTodoSummaries] = useState<TodoSummary[]>([]);
  const [selectedTodo, setSelectedTodo] = useState<TodoDocument | null>(null);
  const [contactDocument, setContactDocument] = useState<ContactDocument | null>(null);
  const [habitDocument, setHabitDocument] = useState<HabitDocument | null>(null);
  const [navigatorSnapshot, setNavigatorSnapshot] = useState<NavigatorSnapshot | null>(null);
  const [dashboardHub, setDashboardHub] = useState<DashboardHubDocument | null>(null);
  const [dashboardWidgetState, setDashboardWidgetState] = useState<DashboardWidgetState | null>(null);
  const [workspaceUiState, setWorkspaceUiState] = useState<WorkspaceUiState | null>(null);
  const [architectActiveTab, setArchitectActiveTab] = useState<ArchitectTabId>("modules");
  const [widgetTypes, setWidgetTypes] = useState<WidgetTypeDefinition[]>([]);
  const [activeTheme, setActiveTheme] = useState<ActiveThemeState | null>(null);
  const [themePreview, setThemePreview] = useState<ThemePreview | null>(null);
  const [searchSnapshot, setSearchSnapshot] = useState<SearchIndexSnapshot | null>(null);
  const [upgradePreview, setUpgradePreview] = useState<EntityUpgradePreview | null>(null);
  const [moduleRegistry, setModuleRegistry] = useState<RegistryState | null>(null);
  const [focusTarget, setFocusTarget] = useState<FocusTarget>({ label: "Dashboard", view: "dashboard" });
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);
  const [selectedNote, setSelectedNote] = useState<NoteDocument | null>(null);
  const [selectedContactId, setSelectedContactId] = useState<string | null>(null);
  const [selectedHabitId, setSelectedHabitId] = useState<string | null>(null);
  const [workspaceLoading, setWorkspaceLoading] = useState(false);
  const [workspaceError, setWorkspaceError] = useState<string | null>(null);
  const [workspaceResourceErrors, setWorkspaceResourceErrors] = useState<WorkspaceResourceErrors>({});
  const { showToast } = useToast();
  const [railExpanded, setRailExpanded] = useState(() => {
    if (typeof window === "undefined") return false;
    return window.localStorage.getItem(RAIL_STORAGE_KEY) === "true";
  });
  const [lastScanAt, setLastScanAt] = useState<Date | null>(null);
  const returnFocusRef = useRef<HTMLElement | null>(null);

  const vaultPath = vaultSnapshot.inspection?.path;
  const language = normalizeLanguage(workspaceUiState?.language);
  const rootT = useCallback(
    (key: TranslationKey, values?: Record<string, string | number>) => translateApp(language, key, values),
    [language],
  );
  const showOperationNotice = useCallback((message: string, kind: AppNoticeKind = "info") => {
    showToast({ kind, message, title: kind === "error" ? rootT("toast.actionFailed") : rootT("toast.updated") });
  }, [rootT, showToast]);
  const activeModuleId = selectActiveModuleId(activeView, focusTarget);
  const shellThemeTokens = useMemo(
    () => {
      const baseTokens = activeTheme ? effectiveTokens(activeTheme, activeModuleId) : {};
      const previewApplies =
        themePreview?.scope === "workspace" ||
        (themePreview?.scope === "module" && themePreview.module_id === activeModuleId);
      return {
        ...baseTokens,
        ...(previewApplies ? themePreview.tokens : {}),
      };
    },
    [activeModuleId, activeTheme, themePreview],
  );
  const shellStyle = useMemo(() => tokensToStyle(shellThemeTokens), [shellThemeTokens]);
  const visibleShortcuts = useMemo(() => buildModuleNavEntries(moduleRegistry), [moduleRegistry]);
  const systemShortcuts = useMemo(() => buildSystemNavEntries(moduleRegistry), [moduleRegistry]);
  const widgetContext = useMemo(
    () => ({
      contacts: contactDocument,
      habits: habitDocument,
      notes: noteSummaries,
      todos: selectedTodo,
      todoSummaries,
    }),
    [contactDocument, habitDocument, noteSummaries, selectedTodo, todoSummaries],
  );

  const handleLanguageChange = useCallback(
    (nextLanguage: AppLanguage) => {
      const nextState: WorkspaceUiState = {
        schema_version: 1,
        workspace_name: "BentoLife",
        default_theme: "clean-slate",
        architect_sections: {},
        updated_at: new Date().toISOString(),
        ...(workspaceUiState ?? {}),
        language: nextLanguage,
      };
      setWorkspaceUiState(nextState);
      if (!vaultPath) {
        return;
      }
      void saveWorkspaceUiState(vaultPath, nextState)
        .then(setWorkspaceUiState)
        .catch((error) => showOperationNotice(rootT("notice.languageSaveFailed", { error: getErrorMessage(error) }), "error"));
    },
    [rootT, showOperationNotice, vaultPath, workspaceUiState],
  );

  useEffect(() => {
    window.localStorage.setItem(RAIL_STORAGE_KEY, String(railExpanded));
  }, [railExpanded]);

  useEffect(() => {
    const root = document.documentElement;
    applyPerfToggles(root, parsePerfToggles(window.location.search));
  }, []);

  useEffect(() => {
    let alive = true;

    async function loadVault() {
      try {
        const defaultPath = await getDefaultVaultPath();
        const selectedPath = getStoredVaultPath();

        if (!selectedPath) {
          if (alive) {
            setVaultSnapshot({ defaultPath, stage: "missing" });
          }
          return;
        }

        const inspection = await inspectVault(selectedPath);
        if (alive) {
          setVaultSnapshot(createVaultSnapshot({ defaultPath, inspection, selectedPath }));
        }
      } catch (error) {
        if (alive) {
          setVaultSnapshot({
            defaultPath: "",
            error: getErrorMessage(error),
            stage: "error",
          });
        }
      }
    }

    loadVault();

    return () => {
      alive = false;
    };
  }, []);

  useEffect(() => {
    if (vaultSnapshot.stage !== "ready" || !vaultPath) {
      return;
    }

    refreshWorkspace(vaultPath);
  }, [vaultSnapshot.stage, vaultPath]);

  useEffect(() => {
    if (isArchitectTabId(workspaceUiState?.architect_active_tab)) {
      setArchitectActiveTab(workspaceUiState.architect_active_tab);
    }
  }, [workspaceUiState?.architect_active_tab]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const isCommandPaletteShortcut = (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k";
      if (!isCommandPaletteShortcut) {
        return;
      }
      const target = event.target as HTMLElement | null;
      if (target?.matches("textarea, input, [contenteditable='true']")) {
        return;
      }
      event.preventDefault();
      setCommandPaletteOpen((open) => !open);
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const refreshWorkspace = async (path: string, preferredNoteId?: string) => {
    const startedAt = performance.now();
    setWorkspaceLoading(true);
    setWorkspaceError(null);
    setWorkspaceResourceErrors({});
    try {
      const today = todayKey();
      const scan = await scanWorkspace(path);
      setWorkspaceScan(scan);
      setLastScanAt(new Date());
      const [hubResource, notesResource, todosResource, contactsResource, habitsResource, navigatorResource, recoveryResource] =
        await Promise.all([
          loadWorkspaceResource(() => readDashboardHub(path), emptyDashboardHubDocument()),
          loadWorkspaceResource(() => listNotes(path), []),
          loadWorkspaceResource(() => listTodos(path), []),
          loadWorkspaceResource(() => readContacts(path), emptyContactDocument()),
          loadWorkspaceResource(() => readHabits(path, today), emptyHabitDocument()),
          loadWorkspaceResource(() => readNavigator(path), emptyNavigatorSnapshot(path)),
          loadWorkspaceResource(() => previewWorkspaceRecovery(path), emptyWorkspaceRecoveryPreview(path)),
        ]);
      const widgetHydrationStartedAt = performance.now();
      const [themeResource, registryResource, widgetRegistryResource, widgetsResource, uiStateResource] = await Promise.all([
        loadWorkspaceResource(() => readActiveTheme(path), null),
        loadWorkspaceResource(() => loadModuleRegistry(path), emptyRegistryState()),
        loadWorkspaceResource(() => readWidgetTypeRegistry(path), []),
        loadWorkspaceResource(
          () => readDashboardWidgets(path).finally(() => logV5Timing("dashboard widget hydration", widgetHydrationStartedAt, {})),
          emptyDashboardWidgetState(rootT("notice.dashboardWidgetMetadataLoadFailed", { error: "unknown" })),
        ),
        loadWorkspaceResource(() => loadWorkspaceUiState(path), null),
      ]);
      const metadataWarnings: string[] = [];
      const nextResourceErrors: WorkspaceResourceErrors = {};
      const rememberResourceError = (resource: keyof WorkspaceResourceErrors, error: string | null) => {
        if (error) nextResourceErrors[resource] = error;
      };

      rememberResourceError("dashboardHub", hubResource.error);
      rememberResourceError("notes", notesResource.error);
      rememberResourceError("todos", todosResource.error);
      rememberResourceError("contacts", contactsResource.error);
      rememberResourceError("habits", habitsResource.error);
      rememberResourceError("navigator", navigatorResource.error);
      rememberResourceError("recovery", recoveryResource.error);
      rememberResourceError("theme", themeResource.error);
      rememberResourceError("moduleRegistry", registryResource.error);
      rememberResourceError("widgetRegistry", widgetRegistryResource.error);
      rememberResourceError("dashboardWidgets", widgetsResource.error);
      rememberResourceError("workspaceUiState", uiStateResource.error);

      const hub = hubResource.data;
      const notes = notesResource.data;
      const todos = todosResource.data;
      const contacts = contactsResource.data;
      const habits = habitsResource.data;
      const navigator = navigatorResource.data;
      const recovery = recoveryResource.data;
      setDashboardHub(hub);
      setNoteSummaries(notes);
      setTodoSummaries(todos);
      const nextDocumentId =
        preferredNoteId ??
        selectedDocumentId ??
        scan.documents.find((document) => document.document_id)?.document_id ??
        null;
      setSelectedDocumentId(nextDocumentId);

      const nextTodoId = resolveSelectedTodoId(todos, selectedTodo, nextDocumentId, preferredNoteId);
      if (nextTodoId) {
        try {
          setSelectedTodo(await readTodo(path, nextTodoId));
        } catch (error) {
          nextResourceErrors.todos = getErrorMessage(error);
          setSelectedTodo(null);
        }
      } else {
        setSelectedTodo(null);
      }
      setContactDocument(contacts);
      setHabitDocument(habits);
      setNavigatorSnapshot(navigator);
      setRecoveryPreview(recovery);
      if (themeResource.status !== "degraded") {
        setActiveTheme(themeResource.data);
      } else {
        setActiveTheme(null);
        metadataWarnings.push(`theme state: ${themeResource.error}`);
      }
      if (registryResource.status !== "degraded") {
        setModuleRegistry(registryResource.data);
      } else {
        setModuleRegistry(registryResource.data);
        metadataWarnings.push(`module registry: ${registryResource.error}`);
      }
      if (widgetRegistryResource.status !== "degraded") {
        setWidgetTypes(widgetRegistryResource.data);
      } else {
        setWidgetTypes(widgetRegistryResource.data);
        metadataWarnings.push(`widget types: ${widgetRegistryResource.error}`);
      }
      if (widgetsResource.status !== "degraded") {
        setDashboardWidgetState(widgetsResource.data);
        const repairWarning = widgetsResource.data.warnings.find(isDashboardWidgetSparseRepairWarning);
        if (repairWarning) {
          showOperationNotice(rootT("notice.dashboardWidgetLayoutRepaired"), "warning");
        }
      } else {
        setDashboardWidgetState(emptyDashboardWidgetState(rootT("notice.dashboardWidgetMetadataLoadFailed", { error: widgetsResource.error })));
        metadataWarnings.push(`dashboard widgets: ${widgetsResource.error}`);
      }
      if (uiStateResource.status !== "degraded") {
        setWorkspaceUiState(uiStateResource.data);
      } else {
        setWorkspaceUiState(uiStateResource.data);
        metadataWarnings.push(`workspace UI state: ${uiStateResource.error}`);
      }
      if (metadataWarnings.length) {
        showOperationNotice(rootT("notice.workspaceLoadedWithMetadataIssues", { issues: metadataWarnings.join("; ") }), "warning");
      }

      const nextNoteId = resolveSelectedNoteId(notes, selectedNote, nextDocumentId, preferredNoteId);
      if (nextNoteId) {
        try {
          setSelectedNote(await readNote(path, nextNoteId));
        } catch (error) {
          nextResourceErrors.notes = getErrorMessage(error);
          setSelectedNote(null);
        }
      } else {
        setSelectedNote(null);
      }

      setSelectedContactId((current) => resolveSelectedContactId(contacts, current));
      setSelectedHabitId((current) => resolveSelectedHabitId(habits, current));
      logV5Timing("workspace refresh", startedAt, {
        contacts: contacts.contacts.length,
        documents: scan.documents.length,
        habits: habits.habits.length,
        notes: notes.length,
        todos: todos.length,
        widgets: widgetsResource.data.instances.length,
      });
      setWorkspaceResourceErrors(nextResourceErrors);
    } catch (error) {
      setWorkspaceError(getErrorMessage(error));
    } finally {
      setWorkspaceLoading(false);
    }
  };

  const handleCreateDefaultVault = async () => {
    await runVaultAction("create", async () => {
      const inspection = await createDefaultVault();
      storeVaultPath(inspection.path);
      return inspection;
    });
  };

  const handleSelectVault = async () => {
    setVaultAction("select");
    try {
      const selectedPath = await selectVaultFolder();
      if (!selectedPath) {
        setVaultAction(null);
        return;
      }

      const inspection = await inspectVault(selectedPath);
      storeVaultPath(selectedPath);
      setVaultSnapshot(
        createVaultSnapshot({
          defaultPath: vaultSnapshot.defaultPath || (await getDefaultVaultPath()),
          inspection,
          selectedPath,
        }),
      );
    } catch (error) {
      setVaultSnapshot({
        ...vaultSnapshot,
        error: getErrorMessage(error),
        stage: "error",
      });
    } finally {
      setVaultAction(null);
    }
  };

  const handleRepairVault = async () => {
    const path = vaultSnapshot.selectedPath ?? vaultSnapshot.inspection?.path;
    if (!path) {
      return;
    }

    await runVaultAction("repair", async () => repairVaultStructure(path));
  };

  const handleResetVault = async () => {
    setVaultAction("reset");
    clearStoredVaultPath();
    const defaultPath = vaultSnapshot.defaultPath || (await getDefaultVaultPath());
    setVaultSnapshot({ defaultPath, stage: "missing" });
    setWorkspaceScan(null);
    setRecoveryPreview(null);
    setNoteSummaries([]);
    setTodoSummaries([]);
    setSelectedTodo(null);
    setContactDocument(null);
    setHabitDocument(null);
    setNavigatorSnapshot(null);
    setDashboardHub(null);
    setDashboardWidgetState(null);
    setWidgetTypes([]);
    setActiveTheme(null);
    setThemePreview(null);
    setSearchSnapshot(null);
    setUpgradePreview(null);
    setFocusTarget({ label: "Dashboard", view: "dashboard" });
    setSelectedDocumentId(null);
    setSelectedNote(null);
    setSelectedContactId(null);
    setSelectedHabitId(null);
    setActiveView("dashboard");
    setVaultAction(null);
  };

  const runVaultAction = async (
    action: "create" | "repair",
    operation: () => Promise<VaultInspection>,
  ) => {
    setVaultAction(action);
    try {
      const inspection = await operation();
      storeVaultPath(inspection.path);
      setVaultSnapshot(
        createVaultSnapshot({
          defaultPath: vaultSnapshot.defaultPath || (await getDefaultVaultPath()),
          inspection,
          selectedPath: inspection.path,
        }),
      );
    } catch (error) {
      setVaultSnapshot({
        ...vaultSnapshot,
        error: getErrorMessage(error),
        stage: "error",
      });
    } finally {
      setVaultAction(null);
    }
  };

  const handleCreateNote = async (title = "Untitled Note", markdownBody?: string) => {
    if (!vaultPath) {
      return null;
    }
    try {
      const note = await createNote(vaultPath, title, markdownBody);
      navigateTo("notes", note.title, { documentId: note.document_id, moduleId: "notes" });
      await refreshWorkspace(vaultPath, note.document_id);
      return note;
    } catch (error) {
      showOperationNotice(rootT("notice.createNoteFailed", { error: getErrorMessage(error) }), "error");
      return null;
    }
  };

  const persistArchitectTab = (tab: ArchitectTabId) => {
    if (!vaultPath) {
      return;
    }
    const baseState = workspaceUiState ?? defaultWorkspaceUiState();
    if (baseState.architect_active_tab === tab) {
      return;
    }
    const nextState: WorkspaceUiState = {
      ...baseState,
      architect_active_tab: tab,
    };
    setWorkspaceUiState(nextState);
    void saveWorkspaceUiState(vaultPath, nextState)
      .then(setWorkspaceUiState)
      .catch((error) => showOperationNotice(rootT("notice.saveArchitectTabFailed", { error: getErrorMessage(error) }), "error"));
  };

  const persistArchitectSection = (section: string, expanded: boolean) => {
    if (!vaultPath || !workspaceUiState) {
      return;
    }
    if (workspaceUiState.architect_sections?.[section] === expanded) {
      return;
    }
    const nextState: WorkspaceUiState = {
      ...workspaceUiState,
      architect_sections: {
        ...(workspaceUiState.architect_sections ?? {}),
        [section]: expanded,
      },
    };
    setWorkspaceUiState(nextState);
    void saveWorkspaceUiState(vaultPath, nextState)
      .then(setWorkspaceUiState)
      .catch((error) => showOperationNotice(rootT("notice.saveArchitectSectionFailed", { error: getErrorMessage(error) }), "error"));
  };

  const navigateTo = (view: AppView, label = viewLabels[view], options?: Partial<FocusTarget>) => {
    const { label: nextLabel, options: nextOptions, view: nextView } = normalizeNavigationTarget(view, label, options);
    if (nextView !== "dashboard") {
      returnFocusRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    }
    const architectTab = nextView === "architect" ? nextOptions?.architectTab ?? architectTabFromLabel(nextLabel) : undefined;
    if (architectTab) {
      setArchitectActiveTab(architectTab);
      persistArchitectTab(architectTab);
    }
    setActiveView(nextView);
    setFocusTarget({
      ...nextOptions,
      label: nextLabel,
      moduleId: (nextView === "module" ? nextOptions?.moduleId : viewToModuleId(nextView)) ?? nextOptions?.moduleId ?? undefined,
      view: nextView,
      architectTab,
    });
  };

  const handleBackToDashboard = () => {
    setActiveView("dashboard");
    setFocusTarget({ label: "Dashboard", view: "dashboard" });
    window.setTimeout(() => returnFocusRef.current?.focus(), 0);
  };

  const handleRefreshCurrentVault = async () => {
    if (vaultPath) {
      await refreshWorkspace(vaultPath);
    }
  };

  const handleToggleModule = async (moduleId: string, enabled: boolean) => {
    if (!vaultPath) {
      return;
    }
    try {
      const updated = await setModuleEnabled(vaultPath, moduleId, enabled);
      setModuleRegistry(updated);
    } catch (error) {
      showOperationNotice(rootT("notice.updateModuleFailed", { error: getErrorMessage(error) }), "error");
    }
  };

  const handleEnableModuleFromWidgetPicker = (moduleId: string) => {
    const module = moduleRegistry?.modules.find((candidate) => candidate.id === moduleId);
    const label = module?.display_name ?? moduleId;
    if (window.confirm(rootT("notice.enableModuleForWidgets", { label }))) {
      void handleToggleModule(moduleId, true);
    }
  };

  const runWidgetAction = useCallback(async (operation: (path: string) => Promise<DashboardWidgetState>) => {
    if (!vaultPath) {
      return;
    }
    try {
      setDashboardWidgetState(await operation(vaultPath));
    } catch (error) {
      if (isDashboardLayoutBlocked(error)) {
        showOperationNotice(rootT("notice.dashboardLayoutBlocked"), "error");
        return;
      }
      showOperationNotice(rootT("notice.updateDashboardWidgetsFailed", { error: getErrorMessage(error) }), "error");
    }
  }, [rootT, showOperationNotice, vaultPath]);

  const widgetActions: WidgetActions = useMemo(() => ({
    addWidget: (widgetType) => void runWidgetAction((path) => createDashboardWidget(path, { widget_type: widgetType.id, module_id: widgetType.module_id })),
    compactWidgets: () => void runWidgetAction((path) => compactDashboardWidgets(path)),
    duplicateWidget: (instanceId) => void runWidgetAction((path) => duplicateDashboardWidget(path, instanceId)),
    moveWidget: (instance, direction) =>
      void runWidgetAction((path) =>
        moveDashboardWidget(path, instance.instance_id, {
          ...instance.layout,
          column: instance.layout.column,
          row: Math.max(1, instance.layout.row + (direction === "down" ? 1 : -1)),
        }),
      ),
    moveWidgetTo: (instance, layout) =>
      void runWidgetAction((path) =>
        moveDashboardWidget(path, instance.instance_id, {
          ...layout,
        }),
      ),
    removeWidget: (instanceId) => void runWidgetAction((path) => removeDashboardWidget(path, instanceId)),
    resizeWidget: (instanceId, size) => void runWidgetAction((path) => resizeDashboardWidget(path, instanceId, size)),
    toggleCollapsed: (instance) =>
      void runWidgetAction((path) => setDashboardWidgetCollapsed(path, instance.instance_id, !instance.collapsed)),
    updateWidget: (instanceId, input) => void runWidgetAction((path) => updateDashboardWidget(path, instanceId, input)),
  }), [runWidgetAction]);

  const handleResetDashboardWidgets = () => {
    void runWidgetAction((path) => resetDashboardWidgets(path));
  };

  const handleArchitectTabChange = (tab: ArchitectTabId) => {
    setArchitectActiveTab(tab);
    persistArchitectTab(tab);
  };

  const handleOpenPinnedEntity = async (entity: DashboardPinnedEntity) => {
    const view = entityView(entity.entity_type);
    let documentId = entity.document_id;
    if (view === "notes" && vaultPath) {
      const resolvedNote =
        noteSummaries.find((note) => note.document_id === entity.document_id) ??
        noteSummaries.find((note) => note.markdown_relative_path === entity.markdown_relative_path) ??
        noteSummaries.find((note) => note.title === entity.title);
      if (resolvedNote) {
        documentId = resolvedNote.document_id;
      }
    }
    navigateTo(view, entity.title, {
      documentId,
      moduleId: entity.entity_type === "todos" ? "todos" : `${entity.entity_type}s`,
    });
    if (view === "notes" && vaultPath) {
      await handleSelectNote(documentId);
    }
  };

  const handleSelectNote = async (documentId: string) => {
    if (!vaultPath) {
      return null;
    }
    setSelectedDocumentId(documentId);
    const note = await readNote(vaultPath, documentId);
    setSelectedNote(note);
    return note;
  };

  const handleSaveNote = async (
    documentId: string,
    markdownBody: string,
    expectedContentHash?: string | null,
    overwriteConflict = false,
  ) => {
    if (!vaultPath) {
      throw new Error("No vault is selected.");
    }
    const note = await updateNote(vaultPath, documentId, markdownBody, expectedContentHash, overwriteConflict);
    setSelectedNote(note);
    await refreshWorkspace(vaultPath, note.document_id);
    return note;
  };

  const handlePasteNoteImage = async (documentId: string, file: File) => {
    if (!vaultPath) {
      throw new Error("No vault is selected.");
    }
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    const asset = await saveMarkdownAsset(vaultPath, "notes", documentId, file.name || "pasted-image", file.type, bytes);
    return asset.markdown_link;
  };

  const handleRenameNote = async (documentId: string, newTitle: string) => {
    if (!vaultPath) {
      return;
    }
    const note = await renameNote(vaultPath, documentId, newTitle);
    await refreshWorkspace(vaultPath, note.document_id);
  };

  const handleToggleNotePin = async (documentId: string, pinned: boolean) => {
    if (!vaultPath) {
      return;
    }
    const hub = pinned
      ? await unpinDashboardEntity(vaultPath, documentId)
      : await pinDashboardEntity(vaultPath, documentId);
    setDashboardHub(hub);
  };

  const handleCreateTodo = async (title?: string, markdownBody?: string) => {
    if (!vaultPath) return null;
    try {
      const created = await createTodo(vaultPath, title ?? "Untitled task", markdownBody);
      await refreshWorkspace(vaultPath, created.document_id);
      return created;
    } catch (error) {
      showOperationNotice(rootT("notice.createTaskFailed", { error: getErrorMessage(error) }), "error");
      return null;
    }
  };

  const handleRenameTodo = async (documentId: string, newTitle: string) => {
    if (!vaultPath) return;
    try {
      await renameTodo(vaultPath, documentId, newTitle);
      await refreshWorkspace(vaultPath, documentId);
    } catch (error) {
      showOperationNotice(rootT("notice.renameTodosFailed", { error: getErrorMessage(error) }), "error");
    }
  };

  const handleSaveTodo = async (documentId: string, markdownBody: string) => {
    if (!vaultPath) return;
    try {
      await updateTodo(vaultPath, documentId, markdownBody);
      await refreshWorkspace(vaultPath, documentId);
    } catch (error) {
      showOperationNotice(rootT("notice.saveTodosFailed", { error: getErrorMessage(error) }), "error");
    }
  };

  const handleSelectTodo = async (documentId: string) => {
    if (!vaultPath) return;
    try {
      setSelectedTodo(await readTodo(vaultPath, documentId));
    } catch (error) {
      showOperationNotice(rootT("notice.loadTodosFailed", { error: getErrorMessage(error) }), "error");
    }
  };

  const handleSaveContact = async (contactId: string | null, input: ContactInput) => {
    if (!vaultPath) {
      return;
    }
    try {
      const nextDocument = contactId
        ? await updateContact(vaultPath, contactId, input)
        : await createContact(vaultPath, input);
      setContactDocument(nextDocument);
      const selected = contactId ?? nextDocument.contacts[nextDocument.contacts.length - 1]?.contact_id ?? null;
      setSelectedContactId(selected);
      await refreshWorkspace(vaultPath);
    } catch (error) {
      showOperationNotice(rootT("notice.saveContactFailed", { error: getErrorMessage(error) }), "error");
    }
  };

  const handleSaveHabit = async (habitId: string | null, input: HabitInput) => {
    if (!vaultPath) {
      return;
    }
    try {
      const today = todayKey();
      const nextDocument = habitId
        ? await updateHabit(vaultPath, habitId, input, today)
        : await createHabit(vaultPath, input, today);
      setHabitDocument(nextDocument);
      const selected = habitId ?? nextDocument.habits[nextDocument.habits.length - 1]?.habit_id ?? null;
      setSelectedHabitId(selected);
      await refreshWorkspace(vaultPath);
    } catch (error) {
      showOperationNotice(rootT("notice.saveHabitFailed", { error: getErrorMessage(error) }), "error");
    }
  };

  const handleHabitCheckin = async (habitId: string) => {
    if (!vaultPath) {
      return;
    }
    try {
      setHabitDocument(await recordHabitCheckin(vaultPath, habitId, todayKey()));
      await refreshWorkspace(vaultPath);
    } catch (error) {
      showOperationNotice(rootT("notice.recordHabitCheckinFailed", { error: getErrorMessage(error) }), "error");
    }
  };

  const widgetInteractions: WidgetInteractionHandlers = useMemo(() => ({
    copyText: async (value, label) => {
      await navigator.clipboard.writeText(value);
      showOperationNotice(rootT("notice.valueCopied", { label: label ?? rootT("notice.value") }), "success");
    },
    openEntity: (target) => {
      const view = viewForModule(target.moduleId);
      navigateTo(view, target.label, { documentId: target.documentId, moduleId: target.moduleId });
      if (target.moduleId === "notes" && target.documentId) {
        void handleSelectNote(target.documentId);
      }
      if (target.moduleId === "todos" && target.documentId) {
        void handleSelectTodo(target.documentId);
      }
      if (target.moduleId === "contacts" && target.entityId) {
        setSelectedContactId(target.entityId);
      }
      if (target.moduleId === "habits" && target.entityId) {
        setSelectedHabitId(target.entityId);
      }
    },
    recordHabitCheckin: async (habitId) => {
      await handleHabitCheckin(habitId);
    },
    refreshWidgetData: async () => {
      await handleRefreshCurrentVault();
    },
    toggleTodoComplete: async (documentId, completed) => {
      if (!vaultPath) return;
      const todo = await readTodo(vaultPath, documentId);
      await updateTodo(vaultPath, documentId, setTodoCompletion(todo.markdown_body, completed));
      await refreshWorkspace(vaultPath, documentId);
    },
  }), [rootT, showOperationNotice, vaultPath]);

  const handleRebuildNavigator = async () => {
    if (!vaultPath) {
      return;
    }
    const startedAt = performance.now();
    const report = await scanAndRebuildNavigator(vaultPath);
    await refreshWorkspace(vaultPath, selectedDocumentId ?? selectedNote?.document_id ?? undefined);
    showOperationNotice(
      rootT("notice.workspaceRefreshed", { documents: report.scan.documents.length, backlinks: report.navigator.backlinks.length }),
      "success",
    );
    logV5Timing("navigator rebuild", startedAt, {
      backlinks: report.navigator.backlinks.length,
      documents: report.scan.documents.length,
      warnings: report.navigator.health_warnings.length,
    });
  };

  const handleSearchGraph = async (query: string) => {
    if (!vaultPath) {
      return;
    }
    const startedAt = performance.now();
    const snapshot = await searchEntities(vaultPath, query);
    setSearchSnapshot(snapshot);
    logV5Timing("graph search", startedAt, {
      queryLength: query.length,
      results: snapshot.entries.length,
    });
  };

  const handlePreviewEntityUpgrade = async () => {
    if (!vaultPath) {
      return;
    }
    setUpgradePreview(await previewEntityUpgrade(vaultPath));
  };

  const handleApplyEntityUpgrade = async () => {
    if (!vaultPath) {
      return;
    }
    const report = await applyEntityUpgrade(vaultPath);
    showOperationNotice(rootT("notice.entityUpgradeCompleted", { count: report.changes.length }), "success");
    setUpgradePreview(null);
    await refreshWorkspace(vaultPath);
  };

  const handlePreviewTheme = async (scope: ThemeScope, moduleId: string | null, sourcePath: string) => {
    if (!vaultPath) {
      return;
    }
    try {
      const preview = await previewThemeTokens(vaultPath, scope, moduleId, sourcePath);
      setThemePreview(preview);
      showOperationNotice(rootT(scope === "module" ? "notice.moduleThemePreviewActive" : "notice.workspaceThemePreviewActive"), "info");
    } catch (error) {
      setThemePreview(null);
      showOperationNotice(getErrorMessage(error), "error");
    }
  };

  const handleApplyTheme = async (scope: ThemeScope, moduleId: string | null, sourcePath: string) => {
    if (!vaultPath) {
      return;
    }
    try {
      const state = await applyThemeTokens(vaultPath, scope, moduleId, sourcePath);
      setActiveTheme(state);
      setThemePreview(null);
      showOperationNotice(rootT(scope === "module" ? "notice.moduleThemeApplied" : "notice.workspaceThemeApplied"), "success");
    } catch (error) {
      showOperationNotice(getErrorMessage(error), "error");
    }
  };

  const handleCancelThemePreview = () => {
    setThemePreview(null);
    showOperationNotice(rootT("notice.themePreviewCanceled"), "info");
  };

  const handleRollbackTheme = async (scope: ThemeScope, moduleId: string | null) => {
    if (!vaultPath) {
      return;
    }
    try {
      const state = await rollbackTheme(vaultPath, scope, moduleId);
      setActiveTheme(state);
      setThemePreview(null);
      showOperationNotice(rootT("notice.themeRolledBack"), "success");
    } catch (error) {
      showOperationNotice(getErrorMessage(error), "error");
    }
  };

  const handleRunRecovery = async (issue: RecoveryIssue) => {
    if (!vaultPath || !issue.action) {
      return;
    }

    let resultMessage = rootT("notice.recoveryActionCompleted");
    if (issue.action === "recover_document_metadata" && issue.markdown_relative_path) {
      resultMessage = (await recoverDocumentMetadata(vaultPath, issue.markdown_relative_path)).message;
    } else if (issue.action === "recover_layout_metadata" && issue.document_id) {
      resultMessage = (await recoverLayoutMetadata(vaultPath, issue.document_id)).message;
    } else if (issue.action === "orphan_missing_document_metadata" && issue.document_id) {
      resultMessage = (await orphanMissingDocumentMetadata(vaultPath, issue.document_id)).message;
    } else if (
      issue.action === "restore_orphaned_document_metadata" &&
      issue.document_id &&
      issue.markdown_relative_path
    ) {
      resultMessage = (
        await restoreOrphanedDocumentMetadata(vaultPath, issue.document_id, issue.markdown_relative_path)
      ).message;
    } else if (issue.action === "repair_document_frontmatter_reference" && issue.document_id) {
      resultMessage = (await repairDocumentFrontmatterReference(vaultPath, issue.document_id)).message;
    }

    showOperationNotice(resultMessage, "success");
    await refreshWorkspace(vaultPath);
  };

  if (vaultSnapshot.stage !== "ready") {
    return (
      <TooltipProvider delayDuration={150}>
        <main className="min-h-screen px-4 py-5 sm:px-6 lg:px-8">
          <div className="mx-auto flex w-full max-w-5xl flex-col gap-5">
            <VaultOnboarding
              action={vaultAction}
              onCreateDefault={handleCreateDefaultVault}
              onRepair={handleRepairVault}
              onReset={handleResetVault}
              onSelectVault={handleSelectVault}
              snapshot={vaultSnapshot}
            />
          </div>
        </main>
      </TooltipProvider>
    );
  }

  return (
    <I18nProvider language={language} setLanguage={handleLanguageChange}>
      <TooltipProvider delayDuration={150}>
        <main className="min-h-screen bg-background px-3 py-3 text-foreground sm:px-4 lg:px-5" style={shellStyle}>
        <div
          className={`grid w-full max-w-none gap-4 ${
            railExpanded ? "lg:grid-cols-[14rem_minmax(0,1fr)]" : "lg:grid-cols-[3.5rem_minmax(0,1fr)]"
          }`}
        >
          <AppRail
            activeView={activeView}
            activeModuleId={activeModuleId}
            expanded={railExpanded}
            onNavigate={navigateTo}
            onToggleExpanded={() => setRailExpanded((current) => !current)}
            shortcuts={visibleShortcuts}
            systemShortcuts={systemShortcuts}
            vaultPath={vaultPath}
          />
          <div className="flex min-w-0 flex-col gap-4">
          <ShellTopBar
            activeView={activeView}
            dashboardHub={dashboardHub}
            focusTarget={focusTarget}
            issueCount={workspaceScan?.issues.length ?? 0}
            lastScanAt={lastScanAt}
            onBack={activeView === "dashboard" ? undefined : handleBackToDashboard}
            onNavigate={navigateTo}
            onOpenCommandPalette={() => setCommandPaletteOpen(true)}
            onRefresh={handleRefreshCurrentVault}
            onStatusClick={() => navigateTo("architect", "Recovery", { architectTab: "recovery" })}
            visibleShortcuts={visibleShortcuts}
          />
          <CommandPalette
            activeTheme={activeTheme}
            contacts={contactDocument}
            habits={habitDocument}
            todoSummaries={todoSummaries}
            onClose={() => setCommandPaletteOpen(false)}
            onCreateNote={() => handleCreateNote("Daily Note", "# Daily Note\n\n## Today\n\n- [ ] First task\n")}
            onCreateTodo={() => navigateTo("todos")}
            onNavigate={navigateTo}
            onRepairVault={handleRepairVault}
            onThemePreview={() => navigateTo("settings", "Theme preview", { moduleId: activeModuleId ?? undefined })}
            open={commandPaletteOpen}
          />
          {activeView === "dashboard" ? (
            <DashboardHub
              dashboardHub={dashboardHub}
              loading={workspaceLoading}
              moduleRegistry={moduleRegistry}
              onEnableModuleFromWidgetPicker={handleEnableModuleFromWidgetPicker}
              onNavigate={navigateTo}
              onOpenArchitect={() => navigateTo("architect", "Architect")}
              onOpenPinnedEntity={handleOpenPinnedEntity}
              onRefresh={handleRefreshCurrentVault}
              widgetActions={widgetActions}
              widgetContext={widgetContext}
              widgetInteractions={widgetInteractions}
              widgetState={dashboardWidgetState}
              widgetTypes={widgetTypes}
              workspaceError={workspaceResourceErrors.dashboardHub ?? workspaceResourceErrors.dashboardWidgets ?? workspaceError}
            />
          ) : (
            <FocusedView
              activeView={activeView}
              activeTheme={activeTheme}
              contacts={contactDocument}
              dashboardHub={dashboardHub}
              focusTarget={focusTarget}
              habits={habitDocument}
              moduleRegistry={moduleRegistry}
              navigator={navigatorSnapshot}
              noteSummaries={noteSummaries}
              todoSummaries={todoSummaries}
              onApplyTheme={handleApplyTheme}
              onCancelThemePreview={handleCancelThemePreview}
              onCreateNote={handleCreateNote}
              onCreateTodo={handleCreateTodo}
              onHabitCheckin={handleHabitCheckin}
              onNavigate={navigateTo}
              onRenameNote={handleRenameNote}
              onRenameTodo={handleRenameTodo}
              onToggleNotePin={handleToggleNotePin}
              onApplyEntityUpgrade={handleApplyEntityUpgrade}
              onPreviewEntityUpgrade={handlePreviewEntityUpgrade}
              onPreviewTheme={handlePreviewTheme}
              onRebuildNavigator={handleRebuildNavigator}
              onRefreshWorkspace={handleRefreshCurrentVault}
              onResetVault={handleResetVault}
              onRollbackTheme={handleRollbackTheme}
              onRunRecovery={handleRunRecovery}
              onSaveContact={handleSaveContact}
              onSaveHabit={handleSaveHabit}
              onSaveNote={handleSaveNote}
              onSaveTodo={handleSaveTodo}
              onPasteNoteImage={handlePasteNoteImage}
              onSelectContact={setSelectedContactId}
              onSelectHabit={setSelectedHabitId}
              onSelectNote={handleSelectNote}
              onSelectTodo={handleSelectTodo}
              onSearchGraph={handleSearchGraph}
              onToggleModule={handleToggleModule}
              recoveryPreview={recoveryPreview}
              searchSnapshot={searchSnapshot}
              selectedContactId={selectedContactId}
              selectedHabitId={selectedHabitId}
              selectedNote={selectedNote}
              todos={selectedTodo}
              themePreview={themePreview}
              upgradePreview={upgradePreview}
              language={language}
              onLanguageChange={handleLanguageChange}
              vaultInspection={vaultSnapshot.inspection}
              vaultPath={vaultPath}
              widgetActions={widgetActions}
              widgetContext={widgetContext}
              widgetInteractions={widgetInteractions}
              widgetState={dashboardWidgetState}
              widgetTypes={widgetTypes}
              onResetWidgets={handleResetDashboardWidgets}
              workspaceUiState={workspaceUiState}
              onArchitectSectionChange={persistArchitectSection}
              onArchitectTabChange={handleArchitectTabChange}
              architectActiveTab={architectActiveTab}
              vaultResetting={vaultAction === "reset"}
              visibleShortcuts={visibleShortcuts}
              workspaceScan={workspaceScan}
              workspaceError={workspaceError}
              workspaceResourceErrors={workspaceResourceErrors}
              workspaceLoading={workspaceLoading}
            />
          )}
          </div>
        </div>
        </main>
      </TooltipProvider>
    </I18nProvider>
  );
}

function AppRail({
  activeView,
  activeModuleId,
  expanded,
  onNavigate,
  onToggleExpanded,
  shortcuts,
  systemShortcuts,
  vaultPath,
}: {
  activeView: AppView;
  activeModuleId: string | null;
  expanded: boolean;
  onNavigate: (view: AppView, label?: string, options?: Partial<FocusTarget>) => void;
  onToggleExpanded: () => void;
  shortcuts: ModuleNavEntry[];
  systemShortcuts: ModuleNavEntry[];
  vaultPath?: string;
}) {
  const { t } = useI18n();
  return (
    <aside
      className={`sticky top-3 hidden h-[calc(100vh-1.5rem)] min-h-0 flex-col rounded-md border border-border bg-card p-2 shadow-soft lg:flex ${
        expanded ? "w-56" : "w-14"
      }`}
      aria-label={t("shell.primaryNavigation")}
    >
      <div className="flex min-w-0 items-center gap-2 border-b border-border pb-3">
        <BentoLifeBrandMark />
        {expanded ? (
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold">BentoLife</p>
          <p className="truncate text-xs text-muted-foreground">{t("shell.localVault")}</p>
        </div>
        ) : null}
      </div>

      <nav className="mt-4 flex min-h-0 flex-1 flex-col gap-1 overflow-auto" aria-label={t("shell.moduleNavigation")}>
        <RailButton active={activeView === "dashboard"} expanded={expanded} label={t("nav.dashboard")} onClick={() => onNavigate("dashboard", t("nav.dashboard"))} view="dashboard" />
        {shortcuts
          .map((shortcut) => {
            const label = shortcut.view === "module" ? shortcut.label : t(navLabelKeys[shortcut.view]);
            return (
              <RailButton
                active={activeView === shortcut.view && (shortcut.view !== "module" || shortcut.moduleId === activeModuleId)}
                expanded={expanded}
                key={shortcut.id}
                label={label}
                onClick={() => onNavigate(shortcut.view, label, { moduleId: shortcut.moduleId })}
                view={shortcut.view}
              />
            );
          })}
        <div className="mt-auto border-t border-border pt-3">
          {systemShortcuts.map((item) => {
            const label = t(navLabelKeys[item.view]);
            return (
              <RailButton
                active={activeView === item.view}
                expanded={expanded}
                key={item.id}
                label={label}
                onClick={() => onNavigate(item.view, label, { moduleId: item.moduleId })}
                view={item.view}
              />
            );
          })}
        </div>
      </nav>

      <div className="mt-3 border-t border-border pt-3">
        <MaybeTooltip content={expanded ? t("shell.collapseNavigation") : t("shell.expandNavigation")} enabled={!expanded}>
          <button
            aria-label={expanded ? t("shell.collapseNavigationRail") : t("shell.expandNavigationRail")}
            className="flex h-10 w-full items-center justify-center gap-2 rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onClick={onToggleExpanded}
            type="button"
          >
            <Menu className="size-4" aria-hidden="true" />
            {expanded ? <span className="truncate text-sm font-medium">{t("shell.collapse")}</span> : null}
          </button>
        </MaybeTooltip>
        {expanded ? <p className="mt-3 line-clamp-2 break-all px-1 text-[11px] text-muted-foreground">{vaultPath ?? t("shell.noVaultPath")}</p> : null}
      </div>
    </aside>
  );
}

function RailButton({
  active,
  expanded,
  label,
  onClick,
  view,
}: {
  active: boolean;
  expanded: boolean;
  label: string;
  onClick: () => void;
  view: AppView;
}) {
  const Icon = iconByView[view];
  return (
    <MaybeTooltip content={label} enabled={!expanded}>
      <button
        aria-label={label}
        className={`flex h-10 w-full items-center justify-center gap-2 rounded-md text-left text-sm transition-colors ${
          expanded ? "px-2.5 justify-start" : "px-0"
        } ${
          active
            ? "bg-primary text-primary-foreground shadow-sm"
            : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
        }`}
        onClick={onClick}
        type="button"
      >
        <Icon className="size-4 shrink-0" aria-hidden="true" />
        {expanded ? <span className="truncate">{label}</span> : null}
      </button>
    </MaybeTooltip>
  );
}

function MaybeTooltip({ children, content, enabled }: { children: React.ReactElement; content: string; enabled: boolean }) {
  if (!enabled) return children;
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side="right">{content}</TooltipContent>
    </Tooltip>
  );
}

type ShellTopBarProps = {
  activeView: AppView;
  dashboardHub: DashboardHubDocument | null;
  focusTarget: FocusTarget;
  issueCount: number;
  lastScanAt: Date | null;
  onBack?: () => void;
  onNavigate: (view: AppView, label?: string, options?: Partial<FocusTarget>) => void;
  onOpenCommandPalette: () => void;
  onRefresh: () => void;
  onStatusClick: () => void;
  visibleShortcuts: ModuleNavEntry[];
};

function ShellTopBar({
  activeView,
  dashboardHub,
  focusTarget,
  issueCount,
  lastScanAt,
  onBack,
  onNavigate,
  onOpenCommandPalette,
  onRefresh,
  onStatusClick,
  visibleShortcuts,
}: ShellTopBarProps) {
  const { t } = useI18n();
  const currentLabel = activeView === "dashboard" ? t("shell.home") : focusTarget.label || t(navLabelKeys[activeView]);
  const statusLabel = issueCount ? `${issueCount} ${t("shell.issues")}` : t("shell.synced");
  const statusTone = issueCount ? "text-amber-note-foreground hover:bg-amber-note/20" : "text-primary";
  const StatusIcon = issueCount ? AlertTriangle : CheckCircle2;
  const moduleMenuEntries = useMemo(() => {
    const entries = new Map<string, ModuleNavEntry>();

    visibleShortcuts.forEach((shortcut) => entries.set(shortcut.moduleId ?? shortcut.id, shortcut));
    dashboardHub?.module_summaries
      .filter((summary) => summary.status === "implemented" && summary.module_id !== "navigator")
      .forEach((summary) => {
        if (entries.has(summary.module_id)) return;
        entries.set(summary.module_id, {
          defaultView: "cards",
          documentType: moduleDocumentType(summary.module_id),
          id: summary.module_id,
          kind: "optional",
          label: summary.display_name,
          moduleId: summary.module_id,
          system: false,
          view: moduleView(summary.module_id),
        });
      });

    return Array.from(entries.values());
  }, [dashboardHub, visibleShortcuts]);

  return (
    <header className="flex min-h-16 flex-wrap items-center justify-between gap-3 rounded-md border border-border bg-card px-3 py-2 shadow-soft">
      <div className="flex min-w-0 items-center gap-2">
        {onBack ? (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button aria-label={t("shell.backToDashboard")} onClick={onBack} size="icon" variant="ghost">
                <ArrowLeft aria-hidden="true" />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t("shell.backToDashboard")}</TooltipContent>
          </Tooltip>
        ) : null}
        <div className="flex min-w-0 items-center gap-2 text-sm">
          <span className="flex items-center gap-2 font-semibold text-primary">
            <Sparkles className="size-4" aria-hidden="true" />
            {t(navLabelKeys[activeView])}
          </span>
          <ChevronRight className="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <span className="truncate text-foreground">{currentLabel}</span>
        </div>
      </div>

      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button aria-label={t("commandPalette.tooltip")} onClick={onOpenCommandPalette} size="icon" variant="outline">
              <Search aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("commandPalette.tooltip")}</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button aria-label={t("shell.rescanVault")} onClick={onRefresh} size="icon" variant="outline">
              <RefreshCw aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent>{t("shell.rescanVault")}</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              aria-label={issueCount ? t("shell.openIssues") : t("shell.vaultSynced")}
              className={`hidden items-center gap-2 rounded-md border border-border bg-background px-3 py-2 text-xs transition-colors sm:flex ${statusTone}`}
              disabled={!issueCount}
              onClick={issueCount ? onStatusClick : undefined}
              type="button"
            >
          <StatusIcon className="size-4" aria-hidden="true" />
          <span className="font-medium">{statusLabel}</span>
          {lastScanAt ? <span className="text-muted-foreground">{formatTime(lastScanAt)}</span> : null}
            </button>
          </TooltipTrigger>
          <TooltipContent>{issueCount ? t("shell.openIssues") : t("shell.vaultSynced")}</TooltipContent>
        </Tooltip>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button aria-label={t("shell.openModulesMenu")} className="lg:hidden" size="icon" variant="outline">
              <Menu aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuGroup>
              {moduleMenuEntries.map((shortcut) => {
                const Icon = iconByView[shortcut.view];

                return (
                  <DropdownMenuItem key={shortcut.id} onSelect={() => onNavigate(shortcut.view, shortcut.label, { moduleId: shortcut.moduleId })}>
                    <Icon aria-hidden="true" data-icon="inline-start" />
                    <span>{shortcut.label}</span>
                  </DropdownMenuItem>
                );
              })}
            </DropdownMenuGroup>
          </DropdownMenuContent>
        </DropdownMenu>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button aria-label={t("shell.moreActions")} size="icon" variant="ghost">
              <SlidersHorizontal aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={() => onNavigate("settings", t("nav.settings"))}>
              <Settings aria-hidden="true" data-icon="inline-start" />
              {t("nav.settings")}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onNavigate("architect", t("nav.architect"))}>
              <SlidersHorizontal aria-hidden="true" data-icon="inline-start" />
              {t("nav.architect")}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onNavigate("trash", t("nav.trash"))}>
              <Trash2 aria-hidden="true" data-icon="inline-start" />
              {t("nav.trash")}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onNavigate("archive", t("nav.archive"))}>
              <Archive aria-hidden="true" data-icon="inline-start" />
              {t("nav.archive")}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>
  );
}

type CommandPaletteProps = {
  activeTheme: ActiveThemeState | null;
  contacts: ContactDocument | null;
  habits: HabitDocument | null;
  onClose: () => void;
  onCreateNote: () => void;
  onCreateTodo: () => void;
  onNavigate: (view: AppView, label?: string, options?: Partial<FocusTarget>) => void;
  onRepairVault: () => void;
  onThemePreview: () => void;
  open: boolean;
  todoSummaries: TodoSummary[];
};

function CommandPalette({
  activeTheme,
  contacts,
  habits,
  onClose,
  onCreateNote,
  onCreateTodo,
  onNavigate,
  onRepairVault,
  onThemePreview,
  open,
  todoSummaries,
}: CommandPaletteProps) {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const commands = useMemo(
    () => [
      { label: t("commandPalette.searchGraph"), icon: Search, action: () => onNavigate("architect", t("nav.dataGraph"), { architectTab: "data_graph" }) },
      { label: t("commandPalette.createNote"), icon: Plus, action: onCreateNote },
      { label: t("commandPalette.addTask"), icon: CheckSquare, action: onCreateTodo },
      { label: t("commandPalette.createContact"), icon: Users, action: () => onNavigate("contacts", t("nav.contacts")) },
      { label: t("commandPalette.createHabit"), icon: Leaf, action: () => onNavigate("habits", t("nav.habits")) },
      { label: t("commandPalette.openSettings"), icon: Settings, action: () => onNavigate("settings", t("nav.settings")) },
      { label: t("commandPalette.openVaultData"), icon: FolderOpen, action: () => onNavigate("settings", t("settings.vaultData.title")) },
      { label: t("commandPalette.openArchitect"), icon: SlidersHorizontal, action: () => onNavigate("architect", t("nav.architect")) },
      { label: t("commandPalette.repairVault"), icon: RefreshCw, action: onRepairVault },
      { label: t("commandPalette.themePreview"), icon: Palette, action: onThemePreview },
    ],
    [onCreateNote, onCreateTodo, onNavigate, onRepairVault, onThemePreview, t],
  );
  const filtered = commands.filter((command) => command.label.toLowerCase().includes(query.toLowerCase()));

  if (!open) {
    return null;
  }

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => !nextOpen && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t("commandPalette.title")}</DialogTitle>
          <DialogDescription>{t("commandPalette.description")}</DialogDescription>
        </DialogHeader>
        <div className="flex flex-col gap-3">
          <Input
            aria-label={t("commandPalette.search")}
            autoFocus
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("commandPalette.searchActions")}
            value={query}
          />
          <div className="flex flex-col gap-2">
            {filtered.map((command) => {
              const Icon = command.icon;
              return (
                <Button
                  className="justify-start"
                  key={command.label}
                  onClick={() => {
                    command.action();
                    onClose();
                  }}
                  variant="outline"
                >
                  <Icon data-icon="inline-start" />
                  {command.label}
                </Button>
              );
            })}
          </div>
          <div className="grid gap-2 text-xs text-muted-foreground sm:grid-cols-2">
            <SummaryRow label={t("commandPalette.openTasks")} value={`${todoSummaries.filter(todo => !todo.is_completed).length}`} />
            <SummaryRow label={t("nav.contacts")} value={`${contacts?.summary.total ?? 0}`} />
            <SummaryRow label={t("nav.habits")} value={`${habits?.summary.total ?? 0}`} />
            <SummaryRow label={t("nav.settings")} value={activeTheme?.workspace_theme.theme_id ?? "clean-slate"} />
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

type FocusedViewProps = {
  activeView: AppView;
  activeTheme: ActiveThemeState | null;
  contacts: ContactDocument | null;
  dashboardHub: DashboardHubDocument | null;
  focusTarget: FocusTarget;
  habits: HabitDocument | null;
  moduleRegistry: RegistryState | null;
  navigator: NavigatorSnapshot | null;
  noteSummaries: NoteSummary[];
  todoSummaries: TodoSummary[];
  onApplyEntityUpgrade: () => void;
  onApplyTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  onCancelThemePreview: () => void;
  onCreateNote: (title?: string, markdownBody?: string) => Promise<NoteDocument | null>;
  onCreateTodo: (title?: string, markdownBody?: string) => Promise<TodoDocument | null>;
  onHabitCheckin: (habitId: string) => void;
  onArchitectSectionChange: (section: string, expanded: boolean) => void;
  onArchitectTabChange: (tab: ArchitectTabId) => void;
  architectActiveTab: ArchitectTabId;
  onNavigate: (view: AppView, label?: string, options?: Partial<FocusTarget>) => void;
  onPreviewEntityUpgrade: () => void;
  onPreviewTheme: (scope: ThemeScope, moduleId: string | null, sourcePath: string) => void;
  onRebuildNavigator: () => void;
  onRefreshWorkspace: () => Promise<void> | void;
  onRenameNote: (documentId: string, newTitle: string) => Promise<void>;
  onRenameTodo: (documentId: string, newTitle: string) => void;
  onToggleNotePin: (documentId: string, pinned: boolean) => Promise<void>;
  onResetVault: () => void;
  onRollbackTheme: (scope: ThemeScope, moduleId: string | null) => void;
  onRunRecovery: (issue: RecoveryIssue) => void;
  onSaveContact: (contactId: string | null, input: ContactInput) => void;
  onSaveHabit: (habitId: string | null, input: HabitInput) => void;
  onSaveNote: (
    documentId: string,
    markdownBody: string,
    expectedContentHash?: string | null,
    overwriteConflict?: boolean,
  ) => Promise<NoteDocument>;
  onSaveTodo: (documentId: string, markdownBody: string) => void;
  onPasteNoteImage: (documentId: string, file: File) => Promise<string>;
  onSelectContact: (contactId: string | null) => void;
  onSelectHabit: (habitId: string | null) => void;
  onSelectNote: (documentId: string) => Promise<NoteDocument | null>;
  onSelectTodo: (documentId: string) => void;
  onSearchGraph: (query: string) => void;
  onToggleModule: (moduleId: string, enabled: boolean) => void;
  onResetWidgets: () => void;
  recoveryPreview: WorkspaceRecoveryPreview | null;
  searchSnapshot: SearchIndexSnapshot | null;
  selectedContactId: string | null;
  selectedHabitId: string | null;
  selectedNote: NoteDocument | null;
  todos: TodoDocument | null;
  themePreview: ThemePreview | null;
  upgradePreview: EntityUpgradePreview | null;
  language: AppLanguage;
  onLanguageChange: (language: AppLanguage) => void;
  vaultInspection?: VaultInspection;
  vaultPath?: string;
  widgetActions: WidgetActions;
  widgetContext: {
    contacts: ContactDocument | null;
    habits: HabitDocument | null;
    notes: NoteSummary[];
    todos: TodoDocument | null;
    todoSummaries: TodoSummary[];
  };
  widgetInteractions: WidgetInteractionHandlers;
  widgetState: DashboardWidgetState | null;
  widgetTypes: WidgetTypeDefinition[];
  workspaceUiState: WorkspaceUiState | null;
  vaultResetting: boolean;
  visibleShortcuts: ModuleNavEntry[];
  workspaceScan: WorkspaceScanResult | null;
  workspaceError: string | null;
  workspaceResourceErrors: WorkspaceResourceErrors;
  workspaceLoading: boolean;
};

function FocusedView({
  activeView,
  activeTheme,
  contacts,
  dashboardHub,
  focusTarget,
  habits,
  moduleRegistry,
  navigator,
  noteSummaries,
  todoSummaries,
  onApplyEntityUpgrade,
  onApplyTheme,
  onCancelThemePreview,
  onCreateNote,
  onCreateTodo,
  onHabitCheckin,
  onArchitectSectionChange,
  onArchitectTabChange,
  architectActiveTab,
  onNavigate,
  onPreviewEntityUpgrade,
  onPreviewTheme,
  onRenameNote,
  onRenameTodo,
  onToggleNotePin,
  onRebuildNavigator,
  onRefreshWorkspace,
  onResetVault,
  onRollbackTheme,
  onRunRecovery,
  onSaveContact,
  onSaveHabit,
  onSaveNote,
  onSaveTodo,
  onPasteNoteImage,
  onSelectContact,
  onSelectHabit,
  onSelectNote,
  onSelectTodo,
  onSearchGraph,
  onToggleModule,
  onResetWidgets,
  recoveryPreview,
  searchSnapshot,
  selectedContactId,
  selectedHabitId,
  selectedNote,
  todos,
  themePreview,
  upgradePreview,
  language,
  onLanguageChange,
  vaultInspection,
  vaultPath,
  widgetActions,
  widgetContext,
  widgetInteractions,
  widgetState,
  widgetTypes,
  workspaceUiState,
  vaultResetting,
  workspaceScan,
  workspaceError,
  workspaceResourceErrors,
  workspaceLoading,
}: FocusedViewProps) {
  const { t } = useI18n();
  const activeModuleId = selectActiveModuleId(activeView, focusTarget);
  const activeModule = moduleRegistry?.modules.find((module) => module.id === activeModuleId) ?? null;
  const activeTitle = activeView === "module" ? focusTarget.label : t(navLabelKeys[activeView]);
  const selectedContact = selectContactById(contacts, selectedContactId);
  const selectedHabit = selectHabitById(habits, selectedHabitId);
  const moduleErrors = selectModuleErrors(workspaceResourceErrors, workspaceError);
  const noteBacklinks = backlinksForTarget(navigator, selectedNote?.document_id, selectedNote?.markdown_relative_path);
  const pinnedNoteIds = useMemo(
    () =>
      dashboardHub?.pinned_entities
        .filter((pin) => pin.entity_type === "note")
        .map((pin) => pin.document_id) ?? [],
    [dashboardHub?.pinned_entities],
  );
  const isSelectedNotePinned = Boolean(
    selectedNote && pinnedNoteIds.includes(selectedNote.document_id),
  );
  const todoBacklinks = backlinksForTarget(navigator, todos?.document_id, todos?.markdown_relative_path);
  const contactBacklinks = backlinksForTarget(navigator, contacts?.document_id, selectedContact?.parsed_entity.path ?? contacts?.markdown_relative_path);
  const habitBacklinks = backlinksForTarget(navigator, habits?.document_id, selectedHabit?.parsed_entity.path ?? habits?.markdown_relative_path);
  const recoveryIssueCount = workspaceScan?.issues.filter(isRecoveryScanIssue).length ?? 0;
  const showSurfaceBadges = activeView === "settings";

  return (
    <section className="flex flex-col gap-5">
      <div className="grid gap-5">
        <section className="min-w-0 rounded-md border border-border bg-card p-5 shadow-soft md:p-6">
          <div className="pb-5">
            {showSurfaceBadges ? (
              <div className="flex flex-wrap items-center gap-3">
                <Badge variant="secondary">{t("workspace.badges.dataOnlyImports")}</Badge>
                <Badge variant="outline">{t("workspace.badges.localFirst")}</Badge>
              </div>
            ) : null}
            <h1 className={`${showSurfaceBadges ? "mt-3" : ""} text-3xl font-semibold leading-tight md:text-4xl`}>{activeTitle}</h1>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">{descriptionForView(activeView, t, activeModule?.display_name)}</p>
          </div>
          <ModuleErrorBoundary
            moduleLabel={activeTitle}
            onRetry={onRefreshWorkspace}
            recoveryAction={
              activeView !== "architect" ? (
                <Button onClick={() => onNavigate("architect", t("nav.recovery"), { architectTab: "recovery" })} size="sm" variant="ghost">
                  {t("architect.graph.openRecovery")}
                </Button>
              ) : null
            }
          >
            {activeView === "notes" ? (
              <NotesPanel
                backlinks={noteBacklinks}
                loading={workspaceLoading}
                notes={noteSummaries}
                onCreateNote={onCreateNote}
                onPasteNoteImage={onPasteNoteImage}
                onRenameNote={onRenameNote}
                onRefreshWorkspace={onRefreshWorkspace}
                onSaveNote={onSaveNote}
                onSelectNote={onSelectNote}
                onToggleNotePin={onToggleNotePin}
                pinnedNoteIds={pinnedNoteIds}
                selectedNotePinned={isSelectedNotePinned}
                selectedNote={selectedNote}
                vaultPath={vaultPath ?? null}
                workspaceError={moduleErrors.notes}
              />
            ) : null}
            {activeView === "todos" ? (
              <TodoPanel
                backlinks={todoBacklinks}
                loading={workspaceLoading}
                onCreateTodo={onCreateTodo}
                onSaveAsNote={onCreateNote}
                onRenameTodo={onRenameTodo}
                onSaveTodo={onSaveTodo}
                onSelectTodo={onSelectTodo}
                selectedTodo={todos}
                todos={todoSummaries}
                workspaceError={moduleErrors.todos}
              />
            ) : null}
            {activeView === "contacts" ? (
              <ContactsPanel
                backlinks={contactBacklinks}
                contacts={contacts}
                loading={workspaceLoading}
                onSaveAsNote={onCreateNote}
                onSaveContact={onSaveContact}
                onSelectContact={onSelectContact}
                selectedContactId={selectedContactId}
                workspaceError={moduleErrors.contacts}
              />
            ) : null}
            {activeView === "habits" ? (
              <HabitsPanel
                backlinks={habitBacklinks}
                habits={habits}
                loading={workspaceLoading}
                onCheckin={onHabitCheckin}
                onSaveAsNote={onCreateNote}
                onSaveHabit={onSaveHabit}
                onSelectHabit={onSelectHabit}
                selectedHabitId={selectedHabitId}
                workspaceError={moduleErrors.habits}
              />
            ) : null}
            {activeView === "architect" ? (
              <ArchitectControlPanel
                activeTheme={activeTheme}
                dashboardHub={dashboardHub}
                onNavigate={onNavigate}
                scan={workspaceScan}
                moduleRegistry={moduleRegistry}
                navigator={navigator}
                onApplyEntityUpgrade={onApplyEntityUpgrade}
                onApplyTheme={onApplyTheme}
                onCancelThemePreview={onCancelThemePreview}
                onPreviewEntityUpgrade={onPreviewEntityUpgrade}
                onPreviewTheme={onPreviewTheme}
                onRebuildNavigator={onRebuildNavigator}
                onToggleModule={onToggleModule}
                onRollbackTheme={onRollbackTheme}
                onSearchGraph={onSearchGraph}
                onArchitectSectionChange={onArchitectSectionChange}
                onArchitectTabChange={onArchitectTabChange}
                activeArchitectTab={architectActiveTab}
                recoveryPanel={
                  <RecoveryPanel
                    onResetWidgets={onResetWidgets}
                    onRunRecovery={onRunRecovery}
                    recoveryPreview={recoveryPreview}
                  />
                }
                onResetWidgets={onResetWidgets}
                searchSnapshot={searchSnapshot}
                widgetActions={widgetActions}
                widgetContext={widgetContext}
                widgetInteractions={widgetInteractions}
                widgetState={widgetState}
                widgetTypes={widgetTypes}
                themePreview={themePreview}
                upgradePreview={upgradePreview}
                workspaceUiState={workspaceUiState}
                vaultPath={vaultPath}
              />
            ) : null}
            {activeView === "vault" ? (
              <VaultStatusPanel
                inspection={vaultInspection}
                onResetVault={onResetVault}
                resetting={vaultResetting}
              />
            ) : null}
            {activeView === "trash" ? (
              <TrashPanel recoveryPreview={recoveryPreview} vaultPath={vaultPath} />
            ) : null}
            {activeView === "archive" ? (
              <ArchivePanel recoveryPreview={recoveryPreview} vaultPath={vaultPath} />
            ) : null}
            {activeView === "module" ? (
              <GenericModulePanel module={activeModule} moduleId={activeModuleId} />
            ) : null}
            {activeView === "settings" ? (
              <SettingsPanel
                activeModuleId={activeModuleId}
                activeTheme={activeTheme}
                onApplyTheme={onApplyTheme}
                onCancelThemePreview={onCancelThemePreview}
                onOpenArchitect={() => onNavigate("architect", "Theme Registry", { architectTab: "appearance" })}
                onOpenModule={(moduleId) => onNavigate(viewForModule(moduleId), moduleId, { moduleId })}
                onPreviewTheme={onPreviewTheme}
                onRefreshWorkspace={onRefreshWorkspace}
                onReviewRecoveryIssues={() => onNavigate("architect", t("nav.recovery"), { architectTab: "recovery" })}
                onRollbackTheme={onRollbackTheme}
                onResetVault={onResetVault}
                language={language}
                onLanguageChange={onLanguageChange}
                recoveryIssueCount={recoveryIssueCount}
                themePreview={themePreview}
                resetting={vaultResetting}
                vaultInspection={vaultInspection}
                vaultPath={vaultPath}
              />
            ) : null}
          </ModuleErrorBoundary>
        </section>
      </div>
    </section>
  );
}

function GenericModulePanel({ module, moduleId }: { module: RegistryState["modules"][number] | null; moduleId: string | null }) {
  const { t } = useI18n();
  if (!module || !moduleId) {
    return <Empty className="min-h-80" title={t("genericModule.unavailable.title")} description={t("genericModule.unavailable.description")} />;
  }

  return (
    <div className="grid gap-4">
      <GeneratedModuleUI
        entity={{
          module_id: module.id,
          entity_type: module.document_type,
          fields: {
            title: module.display_name,
            kind: module.kind,
            default_view: module.default_view,
            available: module.available ? "yes" : "no",
            installed: module.installed ? "yes" : "no",
            enabled: module.enabled ? "yes" : "no",
            schema_version: module.schema_version ? `${module.schema_version}` : t("app.common.none"),
            migration_version: module.schema_migration_version ? `${module.schema_migration_version}` : t("app.common.none"),
          },
          field_descriptors: [
            { id: "kind", label: t("genericModule.fields.kind"), type: "text", renderer_id: "status", value: module.kind, editable: false, aliases: [], warnings: [] },
            { id: "default_view", label: t("genericModule.fields.defaultView"), type: "text", renderer_id: "text", value: module.default_view, editable: false, aliases: [], warnings: [] },
            { id: "document_type", label: t("genericModule.fields.documentType"), type: "text", renderer_id: "text", value: module.document_type, editable: false, aliases: [], warnings: [] },
            { id: "schema_version", label: t("genericModule.fields.schemaVersion"), type: "text", renderer_id: "text", value: module.schema_version ? `${module.schema_version}` : t("app.common.none"), editable: false, aliases: [], warnings: [] },
            { id: "migration_version", label: t("genericModule.fields.migrationVersion"), type: "text", renderer_id: "text", value: module.schema_migration_version ? `${module.schema_migration_version}` : t("app.common.none"), editable: false, aliases: [], warnings: [] },
            { id: "available", label: t("genericModule.fields.available"), type: "text", renderer_id: "status", value: module.available ? t("app.common.enabled") : t("genericModule.unavailable.short"), editable: false, aliases: [], warnings: [] },
            { id: "installed", label: t("genericModule.fields.installed"), type: "text", renderer_id: "status", value: module.installed ? t("genericModule.installed") : t("genericModule.notInstalled"), editable: false, aliases: [], warnings: [] },
            { id: "enabled", label: t("genericModule.fields.enabled"), type: "text", renderer_id: "status", value: module.enabled ? t("app.common.enabled") : t("app.common.disabled"), editable: false, aliases: [], warnings: [] },
          ],
          blocks: [
            { type: "paragraph", text: `${module.display_name} ${t("genericModule.fallbackText")}` },
          ],
          unknown_blocks: [],
          relationships: [],
          tags: module.capabilities,
          path: module.index_path,
          content_hash: module.schema_path ?? module.index_path,
        }}
        fields={[
          { id: "kind", label: t("genericModule.fields.kind"), renderer: "status" },
          { id: "default_view", label: t("genericModule.fields.defaultView"), renderer: "text" },
          { id: "document_type", label: t("genericModule.fields.documentType"), renderer: "text" },
          { id: "schema_version", label: t("genericModule.fields.schemaVersion"), renderer: "text" },
          { id: "migration_version", label: t("genericModule.fields.migrationVersion"), renderer: "text" },
          { id: "available", label: t("genericModule.fields.available"), renderer: "status" },
          { id: "installed", label: t("genericModule.fields.installed"), renderer: "status" },
          { id: "enabled", label: t("genericModule.fields.enabled"), renderer: "status" },
        ]}
        moduleLabel={module.display_name}
        schemaWarnings={(module.schema_warnings ?? []).length ? module.schema_warnings ?? [] : module.schema_path ? [] : [t("genericModule.schemaMissing")]}
        sourceMarkdown={`# ${module.display_name}\n\n- Index: ${module.index_path}\n- Data: ${module.data_path ?? "none"}\n`}
        title={module.display_name}
      />
    </div>
  );
}

function descriptionForView(view: AppView, t: (key: TranslationKey) => string, moduleLabel?: string) {
  switch (view) {
    case "notes":
      return t("views.notes.description");
    case "todos":
      return t("views.todos.description");
    case "contacts":
      return t("views.contacts.description");
    case "habits":
      return t("views.habits.description");
    case "navigator":
      return t("views.navigator.description");
    case "architect":
      return t("views.architect.description");
    case "vault":
      return t("views.vault.description");
    case "settings":
      return t("views.settings.description");
    case "trash":
      return t("views.trash.description");
    case "archive":
      return t("views.archive.description");
    case "module":
      return `${moduleLabel ?? t("nav.module")} ${t("views.module.description")}`;
    default:
      return t("views.default.description");
  }
}

function isRecoveryScanIssue(issue: WorkspaceScanResult["issues"][number]) {
  return issue.classification === "recovery_issue";
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium">{value}</span>
    </div>
  );
}

type RawConflictDescriptor = {
  checklistMarkdown?: string;
  enumFields?: Array<{ label: string; options: string[] }>;
  knownFields?: string[];
  rawMarkdown: string;
  requiredLabel?: string;
  structuredMarkdown: string;
  title?: string;
};

function detectRawMarkdownConflicts({
  checklistMarkdown,
  enumFields = [],
  knownFields = [],
  rawMarkdown,
  requiredLabel,
  structuredMarkdown,
  title,
}: RawConflictDescriptor) {
  const warnings = new Set<string>();
  if (rawMarkdown.trim() !== structuredMarkdown.trim()) {
    warnings.add("Structured fields and raw Markdown no longer match.");
  }
  if (requiredLabel && !title?.trim()) {
    warnings.add(`${requiredLabel} is required before structured fields can be saved.`);
  }
  const rawFields = parseRawFieldLines(rawMarkdown);
  const allKnownFields = new Set([...knownFields, ...enumFields.map((field) => field.label)].map(normalizeFieldName));
  for (const [field, values] of rawFields) {
    if (!allKnownFields.has(normalizeFieldName(field))) {
      warnings.add(`Unknown raw field "${field}" will remain recoverable.`);
    }
    if (values.length > 1) {
      warnings.add(`Duplicate raw field "${field}" needs an explicit save choice.`);
    }
    if (["relationships", "related"].includes(normalizeFieldName(field))) {
      const relationshipValues = values.flatMap(inputToList).map((value) => value.toLowerCase());
      if (new Set(relationshipValues).size !== relationshipValues.length) {
        warnings.add("Duplicate relationships need an explicit save choice.");
      }
    }
  }
  enumFields.forEach((field) => {
    const rawValue = rawFields.get(normalizeFieldName(field.label))?.[0];
    if (rawValue && !field.options.some((option) => option.toLowerCase() === rawValue.toLowerCase())) {
      warnings.add(`${field.label} value "${rawValue}" is outside the approved options.`);
    }
  });
  if (checklistMarkdown && parseTodoTasks(rawMarkdown).length !== parseTodoTasks(checklistMarkdown).length) {
    warnings.add("Checklist item count changed between raw Markdown and structured checklist state.");
  }
  return [...warnings];
}

function parseRawFieldLines(markdown: string) {
  const fields = new Map<string, string[]>();
  markdown.split(/\r?\n/).forEach((line) => {
    const match = line.match(/^([A-Za-z][A-Za-z0-9 _/-]*):\s*(.*)$/);
    if (!match) return;
    const key = normalizeFieldName(match[1]);
    fields.set(key, [...(fields.get(key) ?? []), match[2].trim()]);
  });
  return fields;
}

function normalizeFieldName(field: string) {
  return field.trim().toLowerCase().replace(/[_\s-]+/g, " ");
}

function rawFieldValue(markdown: string, labels: string[]) {
  const fields = parseRawFieldLines(markdown);
  for (const label of labels) {
    const value = fields.get(normalizeFieldName(label))?.[0];
    if (value) return value;
  }
  return "";
}

function upsertRawFieldLine(markdown: string, label: string, value: string) {
  const normalized = normalizeFieldName(label);
  const lines = markdown.trimEnd().split(/\r?\n/);
  const nextValue = value.trim();
  const index = lines.findIndex((line) => {
    const match = line.match(/^([A-Za-z][A-Za-z0-9 _/-]*):/);
    return Boolean(match && normalizeFieldName(match[1]) === normalized);
  });
  if (!nextValue) {
    return `${lines.filter((_, lineIndex) => lineIndex !== index).join("\n").trimEnd()}\n`;
  }
  const nextLine = `${label}: ${nextValue}`;
  if (index >= 0) {
    lines[index] = nextLine;
    return `${lines.join("\n").trimEnd()}\n`;
  }
  const insertAfter = Math.max(0, lines.findIndex((line) => line.trim() === ""));
  lines.splice(insertAfter + 1, 0, nextLine);
  return `${lines.join("\n").trimEnd()}\n`;
}

function listToInput(values?: string[]) {
  return values?.join(", ") ?? "";
}

function inputToList(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function rawLinesNotInKnownFields(markdown: string, knownFields: string[]) {
  const known = new Set(knownFields.map(normalizeFieldName));
  return markdown
    .split(/\r?\n/)
    .filter((line) => {
      if (!line.trim() || line.trim().startsWith("#")) return false;
      const match = line.match(/^\s*[-*]\s+([^:\n]+):/);
      return Boolean(match && !known.has(normalizeFieldName(match[1])));
    })
    .join("\n")
    .trim();
}

function appendPreservedMarkdown(notes: string | null | undefined, preservedMarkdown: string) {
  const trimmedNotes = notes?.trim() ?? "";
  const trimmedPreserved = preservedMarkdown.trim();
  if (!trimmedPreserved) return notes ?? "";
  return `${trimmedNotes ? `${trimmedNotes}\n\n` : ""}## Preserved Markdown\n\n${trimmedPreserved}`;
}

type TodoPanelProps = {
  backlinks: ModuleBacklink[];
  loading: boolean;
  todos: TodoSummary[];
  onCreateTodo: (title?: string, markdownBody?: string) => Promise<TodoDocument | null>;
  onSaveAsNote: (title?: string, markdownBody?: string) => Promise<NoteDocument | null>;
  onRenameTodo: (documentId: string, newTitle: string) => void;
  onSaveTodo: (documentId: string, markdownBody: string) => void;
  onSelectTodo: (documentId: string) => void;
  selectedTodo: TodoDocument | null;
  workspaceError: string | null;
};

function TodoPanel({
  backlinks,
  loading,
  todos,
  onCreateTodo,
  onSaveAsNote,
  onRenameTodo,
  onSaveTodo,
  onSelectTodo,
  selectedTodo,
  workspaceError,
}: TodoPanelProps) {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [draftTitle, setDraftTitle] = useState("");
  const [draftMarkdown, setDraftMarkdown] = useState("");
  const [checklistItem, setChecklistItem] = useState("");
  const [isCreatingTask, setIsCreatingTask] = useState(false);
  const [createTitle, setCreateTitle] = useState("");
  const [createStatus, setCreateStatus] = useState("Not started");
  const [createPriority, setCreatePriority] = useState("Medium");
  const [createDueDate, setCreateDueDate] = useState("");
  const [createTags, setCreateTags] = useState("");
  const [createRelationships, setCreateRelationships] = useState("");
  const [createChecklistStarter, setCreateChecklistStarter] = useState("");
  const [createRawMarkdown, setCreateRawMarkdown] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [rawConflictChoice, setRawConflictChoice] = useState<RawConflictChoice>("structured");

  useEffect(() => {
    setDraftTitle(selectedTodo?.title ?? "");
    setDraftMarkdown(selectedTodo?.markdown_body ?? "");
    setRawConflictChoice("structured");
    setIsEditing(false);
  }, [selectedTodo]);

  const filteredTodos = todos.filter((todos) =>
    `${todos.title} ${todos.markdown_relative_path} ${todos.excerpt}`.toLowerCase().includes(query.toLowerCase()),
  );
  const selectedTasks = selectedTodo ? parseTodoTasks(selectedTodo.markdown_body) : [];
  const todoStatus = rawFieldValue(draftMarkdown, ["status"]) || "Not started";
  const todoPriority = rawFieldValue(draftMarkdown, ["priority"]) || "Medium";
  const todoDueDate = rawFieldValue(draftMarkdown, ["due date", "due"]) || "";
  const todoTags = rawFieldValue(draftMarkdown, ["tags"]) || "";
  const todoRelationships = rawFieldValue(draftMarkdown, ["relationships", "related"]) || "";
  const todoDirty = Boolean(selectedTodo && (draftTitle !== selectedTodo.title || draftMarkdown !== selectedTodo.markdown_body));
  const todoConflictWarnings = selectedTodo && todoDirty
    ? detectRawMarkdownConflicts({
        rawMarkdown: selectedTodo.markdown_body,
        structuredMarkdown: draftMarkdown,
        title: draftTitle,
        requiredLabel: "Title",
        enumFields: [
          { label: "Status", options: TODO_STATUS_OPTIONS },
          { label: "Priority", options: TODO_PRIORITY_OPTIONS },
        ],
        knownFields: ["Status", "Priority", "Due date", "Due", "Tags", "Relationships", "Related"],
        checklistMarkdown: draftMarkdown,
      })
    : [];
  const updateTodoMarkdownField = (label: string, value: string) => {
    setDraftMarkdown((current) => upsertRawFieldLine(current || selectedTodo?.markdown_body || `# ${draftTitle || "Todos"}\n`, label, value));
  };
  const cancelTodoEdit = () => {
    setDraftTitle(selectedTodo?.title ?? "");
    setDraftMarkdown(selectedTodo?.markdown_body ?? "");
    setRawConflictChoice("structured");
    setIsEditing(false);
  };
  const saveTodoEdit = async () => {
    if (!selectedTodo || rawConflictChoice === "cancel") return;
    if (rawConflictChoice === "raw") {
      cancelTodoEdit();
      return;
    }
    if (rawConflictChoice === "note") {
      await onSaveAsNote(`${draftTitle || selectedTodo.title} raw copy`, draftMarkdown);
    }
    if (draftTitle.trim() && draftTitle !== selectedTodo.title) {
      onRenameTodo(selectedTodo.document_id, draftTitle);
    }
    onSaveTodo(selectedTodo.document_id, draftMarkdown);
    setIsEditing(false);
  };
  const cancelCreateTask = () => {
    setIsCreatingTask(false);
    setCreateTitle("");
    setCreateStatus("Not started");
    setCreatePriority("Medium");
    setCreateDueDate("");
    setCreateTags("");
    setCreateRelationships("");
    setCreateChecklistStarter("");
    setCreateRawMarkdown("");
  };
  const saveCreateTask = async () => {
    const title = createTitle.trim();
    if (!title) return;
    const markdown = createRawMarkdown.trim()
      ? createRawMarkdown
      : createTodoMarkdown({
          title,
          status: createStatus,
          priority: createPriority,
          dueDate: createDueDate,
          tags: createTags,
          relationships: createRelationships,
          checklistStarter: createChecklistStarter,
        });
    const created = await onCreateTodo(title, markdown);
    if (created) {
      cancelCreateTask();
    }
  };
  const addChecklistItem = () => {
    const title = checklistItem.trim();
    if (!title) return;
    if (!selectedTodo) return;
    const markdown = appendTodoTask(draftMarkdown || selectedTodo.markdown_body, title);
    setDraftMarkdown(markdown);
    onSaveTodo(selectedTodo.document_id, markdown);
    setChecklistItem("");
  };
  const toggleTask = (index: number, completed: boolean) => {
    if (!selectedTodo) return;
    const markdown = setTodoTaskCompletion(draftMarkdown || selectedTodo.markdown_body, index, completed);
    setDraftMarkdown(markdown);
    onSaveTodo(selectedTodo.document_id, markdown);
  };

  return (
    <div className="grid gap-5 xl:grid-cols-[18rem_minmax(0,1fr)]">
      <div className="flex min-w-0 flex-col gap-4">
        <div className="flex gap-2">
          <div className="relative min-w-0 flex-1">
            <Search aria-hidden="true" className="pointer-events-none absolute left-3 top-3 text-muted-foreground" data-icon="inline-start" />
            <Input
              aria-label={t("todos.search")}
              className="pl-9"
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("todos.search")}
              value={query}
            />
          </div>
          <Button aria-label={t("todos.addTask")} onClick={() => setIsCreatingTask(true)}>
            <Plus aria-hidden="true" data-icon="inline-start" />
            {t("todos.addTask")}
          </Button>
        </div>

        <div className="flex max-h-[30rem] min-w-0 flex-col gap-2 overflow-auto pr-1">
          {filteredTodos.map((todos) => (
            <button
              className="min-w-0 rounded-md border border-border bg-background p-3 text-left transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              key={todos.document_id}
              onClick={() => onSelectTodo(todos.document_id)}
              type="button"
            >
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={todos.is_completed}
                  readOnly
                  className="size-4"
                />
                <p className={`truncate text-sm font-medium ${todos.is_completed ? "line-through text-muted-foreground" : ""}`}>
                  {todos.title}
                </p>
              </div>
              <p className="mt-1 truncate text-xs text-muted-foreground">{todos.markdown_relative_path}</p>
              <p className="mt-2 line-clamp-2 text-xs leading-5 text-muted-foreground">{todos.excerpt}</p>
            </button>
          ))}
          {!filteredTodos.length ? (
            <Empty
              className="min-h-48"
              title={todos.length ? t("todos.empty.noMatches") : t("todos.empty.none")}
              description={todos.length ? t("todos.empty.tryDifferent") : t("todos.empty.createMarkdown")}
            />
          ) : null}
        </div>
        <EntityEditDrawer
          conflictWarnings={[]}
          description={t("todos.drawer.createDescription")}
          dirty={Boolean(createTitle.trim() || createRawMarkdown.trim())}
          onCancel={cancelCreateTask}
          onOpenChange={(open) => {
            if (!open) cancelCreateTask();
            else setIsCreatingTask(true);
          }}
          onSave={saveCreateTask}
          open={isCreatingTask}
          saveDisabled={!createTitle.trim()}
          saveLabel={t("todos.createTask")}
          title={t("todos.addTask")}
        >
          <TextField label={t("fields.title")} value={createTitle} onChange={setCreateTitle} />
          <div className="grid gap-3 md:grid-cols-2">
            <SelectField label={t("fields.status")} options={TODO_STATUS_OPTIONS} value={createStatus} onChange={setCreateStatus} />
            <SelectField label={t("fields.priority")} options={TODO_PRIORITY_OPTIONS} value={createPriority} onChange={setCreatePriority} />
            <DateField label={t("fields.dueDate")} value={createDueDate} onChange={setCreateDueDate} />
            <TagsField value={createTags} onChange={setCreateTags} />
          </div>
          <details className="rounded-md border border-border bg-muted/35 p-3 text-sm [&:not([open])>*:not(summary)]:hidden">
            <summary className="cursor-pointer font-medium">{t("modules.editor.advancedRelated")}</summary>
            <p className="mt-2 text-xs leading-5 text-muted-foreground">{t("fields.relatedEntitiesTooltip")}</p>
            <div className="mt-3">
              <EntityLinksField value={createRelationships} onChange={setCreateRelationships} />
            </div>
          </details>
          <ChecklistField value={createChecklistStarter} onChange={setCreateChecklistStarter} />
          <details className="rounded-md border border-border bg-muted/35 p-3 text-sm [&:not([open])>*:not(summary)]:hidden">
            <summary className="cursor-pointer font-medium">{t("modules.editor.rawMarkdownAdvanced")}</summary>
            <TextAreaField
              label={t("modules.editor.markdownSource")}
              value={createRawMarkdown}
              onChange={setCreateRawMarkdown}
            />
          </details>
        </EntityEditDrawer>
      </div>

      <div className="min-w-0">
        {workspaceError ? <RepairNotice title="Todos failed to load" message={workspaceError} /> : null}
        {loading ? <Skeleton className="h-96" /> : null}
        {!loading && selectedTodo ? (
          <div className="flex min-w-0 flex-col gap-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="min-w-0">
                <h2 className="truncate text-lg font-semibold">{selectedTodo.title}</h2>
                <p className="text-sm text-muted-foreground">{selectedTodo.markdown_relative_path}</p>
              </div>
              <Button onClick={() => setIsEditing(true)} variant="outline">
                <Pencil data-icon="inline-start" />
                {t("todos.editTask")}
              </Button>
            </div>
            <div className="rounded-md border border-border bg-background p-3">
              <p className="text-xs font-semibold uppercase text-muted-foreground">{t("widgets.labels.checklist")}</p>
              {selectedTasks.length ? (
                <div className="mt-3 flex flex-col gap-2">
                  {selectedTasks.map((task, index) => (
                    <label className="flex items-center gap-2 text-sm" key={`${task.text}-${index}`}>
                      <input
                        aria-label={task.text}
                        checked={task.checked}
                        className="size-4"
                        onChange={(event) => toggleTask(index, event.target.checked)}
                        type="checkbox"
                      />
                      <span className={task.checked ? "text-muted-foreground line-through" : ""}>{task.text}</span>
                    </label>
                  ))}
                </div>
              ) : (
                <p className="mt-3 text-sm text-muted-foreground">{t("todos.checklist.empty")}</p>
              )}
              <div className="mt-3 flex gap-2">
                <Input
                  aria-label={t("todos.checklist.addItem")}
                  onChange={(event) => setChecklistItem(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") addChecklistItem();
                  }}
                  placeholder={t("todos.checklist.addItem")}
                  value={checklistItem}
                />
                <Button disabled={!checklistItem.trim()} onClick={addChecklistItem}>
                  {t("app.actions.add")}
                </Button>
              </div>
            </div>
            <TodosGeneratedUI backlinks={backlinks} todos={selectedTodo} />
            <details className="rounded-md border border-border bg-muted/35 p-3 text-sm">
              <summary className="cursor-pointer font-medium">{t("modules.editor.inspector")}</summary>
              <div className="mt-3 grid gap-2 text-muted-foreground">
                <SummaryRow label={t("fields.path")} value={selectedTodo.markdown_relative_path} />
                <SummaryRow label={t("fields.hash")} value={selectedTodo.parsed_entity.content_hash.slice(0, 12) || t("app.common.working")} />
                <SummaryRow label={t("fields.schemaWarnings")} value={`${selectedTodo.schema_warnings.length}`} />
                <pre className="mt-2 max-h-72 overflow-auto whitespace-pre-wrap break-words rounded-md bg-background p-3 text-xs">{selectedTodo.markdown_body}</pre>
              </div>
            </details>
            <EntityEditDrawer
              conflictChoice={rawConflictChoice}
              conflictWarnings={todoConflictWarnings}
              description={t("todos.drawer.editDescription")}
              dirty={todoDirty}
              onCancel={cancelTodoEdit}
              onConflictChoiceChange={setRawConflictChoice}
              onOpenChange={setIsEditing}
              onSave={saveTodoEdit}
              open={isEditing}
              saveDisabled={!draftTitle.trim() || !draftMarkdown.trim() || rawConflictChoice === "cancel"}
              saveLabel={rawConflictChoice === "raw" ? t("modules.drawer.choice.raw") : rawConflictChoice === "note" ? t("todos.saveNoteAndTask") : t("todos.saveTask")}
              title={t("todos.editTask")}
            >
              <TextField
                label={t("fields.title")}
                value={draftTitle}
                onChange={(title) => {
                  setDraftTitle(title);
                  setDraftMarkdown((current) => replaceOrInsertTitle(current || selectedTodo.markdown_body, title));
                }}
              />
              <div className="grid gap-3 md:grid-cols-2">
                <SelectField
                  label={t("fields.status")}
                  options={TODO_STATUS_OPTIONS}
                  value={todoStatus}
                  onChange={(status) => updateTodoMarkdownField("Status", status)}
                />
                <SelectField
                  label={t("fields.priority")}
                  options={TODO_PRIORITY_OPTIONS}
                  value={todoPriority}
                  onChange={(priority) => updateTodoMarkdownField("Priority", priority)}
                />
                <DateField label={t("fields.dueDate")} value={todoDueDate} onChange={(due) => updateTodoMarkdownField("Due date", due)} />
                <TagsField value={todoTags} onChange={(tags) => updateTodoMarkdownField("Tags", tags)} />
              </div>
              <details className="rounded-md border border-border bg-muted/35 p-3 text-sm [&:not([open])>*:not(summary)]:hidden">
                <summary className="cursor-pointer font-medium">{t("modules.editor.advancedRelated")}</summary>
                <p className="mt-2 text-xs leading-5 text-muted-foreground">{t("fields.relatedEntitiesTooltip")}</p>
                <div className="mt-3">
                  <EntityLinksField value={todoRelationships} onChange={(relationships) => updateTodoMarkdownField("Relationships", relationships)} />
                </div>
              </details>
              <TextAreaField label={t("modules.editor.markdownSource")} value={draftMarkdown} onChange={setDraftMarkdown} />
              <p className="text-xs leading-5 text-muted-foreground">
                {t("todos.rawPreserveNote")}
              </p>
            </EntityEditDrawer>
          </div>
        ) : null}
        {!loading && !selectedTodo ? (
          <Empty title={t("todos.empty.selectOrCreate")} description={t("todos.empty.storage")} />
        ) : null}
      </div>
    </div>
  );
}

function parseTodoTasks(markdown: string) {
  return markdown
    .split(/\r?\n/)
    .map((line, lineIndex) => {
      const match = line.match(/^(\s*[-*]\s+\[)([ xX])(\]\s+)(.+)$/);
      return match ? { checked: match[2].toLowerCase() === "x", lineIndex, text: match[4].trim() } : null;
    })
    .filter((task): task is { checked: boolean; lineIndex: number; text: string } => Boolean(task));
}

function appendTodoTask(markdown: string, task: string) {
  const trimmed = markdown.trimEnd();
  const prefix = trimmed ? `${trimmed}\n` : "# Todos\n\n";
  return `${prefix}- [ ] ${task}\n`;
}

function createTodoMarkdown({
  title,
  status,
  priority,
  dueDate,
  tags,
  relationships,
  checklistStarter,
}: {
  title: string;
  status: string;
  priority: string;
  dueDate: string;
  tags: string;
  relationships: string;
  checklistStarter: string;
}) {
  const lines = [`# ${title}`, "", `Status: ${status || "Not started"}`, `Priority: ${priority || "Medium"}`];
  if (dueDate.trim()) lines.push(`Due date: ${dueDate.trim()}`);
  if (tags.trim()) lines.push(`Tags: ${tags.trim()}`);
  if (relationships.trim()) lines.push(`Relationships: ${relationships.trim()}`);
  const starter = checklistStarter.trim();
  if (starter) {
    lines.push("", "## Checklist", "", `- [ ] ${starter}`);
  }
  return `${lines.join("\n").trimEnd()}\n`;
}

function setTodoTaskCompletion(markdown: string, taskIndex: number, completed: boolean) {
  const lines = markdown.split(/\r?\n/);
  let seen = -1;
  return `${lines
    .map((line) => {
      const match = line.match(/^(\s*[-*]\s+\[)([ xX])(\]\s+.+)$/);
      if (!match) return line;
      seen += 1;
      return seen === taskIndex ? `${match[1]}${completed ? "x" : " "}${match[3]}` : line;
    })
    .join("\n")
    .trimEnd()}\n`;
}

function replaceOrInsertTitle(markdown: string, title: string) {
  const lines = markdown.trimEnd().split(/\r?\n/);
  const nextTitle = title.trim() || "Untitled Todo";
  const index = lines.findIndex((line) => line.startsWith("# "));
  if (index >= 0) {
    lines[index] = `# ${nextTitle}`;
  } else {
    lines.unshift(`# ${nextTitle}`);
  }
  return `${lines.join("\n").trimEnd()}\n`;
}

type ContactsPanelProps = {
  backlinks: ModuleBacklink[];
  contacts: ContactDocument | null;
  loading: boolean;
  onSaveAsNote: (title?: string, markdownBody?: string) => Promise<NoteDocument | null>;
  onSaveContact: (contactId: string | null, input: ContactInput) => void;
  onSelectContact: (contactId: string | null) => void;
  selectedContactId: string | null;
  workspaceError: string | null;
};

function ContactsPanel({
  backlinks,
  contacts,
  loading,
  onSaveAsNote,
  onSaveContact,
  onSelectContact,
  selectedContactId,
  workspaceError,
}: ContactsPanelProps) {
  const { t } = useI18n();
  const selectedContact = contacts?.contacts.find((contact) => contact.contact_id === selectedContactId) ?? null;
  const [creating, setCreating] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);

  useEffect(() => {
    if (selectedContactId) {
      setCreating(false);
    }
  }, [selectedContactId]);

  return (
    <div className="grid gap-5 xl:grid-cols-[18rem_minmax(0,1fr)]">
      <div className="flex min-w-0 flex-col gap-4">
        <div className="grid grid-cols-2 gap-3">
          <MetricCard label={t("contacts.metrics.contacts")} value={String(contacts?.summary.total ?? 0)} />
          <MetricCard label={t("contacts.metrics.withEmail")} value={String(contacts?.summary.contacts_with_email ?? 0)} />
        </div>
        <Button onClick={() => { setCreating(true); onSelectContact(null); setEditorOpen(true); }} variant="outline">
          <Plus data-icon="inline-start" />
          {t("contacts.newContact")}
        </Button>
        <div className="flex max-h-[30rem] min-w-0 flex-col gap-2 overflow-auto pr-1">
          {contacts?.contacts.map((contact) => (
            <button
              className="min-w-0 rounded-md border border-border bg-background p-3 text-left transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              key={contact.contact_id}
              onClick={() => onSelectContact(contact.contact_id)}
              type="button"
            >
              <p className="truncate text-sm font-medium">{contact.name}</p>
              <p className="mt-1 truncate text-xs text-muted-foreground">{contact.relationship || t("contacts.noRelationship")}</p>
              <p className="mt-2 truncate text-xs text-muted-foreground">{contact.tags.join(", ") || t("renderer.noTags")}</p>
            </button>
          ))}
          {!contacts?.contacts.length ? (
            <Empty className="min-h-48" title={t("contacts.empty.none")} description={t("contacts.empty.createMarkdown")} />
          ) : null}
        </div>
      </div>
      <div className="min-w-0">
        {workspaceError ? <RepairNotice title={t("contacts.failedToLoad")} message={workspaceError} /> : null}
        {contacts?.warnings.map((warning) => (
          <RepairNotice key={warning} title={t("contacts.schemaWarning")} message={warning} />
        ))}
        {loading ? (
          <Skeleton className="h-96" />
        ) : (
          <div className="space-y-5">
            {contacts && selectedContact ? (
              <>
                <div className="flex justify-end">
                  <Button onClick={() => setEditorOpen(true)} variant="outline">
                    <Pencil data-icon="inline-start" />
                    {t("contacts.editContact")}
                  </Button>
                </div>
                <ContactsGeneratedUI backlinks={backlinks} contact={selectedContact} document={contacts} />
              </>
            ) : (
              <Empty title={t("contacts.empty.selectOrCreate")} description={t("contacts.empty.chooseOrCreate")} />
            )}
            <ContactEditor
              contact={creating ? null : selectedContact}
              onCancel={() => {
                setCreating(false);
                setEditorOpen(false);
              }}
              onOpenChange={(open) => {
                setEditorOpen(open);
                if (!open) setCreating(false);
              }}
              onSave={(contactId, input) => {
                setCreating(false);
                setEditorOpen(false);
                onSaveContact(contactId, input);
              }}
              onSaveAsNote={onSaveAsNote}
              open={editorOpen}
            />
          </div>
        )}
      </div>
    </div>
  );
}

function ContactEditor({
  contact,
  onCancel,
  onOpenChange,
  onSave,
  onSaveAsNote,
  open,
}: {
  contact: ContactEntry | null;
  onCancel: () => void;
  onOpenChange: (open: boolean) => void;
  onSave: (contactId: string | null, input: ContactInput) => void;
  onSaveAsNote: (title?: string, markdownBody?: string) => Promise<NoteDocument | null>;
  open: boolean;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<ContactInput>(emptyContactInput());
  const [rawDraft, setRawDraft] = useState("");
  const [rawConflictChoice, setRawConflictChoice] = useState<RawConflictChoice>("structured");

  useEffect(() => {
    const nextDraft = contact ? contactToInput(contact) : emptyContactInput();
    setDraft(nextDraft);
    setRawDraft(contact?.raw_markdown ?? renderContactDraftMarkdown(nextDraft));
    setRawConflictChoice("structured");
  }, [contact]);

  const baseline = contact ? contactToInput(contact) : emptyContactInput();
  const baselineRaw = contact?.raw_markdown ?? renderContactDraftMarkdown(baseline);
  const dirty = !contactInputEquals(draft, baseline) || rawDraft.trim() !== baselineRaw.trim();
  const knownFields = ["Relationship", "Organization", "Email", "Phone", "Tags", "Relationships", "Notes"];
  const conflictWarnings = contact && dirty
    ? detectRawMarkdownConflicts({
        rawMarkdown: rawDraft,
        structuredMarkdown: renderContactDraftMarkdown(draft),
        title: draft.name,
        requiredLabel: "Name",
        knownFields,
        enumFields: [{ label: "Relationship", options: CONTACT_RELATIONSHIP_OPTIONS }],
      })
    : [];
  const resetDraft = () => {
    const nextDraft = contact ? contactToInput(contact) : emptyContactInput();
    setDraft(nextDraft);
    setRawDraft(contact?.raw_markdown ?? renderContactDraftMarkdown(nextDraft));
    setRawConflictChoice("structured");
  };
  const saveContact = async () => {
    if (rawConflictChoice === "cancel") return;
    if (rawConflictChoice === "raw") {
      resetDraft();
      onCancel();
      return;
    }
    let input = rawConflictChoice === "convert" ? mergeContactRawFields(draft, rawDraft) : draft;
    if (contact) {
      const preservedMarkdown = rawLinesNotInKnownFields(rawDraft, knownFields);
      input = { ...input, notes: appendPreservedMarkdown(input.notes, preservedMarkdown) };
    }
    if (rawConflictChoice === "note") {
      await onSaveAsNote(`${draft.name || contact?.name || t("widgets.labels.contact")} ${t("modules.drawer.rawCopy")}`, rawDraft);
    }
    onSave(contact?.contact_id ?? null, input);
  };

  return (
    <EntityEditDrawer
      conflictChoice={rawConflictChoice}
      conflictWarnings={conflictWarnings}
      description={t("contacts.drawer.description")}
      dirty={dirty}
      onCancel={() => {
        resetDraft();
        onCancel();
      }}
      onConflictChoiceChange={setRawConflictChoice}
      onOpenChange={onOpenChange}
      onSave={saveContact}
      open={open}
      saveDisabled={!draft.name.trim() || rawConflictChoice === "cancel"}
      saveLabel={rawConflictChoice === "raw" ? t("modules.drawer.choice.raw") : rawConflictChoice === "note" ? t("contacts.saveNoteAndContact") : contact ? t("contacts.saveContact") : t("contacts.createContact")}
      title={contact ? t("contacts.editContact") : t("contacts.newContact")}
    >
      <div className="grid gap-3 md:grid-cols-2">
        <TextField label={t("fields.name")} value={draft.name} onChange={(name) => setDraft((current) => ({ ...current, name }))} />
        <SelectField
          label={t("fields.relationship")}
          options={CONTACT_RELATIONSHIP_OPTIONS}
          value={draft.relationship ?? ""}
          onChange={(relationship) => setDraft((current) => ({ ...current, relationship }))}
        />
        <TextField
          label={t("fields.organization")}
          value={draft.organization ?? ""}
          onChange={(organization) => setDraft((current) => ({ ...current, organization }))}
        />
        <TextField label={t("fields.email")} value={draft.email ?? ""} onChange={(email) => setDraft((current) => ({ ...current, email }))} />
        <TextField label={t("fields.phone")} value={draft.phone ?? ""} onChange={(phone) => setDraft((current) => ({ ...current, phone }))} />
      </div>
      <TextField
        label={t("fields.tags")}
        value={listToInput(draft.tags)}
        onChange={(tags) => setDraft((current) => ({ ...current, tags: inputToList(tags) }))}
      />
      <details className="rounded-md border border-border bg-muted/35 p-3 text-sm [&:not([open])>*:not(summary)]:hidden">
        <summary className="cursor-pointer font-medium">{t("modules.editor.advancedRelated")}</summary>
        <p className="mt-2 text-xs leading-5 text-muted-foreground">
          {t("contacts.relatedHelp")}
        </p>
        <div className="mt-3">
          <TextField
            label={t("fields.relatedEntities")}
            value={listToInput(draft.relationships)}
            onChange={(relationships) => setDraft((current) => ({ ...current, relationships: inputToList(relationships) }))}
          />
        </div>
      </details>
      <TextAreaField label={t("fields.notes")} value={draft.notes ?? ""} onChange={(notes) => setDraft((current) => ({ ...current, notes }))} />
      <TextAreaField label={t("modules.editor.rawMarkdownSource")} value={rawDraft} onChange={setRawDraft} />
      <p className="text-xs leading-5 text-muted-foreground">
        {t("modules.drawer.rawEditHelp")}
      </p>
    </EntityEditDrawer>
  );
}

type HabitsPanelProps = {
  backlinks: ModuleBacklink[];
  habits: HabitDocument | null;
  loading: boolean;
  onCheckin: (habitId: string) => void;
  onSaveAsNote: (title?: string, markdownBody?: string) => Promise<NoteDocument | null>;
  onSaveHabit: (habitId: string | null, input: HabitInput) => void;
  onSelectHabit: (habitId: string | null) => void;
  selectedHabitId: string | null;
  workspaceError: string | null;
};

function HabitsPanel({
  backlinks,
  habits,
  loading,
  onCheckin,
  onSaveAsNote,
  onSaveHabit,
  onSelectHabit,
  selectedHabitId,
  workspaceError,
}: HabitsPanelProps) {
  const { t } = useI18n();
  const selectedHabit = habits?.habits.find((habit) => habit.habit_id === selectedHabitId) ?? null;
  const [creating, setCreating] = useState(false);
  const [editorOpen, setEditorOpen] = useState(false);
  const today = todayKey();

  useEffect(() => {
    if (selectedHabitId) {
      setCreating(false);
    }
  }, [selectedHabitId]);

  return (
    <div className="grid gap-5 xl:grid-cols-[18rem_minmax(0,1fr)]">
      <div className="flex min-w-0 flex-col gap-4">
        <div className="grid grid-cols-2 gap-3">
          <MetricCard label={t("nav.habits")} value={String(habits?.summary.total ?? 0)} />
          <MetricCard label={t("habits.metrics.today")} value={`${habits?.summary.checked_in_on_date ?? 0}`} />
        </div>
        <Button onClick={() => { setCreating(true); onSelectHabit(null); setEditorOpen(true); }} variant="outline">
          <Plus data-icon="inline-start" />
          {t("habits.newHabit")}
        </Button>
        <div className="flex max-h-[30rem] min-w-0 flex-col gap-2 overflow-auto pr-1">
          {habits?.habits.map((habit) => (
            <button
              className="min-w-0 rounded-md border border-border bg-background p-3 text-left transition-colors hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              key={habit.habit_id}
              onClick={() => onSelectHabit(habit.habit_id)}
              type="button"
            >
              <p className="truncate text-sm font-medium">{habit.name}</p>
              <p className="mt-1 truncate text-xs text-muted-foreground">{habit.frequency || t("habits.noFrequency")}</p>
              <p className="mt-2 truncate text-xs text-muted-foreground">
                {habit.checkins.includes(today) ? t("habits.checkedInToday") : t("habits.openToday")}
              </p>
            </button>
          ))}
          {!habits?.habits.length ? <Empty className="min-h-48" title={t("habits.empty.none")} description={t("habits.empty.createLocal")} /> : null}
        </div>
      </div>
      <div className="min-w-0">
        {workspaceError ? <RepairNotice title={t("habits.failedToLoad")} message={workspaceError} /> : null}
        {habits?.warnings.map((warning) => (
          <RepairNotice key={warning} title={t("habits.schemaWarning")} message={warning} />
        ))}
        {loading ? (
          <Skeleton className="h-96" />
        ) : (
          <div className="space-y-5">
            {habits && selectedHabit ? (
              <>
                <div className="flex flex-wrap justify-end gap-3">
                  <Button disabled={selectedHabit.checkins.includes(today)} onClick={() => onCheckin(selectedHabit.habit_id)} variant="outline">
                    <CalendarCheck data-icon="inline-start" />
                    {selectedHabit.checkins.includes(today) ? t("habits.checkedInToday") : t("habits.checkInToday")}
                  </Button>
                  <Button onClick={() => setEditorOpen(true)} variant="outline">
                    <Pencil data-icon="inline-start" />
                    {t("habits.editHabit")}
                  </Button>
                </div>
                <HabitsGeneratedUI backlinks={backlinks} document={habits} habit={selectedHabit} />
              </>
            ) : (
              <Empty title={t("habits.empty.selectOrCreate")} description={t("habits.empty.chooseOrCreate")} />
            )}
            <HabitEditor
              habit={creating ? null : selectedHabit}
              habits={habits}
              onCancel={() => {
                setCreating(false);
                setEditorOpen(false);
              }}
              onOpenChange={(open) => {
                setEditorOpen(open);
                if (!open) setCreating(false);
              }}
              onSave={(habitId, input) => {
                setCreating(false);
                setEditorOpen(false);
                onSaveHabit(habitId, input);
              }}
              onSaveAsNote={onSaveAsNote}
              open={editorOpen}
            />
          </div>
        )}
      </div>
    </div>
  );
}

function HabitEditor({
  habit,
  habits,
  onCancel,
  onOpenChange,
  onSave,
  onSaveAsNote,
  open,
}: {
  habit: HabitEntry | null;
  habits: HabitDocument | null;
  onCancel: () => void;
  onOpenChange: (open: boolean) => void;
  onSave: (habitId: string | null, input: HabitInput) => void;
  onSaveAsNote: (title?: string, markdownBody?: string) => Promise<NoteDocument | null>;
  open: boolean;
}) {
  const { t } = useI18n();
  const [draft, setDraft] = useState<HabitInput>(emptyHabitInput());
  const [rawDraft, setRawDraft] = useState("");
  const [rawConflictChoice, setRawConflictChoice] = useState<RawConflictChoice>("structured");
  const today = todayKey();
  const streak = habits?.summary.streaks.find((candidate) => candidate.habit_id === habit?.habit_id)?.current_streak ?? 0;

  useEffect(() => {
    const nextDraft = habit ? habitToInput(habit) : emptyHabitInput();
    setDraft(nextDraft);
    setRawDraft(habit?.raw_markdown ?? renderHabitDraftMarkdown(nextDraft));
    setRawConflictChoice("structured");
  }, [habit]);

  const baseline = habit ? habitToInput(habit) : emptyHabitInput();
  const baselineRaw = habit?.raw_markdown ?? renderHabitDraftMarkdown(baseline);
  const dirty = !habitInputEquals(draft, baseline) || rawDraft.trim() !== baselineRaw.trim();
  const knownFields = ["Frequency", "Target", "Tags", "Relationships", "Notes", "Checkins"];
  const conflictWarnings = habit && dirty
    ? detectRawMarkdownConflicts({
        rawMarkdown: rawDraft,
        structuredMarkdown: renderHabitDraftMarkdown(draft),
        title: draft.name,
        requiredLabel: "Name",
        knownFields,
        enumFields: [{ label: "Frequency", options: HABIT_FREQUENCY_OPTIONS }],
      })
    : [];
  const resetDraft = () => {
    const nextDraft = habit ? habitToInput(habit) : emptyHabitInput();
    setDraft(nextDraft);
    setRawDraft(habit?.raw_markdown ?? renderHabitDraftMarkdown(nextDraft));
    setRawConflictChoice("structured");
  };
  const saveHabit = async () => {
    if (rawConflictChoice === "cancel") return;
    if (rawConflictChoice === "raw") {
      resetDraft();
      onCancel();
      return;
    }
    let input = rawConflictChoice === "convert" ? mergeHabitRawFields(draft, rawDraft) : draft;
    if (habit) {
      const preservedMarkdown = rawLinesNotInKnownFields(rawDraft, knownFields);
      input = { ...input, notes: appendPreservedMarkdown(input.notes, preservedMarkdown) };
    }
    if (rawConflictChoice === "note") {
      await onSaveAsNote(`${draft.name || habit?.name || t("widgets.labels.habit")} ${t("modules.drawer.rawCopy")}`, rawDraft);
    }
    onSave(habit?.habit_id ?? null, input);
  };

  return (
    <EntityEditDrawer
      conflictChoice={rawConflictChoice}
      conflictWarnings={conflictWarnings}
      description={t("habits.drawer.description")}
      dirty={dirty}
      onCancel={() => {
        resetDraft();
        onCancel();
      }}
      onConflictChoiceChange={setRawConflictChoice}
      onOpenChange={onOpenChange}
      onSave={saveHabit}
      open={open}
      saveDisabled={!draft.name.trim() || rawConflictChoice === "cancel"}
      saveLabel={rawConflictChoice === "raw" ? t("modules.drawer.choice.raw") : rawConflictChoice === "note" ? t("habits.saveNoteAndHabit") : habit ? t("habits.saveHabit") : t("habits.createHabit")}
      title={habit ? t("habits.editHabit") : t("habits.newHabit")}
    >
      {habit ? (
        <div className="grid gap-3 md:grid-cols-3">
          <MetricCard label={t("habits.metrics.currentStreak")} value={`${streak}`} />
          <MetricCard label={t("habits.metrics.checkins")} value={`${habit.checkins.length}`} />
          <MetricCard label={t("habits.metrics.today")} value={habit.checkins.includes(today) ? t("habits.done") : t("habits.open")} />
        </div>
      ) : null}
      <div className="grid gap-3 md:grid-cols-2">
        <TextField label={t("fields.name")} value={draft.name} onChange={(name) => setDraft((current) => ({ ...current, name }))} />
        <SelectField
          label={t("fields.frequency")}
          options={HABIT_FREQUENCY_OPTIONS}
          value={draft.frequency ?? ""}
          onChange={(frequency) => setDraft((current) => ({ ...current, frequency }))}
        />
        <TextField
          label={draft.frequency === "Custom" ? t("fields.customFrequency") : t("fields.target")}
          value={draft.target ?? ""}
          onChange={(target) => setDraft((current) => ({ ...current, target }))}
        />
        <TextField
          label={t("fields.tags")}
          value={listToInput(draft.tags)}
          onChange={(tags) => setDraft((current) => ({ ...current, tags: inputToList(tags) }))}
        />
      </div>
      <details className="rounded-md border border-border bg-muted/35 p-3 text-sm [&:not([open])>*:not(summary)]:hidden">
        <summary className="cursor-pointer font-medium">{t("modules.editor.advancedRelated")}</summary>
        <p className="mt-2 text-xs leading-5 text-muted-foreground">{t("fields.relatedEntitiesTooltip")}</p>
        <div className="mt-3">
          <TextField
            label={t("fields.relatedEntities")}
            value={listToInput(draft.relationships)}
            onChange={(relationships) => setDraft((current) => ({ ...current, relationships: inputToList(relationships) }))}
          />
        </div>
      </details>
      <TextAreaField label={t("fields.notes")} value={draft.notes ?? ""} onChange={(notes) => setDraft((current) => ({ ...current, notes }))} />
      {habits?.summary.recent_checkins.length ? (
        <div className="rounded-md bg-muted p-3 text-sm">
          <p className="font-medium">{t("habits.recentActivity")}</p>
          <ul className="mt-2 flex flex-col gap-1 text-muted-foreground">
            {habits.summary.recent_checkins.slice(0, 5).map((checkin) => (
              <li key={`${checkin.habit_id}-${checkin.date}`}>
                {checkin.name} - {checkin.date}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      <TextAreaField label={t("modules.editor.rawMarkdownSource")} value={rawDraft} onChange={setRawDraft} />
      <p className="text-xs leading-5 text-muted-foreground">
        {t("modules.drawer.rawEditHelp")}
      </p>
    </EntityEditDrawer>
  );
}

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-background p-3">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 truncate text-lg font-semibold">{value}</p>
    </div>
  );
}

function emptyContactInput(): ContactInput {
  return { name: "", relationship: "Other", organization: "", email: "", phone: "", tags: [], relationships: [], notes: "" };
}

function emptyHabitInput(): HabitInput {
  return { name: "", frequency: "Daily", target: "", tags: [], relationships: [], notes: "" };
}

function contactToInput(contact: ContactEntry): ContactInput {
  return {
    name: contact.name,
    relationship: contact.relationship ?? "Other",
    organization: contact.organization ?? "",
    email: contact.email ?? "",
    phone: contact.phone ?? "",
    tags: contact.tags,
    relationships: contact.relationships,
    notes: contact.notes ?? "",
  };
}

function habitToInput(habit: HabitEntry): HabitInput {
  return {
    name: habit.name,
    frequency: habit.frequency ?? "Daily",
    target: habit.target ?? "",
    tags: habit.tags,
    relationships: habit.relationships,
    notes: habit.notes ?? "",
  };
}

function contactInputEquals(left: ContactInput, right: ContactInput) {
  return renderContactDraftMarkdown(left).trim() === renderContactDraftMarkdown(right).trim();
}

function habitInputEquals(left: HabitInput, right: HabitInput) {
  return renderHabitDraftMarkdown(left).trim() === renderHabitDraftMarkdown(right).trim();
}

function renderContactDraftMarkdown(input: ContactInput) {
  const lines = [`# ${input.name || "Untitled Contact"}`, ""];
  if (input.relationship) lines.push(`- Relationship: ${input.relationship}`);
  if (input.organization) lines.push(`- Organization: ${input.organization}`);
  if (input.email) lines.push(`- Email: ${input.email}`);
  if (input.phone) lines.push(`- Phone: ${input.phone}`);
  if (input.tags?.length) lines.push(`- Tags: ${input.tags.join(", ")}`);
  if (input.relationships?.length) lines.push(`- Relationships: ${input.relationships.join(", ")}`);
  if (input.notes) lines.push("", input.notes);
  return `${lines.join("\n").trimEnd()}\n`;
}

function renderHabitDraftMarkdown(input: HabitInput) {
  const lines = [`# ${input.name || "Untitled Habit"}`, ""];
  if (input.frequency) lines.push(`- Frequency: ${input.frequency}`);
  if (input.target) lines.push(`- Target: ${input.target}`);
  if (input.tags?.length) lines.push(`- Tags: ${input.tags.join(", ")}`);
  if (input.relationships?.length) lines.push(`- Relationships: ${input.relationships.join(", ")}`);
  if (input.notes) lines.push("", input.notes);
  return `${lines.join("\n").trimEnd()}\n`;
}

function mergeContactRawFields(draft: ContactInput, markdown: string): ContactInput {
  const fields = parseRawFieldLines(markdown);
  return {
    ...draft,
    relationship: fields.get("relationship")?.[0] ?? draft.relationship,
    organization: fields.get("organization")?.[0] ?? fields.get("company")?.[0] ?? draft.organization,
    email: fields.get("email")?.[0] ?? draft.email,
    phone: fields.get("phone")?.[0] ?? draft.phone,
    tags: fields.get("tags")?.[0] ? inputToList(fields.get("tags")?.[0] ?? "") : draft.tags,
    relationships: fields.get("relationships")?.[0] ? inputToList(fields.get("relationships")?.[0] ?? "") : draft.relationships,
  };
}

function mergeHabitRawFields(draft: HabitInput, markdown: string): HabitInput {
  const fields = parseRawFieldLines(markdown);
  return {
    ...draft,
    frequency: fields.get("frequency")?.[0] ?? fields.get("repeat")?.[0] ?? draft.frequency,
    target: fields.get("target")?.[0] ?? draft.target,
    tags: fields.get("tags")?.[0] ? inputToList(fields.get("tags")?.[0] ?? "") : draft.tags,
    relationships: fields.get("relationships")?.[0] ? inputToList(fields.get("relationships")?.[0] ?? "") : draft.relationships,
  };
}

type VaultOnboardingProps = {
  action: "create" | "select" | "repair" | "reset" | null;
  onCreateDefault: () => void;
  onRepair: () => void;
  onReset: () => void;
  onSelectVault: () => void;
  snapshot: VaultSnapshot;
};

function VaultOnboarding({
  action,
  onCreateDefault,
  onRepair,
  onReset,
  onSelectVault,
  snapshot,
}: VaultOnboardingProps) {
  const { t } = useI18n();
  const checking = snapshot.stage === "checking";
  const needsRepair = snapshot.stage === "needs_repair";
  const needsResetGuidance = snapshot.stage === "needs_reset_guidance";
  const invalid = snapshot.stage === "invalid";
  const error = snapshot.stage === "error";
  const busy = action !== null || checking;

  return (
    <section className="flex min-h-[calc(100vh-2.5rem)] flex-col justify-center gap-5">
      <header className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border bg-card/78 px-4 py-3 shadow-soft backdrop-blur">
        <div className="flex min-w-0 items-center gap-3">
          <div className="flex size-10 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
            <Sparkles aria-hidden="true" />
          </div>
          <div className="min-w-0">
            <p className="truncate text-base font-semibold">BentoLife</p>
            <p className="truncate text-xs text-muted-foreground">{t("shell.localMarkdownVault")}</p>
          </div>
        </div>
        <Badge variant={needsRepair || needsResetGuidance || invalid || error ? "secondary" : "outline"}>
          {checking
            ? t("onboarding.checkingVault")
            : needsResetGuidance
              ? t("onboarding.backupNeeded")
              : needsRepair
                ? t("onboarding.repairAvailable")
                : invalid || error
                  ? t("architect.modules.needsAttention")
                  : t("onboarding.firstRun")}
        </Badge>
      </header>

      <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <Card className="min-w-0 bg-card/88">
          <CardHeader className="md:p-7">
            <div className="flex flex-wrap items-center gap-3">
              <Badge variant="status">{t("onboarding.localFirst")}</Badge>
              <Badge variant="secondary">Documents/.bentolifevault</Badge>
            </div>
            <CardTitle className="text-3xl md:text-4xl">
              {needsResetGuidance
                ? t("onboarding.backupTitle")
                : needsRepair
                  ? t("onboarding.repairTitle")
                  : t("onboarding.createTitle")}
            </CardTitle>
            <CardDescription>
              {needsResetGuidance
                ? t("onboarding.backupDescription")
                : t("onboarding.createDescription")}
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-5 md:p-7 md:pt-0">
            <div className="rounded-md bg-muted px-4 py-3 text-sm">
              <p className="font-medium">{t("onboarding.defaultLocation")}</p>
              <p className="mt-1 break-all text-muted-foreground">{snapshot.defaultPath || t("onboarding.resolvingDocuments")}</p>
            </div>

            {snapshot.inspection ? <VaultStatusBlock hideMissingPaths={!snapshot.inspection.exists} inspection={snapshot.inspection} /> : null}
            {needsResetGuidance ? (
              <RepairNotice
                title={t("onboarding.olderVault")}
                message={t("onboarding.olderVaultMessage")}
              />
            ) : null}
            {snapshot.error ? <RepairNotice title={t("onboarding.vaultActionFailed")} message={snapshot.error} /> : null}

            <div className="flex flex-wrap gap-3">
              {needsResetGuidance ? (
                <Button disabled={busy} onClick={onSelectVault}>
                  <FolderOpen data-icon="inline-start" />
                  {action === "select" ? t("onboarding.opening") : t("onboarding.chooseFreshVault")}
                </Button>
              ) : needsRepair ? (
                <Button disabled={busy} onClick={onRepair}>
                  <RefreshCw data-icon="inline-start" />
                  {action === "repair" ? t("onboarding.repairing") : t("onboarding.repairVaultStructure")}
                </Button>
              ) : (
                <Button disabled={busy} onClick={onCreateDefault}>
                  <FolderPlus data-icon="inline-start" />
                  {action === "create" ? t("onboarding.creating") : t("onboarding.createVault")}
                </Button>
              )}
              <Button disabled={busy} onClick={onSelectVault} variant="outline">
                <FolderOpen data-icon="inline-start" />
                {action === "select" ? t("onboarding.opening") : t("onboarding.selectExistingVault")}
              </Button>
              {invalid || error || needsResetGuidance ? (
                <Button disabled={busy} onClick={onReset} variant="ghost">
                  <RefreshCw data-icon="inline-start" />
                  {t("onboarding.startOver")}
                </Button>
              ) : null}
            </div>
          </CardContent>
        </Card>

        <aside className="flex min-w-0 flex-col gap-5">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <CheckCircle2 aria-hidden="true" data-icon="inline-start" />
                {t("onboarding.whatSetupCreates")}
              </CardTitle>
              <CardDescription>{t("onboarding.description")}</CardDescription>
            </CardHeader>
            <CardContent className="flex flex-col gap-2 text-sm">
              {[
                ".bentolifevault/",
                "assets/",
                ".bentolifelayout/",
                "modules/<module>/data/",
                "MODULE.md + module.schema.json",
                "Trash and Archive system modules",
              ].map((item) => (
                <div className="rounded-md bg-muted px-3 py-2" key={item}>
                  {item}
                </div>
              ))}
            </CardContent>
          </Card>

          <Empty
            className="min-h-0"
            title={t("onboarding.hiddenMetadataTitle")}
            description={t("onboarding.hiddenMetadataDescription")}
          />
        </aside>
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

function moduleView(moduleId: string): AppView {
  return viewForModule(moduleId);
}

function moduleDocumentType(moduleId: string) {
  switch (moduleId) {
    case "todos":
      return "todos";
    case "contacts":
      return "contact";
    case "habits":
      return "habit";
    case "navigator":
      return "navigator";
    default:
      return "note";
  }
}

function entityView(entityType: string): AppView {
  switch (entityType) {
    case "todos":
      return "todos";
    case "contact":
      return "contacts";
    case "habit":
      return "habits";
    case "navigator":
      return "navigator";
    default:
      return "notes";
  }
}

function viewToModuleId(view: AppView): string | null {
  switch (view) {
    case "notes":
      return "notes";
    case "todos":
      return "todos";
    case "contacts":
      return "contacts";
    case "habits":
      return "habits";
    case "navigator":
      return "navigator";
    case "trash":
      return "trash";
    case "archive":
      return "archive";
    default:
      return null;
  }
}

function architectTabFromLabel(label?: string): ArchitectTabId | undefined {
  switch (label) {
    case "Dashboard":
    case "Dashboard Customization":
      return "dashboard";
    case "Appearance":
    case "Theme Registry":
      return "appearance";
    case "Schemas":
      return "schemas";
    case "Data & Graph":
    case "Search graph":
    case "Search local graph":
      return "data_graph";
    case "Recovery":
      return "recovery";
    case "Modules":
      return "modules";
    default:
      return undefined;
  }
}

function isArchitectTabId(value: unknown): value is ArchitectTabId {
  return value === "modules" || value === "dashboard" || value === "appearance" || value === "schemas" || value === "data_graph" || value === "recovery";
}

function tokensToStyle(tokens: ThemeTokenMap): CSSProperties {
  return Object.fromEntries(Object.entries(tokens)) as CSSProperties;
}

function todayKey() {
  const now = new Date();
  const month = `${now.getMonth() + 1}`.padStart(2, "0");
  const day = `${now.getDate()}`.padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

function setTodoCompletion(markdownBody: string, completed: boolean) {
  const lines = markdownBody.trimEnd().split(/\r?\n/);
  const nextLines = [...lines];
  const statusIndex = nextLines.findIndex((line) => /^Status:\s*/i.test(line));
  const nextStatus = `Status: ${completed ? "Done" : "Not started"}`;
  if (statusIndex >= 0) {
    nextLines[statusIndex] = nextStatus;
  } else {
    const titleIndex = nextLines.findIndex((line) => line.startsWith("# "));
    nextLines.splice(titleIndex >= 0 ? titleIndex + 1 : 0, 0, nextStatus);
  }
  return `${nextLines.join("\n")}\n`;
}

function formatTime(date: Date) {
  return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function isDashboardLayoutBlocked(error: unknown) {
  const message = getErrorMessage(error).toLowerCase();
  return message.includes("overlaps another widget") || message.includes("compact layout");
}

function backlinksForTarget(navigator: NavigatorSnapshot | null, documentId?: string | null, markdownPath?: string | null): ModuleBacklink[] {
  if (!navigator || (!documentId && !markdownPath)) {
    return [];
  }
  const normalizedPath = markdownPath?.replace(/\\/g, "/") ?? null;
  return navigator.backlinks.filter((link) => {
    if (documentId && link.resolved_document_id === documentId) {
      return true;
    }
    if (normalizedPath && link.resolved_path?.replace(/\\/g, "/") === normalizedPath) {
      return true;
    }
    return Boolean(normalizedPath && link.target.replace(/\\/g, "/") === normalizedPath);
  });
}

function logV5Timing(label: string, startedAt: number, details: Record<string, number>) {
  if (typeof console === "undefined") {
    return;
  }
  console.info("[BentoLife V5 timing]", {
    ...details,
    duration_ms: Math.round(performance.now() - startedAt),
    label,
  });
}

export default App;
