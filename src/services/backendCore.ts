import { invoke } from "@tauri-apps/api/core";
import {
  emptyNavigatorSnapshot,
  emptyRegistryState,
  emptySearchIndexSnapshot,
  normalizeNavigatorRebuildReport,
  normalizeNavigatorSnapshot,
  normalizeRegistryState,
  normalizeSearchIndexSnapshot,
} from "@/services/contracts/backendCore.contract";
import { safeInvoke } from "@/services/contracts/invokeClient";
import type {
  ArchiveResult,
  BulkImportAcceptReport,
  CoreCacheSnapshot,
  EntityUpgradePreview,
  EntityUpgradeReport,
  ExternalSourceFile,
  FileLifecycleEntry,
  FileLifecycleMutationReport,
  FolderImportManifest,
  FolderImportPreview,
  IgnoredStagedImportReport,
  ImportAcceptPreview,
  ImportAcceptReport,
  ImportAcceptanceOptions,
  NavigatorSnapshot,
  SearchIndexSnapshot,
  SnapshotRestorePreview,
  SnapshotRestoreReport,
  SnapshotStageReport,
  StagedImportIndex,
  StagedImportRecord,
  TrashResult,
  VaultSnapshotManifest,
  VaultSnapshotPreview,
} from "../types/bentolife-core";
import type { WorkspaceScanResult } from "@/services/notes";

export type {
  ArchiveResult,
  BulkImportAcceptReport,
  CoreCacheSnapshot,
  EntityUpgradePreview,
  EntityUpgradeReport,
  ExternalSourceFile,
  FileLifecycleEntry,
  FileLifecycleMutationReport,
  FolderImportManifest,
  FolderImportPreview,
  IgnoredStagedImportReport,
  ImportAcceptPreview,
  ImportAcceptReport,
  ImportAcceptanceOptions,
  NavigatorSnapshot,
  SearchIndexSnapshot,
  SnapshotRestorePreview,
  SnapshotRestoreReport,
  SnapshotStageReport,
  StagedImportIndex,
  StagedImportRecord,
  TrashResult,
  VaultSnapshotManifest,
  VaultSnapshotPreview,
};

export type NavigatorRebuildReport = {
  scan: WorkspaceScanResult;
  navigator: NavigatorSnapshot;
};

const mockTrashEntriesStorageKey = "bentolife:mockTrashEntries";
const mockArchiveEntriesStorageKey = "bentolife:mockArchiveEntries";

function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

function trashConfirmationToken(relativePath: string) {
  return `trash:${normalizeVaultRelativePath(relativePath)}`;
}

function archiveConfirmationToken(relativePath: string) {
  return `archive:${normalizeVaultRelativePath(relativePath)}`;
}

function deleteTrashConfirmationToken(entryId: string) {
  return `permanently-delete-trash-entry:${normalizeVaultRelativePath(entryId)}`;
}

function emptyTrashConfirmationToken() {
  return "empty-trash";
}

function restoreSnapshotConfirmationToken() {
  return "restore-vault-snapshot";
}

function applyEntityUpgradeConfirmationToken() {
  return "apply-entity-upgrade";
}

function normalizeVaultRelativePath(path: string) {
  return path.trim().replace(/\\/g, "/");
}

export async function previewFolderImport(sourcePath: string, vaultPath: string): Promise<FolderImportPreview> {
  if (!isTauriRuntime()) {
    return mockFolderImportPreview(sourcePath, vaultPath);
  }
  return invoke<FolderImportPreview>("preview_folder_import", { sourcePath, vaultPath });
}

export async function importFolderIntoVault(sourcePath: string, vaultPath: string): Promise<FolderImportManifest> {
  if (!isTauriRuntime()) {
    const preview = mockFolderImportPreview(sourcePath, vaultPath);
    const manifest = {
      schema_version: 1,
      source_path: sourcePath,
      vault_path: vaultPath,
      target_root: preview.target_root,
      imported_at: new Date().toISOString(),
      files: preview.planned_files.map((file) => ({ ...file, copied: !file.skipped })),
      conflicts: preview.conflicts,
      warnings: preview.warnings,
    };
    mergeMockStagedImports(
      vaultPath,
      manifest.files
        .filter((file) => file.copied && shouldShowInMockImportReview(file.source_relative_path, preview.scan.source_kind))
        .map((file) => mockRecordFromExternalFile(file, preview.scan.source_kind)),
    );
    return manifest;
  }
  return invoke<FolderImportManifest>("import_folder_into_vault", { sourcePath, vaultPath });
}

export async function previewVaultSnapshot(
  sourceVaultPath: string,
  snapshotPath: string,
): Promise<VaultSnapshotPreview> {
  if (!isTauriRuntime()) {
    return mockVaultSnapshotPreview(sourceVaultPath, snapshotPath);
  }
  return invoke<VaultSnapshotPreview>("preview_vault_snapshot", { sourceVaultPath, snapshotPath });
}

export async function createVaultSnapshot(
  sourceVaultPath: string,
  snapshotPath: string,
): Promise<VaultSnapshotManifest> {
  if (!isTauriRuntime()) {
    mockVaultSnapshotPreview(sourceVaultPath, snapshotPath);
    return {
      schema_version: 1,
      source_vault_path: sourceVaultPath,
      snapshot_path: snapshotPath,
      created_at: new Date().toISOString(),
      source_machine: "browser-fallback",
      files: [
        { relative_path: "INDEX.md", file_kind: "markdown", content_hash: "mock-index", byte_len: 120 },
        { relative_path: "modules/notes/data/daily.md", file_kind: "markdown", content_hash: "mock-note", byte_len: 240 },
        { relative_path: ".bentolifelayout/index.json", file_kind: "metadata", content_hash: "mock-cache", byte_len: 320 },
      ],
      warnings: ["Browser fallback created an in-memory snapshot manifest for UI verification."],
    };
  }
  return invoke<VaultSnapshotManifest>("create_vault_snapshot", { sourceVaultPath, snapshotPath });
}

export async function previewSnapshotRestore(
  snapshotPath: string,
  targetVaultPath: string,
): Promise<SnapshotRestorePreview> {
  if (!isTauriRuntime()) {
    return mockSnapshotRestorePreview(snapshotPath, targetVaultPath);
  }
  return invoke<SnapshotRestorePreview>("preview_snapshot_restore", { snapshotPath, targetVaultPath });
}

export async function restoreVaultSnapshot(
  snapshotPath: string,
  targetVaultPath: string,
): Promise<SnapshotRestoreReport> {
  if (!isTauriRuntime()) {
    const preview = mockSnapshotRestorePreview(snapshotPath, targetVaultPath);
    if (!preview.direct_restore_allowed) {
      throw new Error(preview.blocked_reason ?? "Snapshot must be staged for review before restore.");
    }
    return {
      snapshot_path: snapshotPath,
      target_vault_path: targetVaultPath,
      restored_at: new Date().toISOString(),
      restored_files: [
        { relative_path: "INDEX.md", file_kind: "markdown", content_hash: "mock-index", byte_len: 120 },
        { relative_path: "modules/notes/data/daily.md", file_kind: "markdown", content_hash: "mock-note", byte_len: 240 },
      ],
      conflicts: preview.conflicts,
      warnings: preview.warnings,
      cache: mockCoreCache(targetVaultPath),
    };
  }
  return invoke<SnapshotRestoreReport>("restore_vault_snapshot", {
    snapshotPath,
    targetVaultPath,
    confirmationToken: restoreSnapshotConfirmationToken(),
  });
}

export async function stageSnapshotForImport(
  snapshotPath: string,
  targetVaultPath: string,
): Promise<SnapshotStageReport> {
  if (!isTauriRuntime()) {
    return mockStageSnapshotForImport(snapshotPath, targetVaultPath);
  }
  return invoke<SnapshotStageReport>("stage_snapshot_for_import", { snapshotPath, targetVaultPath });
}

export async function listStagedImports(vaultPath: string): Promise<StagedImportIndex> {
  if (!isTauriRuntime()) {
    return mockListStagedImports(vaultPath);
  }
  return invoke<StagedImportIndex>("list_staged_imports", { vaultPath });
}

export async function previewAcceptImport(
  vaultPath: string,
  stagedFilePath: string,
  targetModule: string,
  options: ImportAcceptanceOptions,
): Promise<ImportAcceptPreview> {
  if (!isTauriRuntime()) {
    return mockPreviewAcceptImport(vaultPath, stagedFilePath, targetModule, options);
  }
  return invoke<ImportAcceptPreview>("preview_accept_import", { vaultPath, stagedFilePath, targetModule, options });
}

export async function acceptImportIntoModule(
  vaultPath: string,
  stagedFilePath: string,
  targetModule: string,
  options: ImportAcceptanceOptions,
): Promise<ImportAcceptReport> {
  if (!isTauriRuntime()) {
    return mockAcceptImportIntoModule(vaultPath, stagedFilePath, targetModule, options);
  }
  return invoke<ImportAcceptReport>("accept_import_into_module", { vaultPath, stagedFilePath, targetModule, options });
}

export async function ignoreStagedImport(
  vaultPath: string,
  stagedFilePath: string,
): Promise<IgnoredStagedImportReport> {
  if (!isTauriRuntime()) {
    return mockIgnoreStagedImport(vaultPath, stagedFilePath);
  }
  return invoke<IgnoredStagedImportReport>("ignore_staged_import", { vaultPath, stagedFilePath });
}

export async function bulkAcceptImports(
  vaultPath: string,
  selectedFiles: string[],
  targetModule: string,
  options: ImportAcceptanceOptions,
): Promise<BulkImportAcceptReport> {
  if (!isTauriRuntime()) {
    const accepted: ImportAcceptReport[] = [];
    const errors: string[] = [];
    for (const stagedFile of selectedFiles) {
      try {
        accepted.push(await mockAcceptImportIntoModule(vaultPath, stagedFile, targetModule, options));
      } catch (error) {
        errors.push(`${stagedFile}: ${error instanceof Error ? error.message : String(error)}`);
      }
    }
    return { vault_path: vaultPath, accepted, errors };
  }
  return invoke<BulkImportAcceptReport>("bulk_accept_imports", { vaultPath, selectedFiles, targetModule, options });
}

export async function rebuildCoreCache(vaultPath: string): Promise<CoreCacheSnapshot> {
  if (!isTauriRuntime()) {
    return mockCoreCache(vaultPath);
  }
  return invoke<CoreCacheSnapshot>("rebuild_core_cache", { vaultPath });
}

export async function previewEntityUpgrade(vaultPath: string): Promise<EntityUpgradePreview> {
  if (!isTauriRuntime()) {
    return {
      schema_version: 1,
      vault_path: vaultPath,
      changes: [],
      legacy_paths: [],
      warnings: ["Browser fallback has no legacy filesystem paths to upgrade."],
    };
  }
  return invoke<EntityUpgradePreview>("preview_entity_upgrade", { vaultPath });
}

export async function applyEntityUpgrade(vaultPath: string): Promise<EntityUpgradeReport> {
  if (!isTauriRuntime()) {
    return {
      schema_version: 1,
      vault_path: vaultPath,
      upgraded_at: new Date().toISOString(),
      manifest_path: ".bentolifelayout/imports/entity-upgrades/mock.json",
      backup_root: ".bentolifelayout/trash/entity-upgrades/mock",
      changes: [],
      trashed_legacy_paths: [],
      cache: mockCoreCache(vaultPath),
    };
  }
  return invoke<EntityUpgradeReport>("apply_entity_upgrade", {
    vaultPath,
    confirmationToken: applyEntityUpgradeConfirmationToken(),
  });
}

export async function readNavigator(vaultPath: string): Promise<NavigatorSnapshot> {
  if (!isTauriRuntime()) {
    return mockNavigator(vaultPath);
  }
  const result = await safeInvoke("read_navigator", { vaultPath }, normalizeNavigatorSnapshot, emptyNavigatorSnapshot(vaultPath));
  return result.data;
}

export async function rebuildNavigator(vaultPath: string): Promise<NavigatorSnapshot> {
  if (!isTauriRuntime()) {
    return mockNavigator(vaultPath);
  }
  return normalizeNavigatorSnapshot(await invoke<unknown>("rebuild_navigator", { vaultPath }));
}

export async function scanAndRebuildNavigator(vaultPath: string): Promise<NavigatorRebuildReport> {
  if (!isTauriRuntime()) {
    return {
      scan: mockWorkspaceScan(vaultPath),
      navigator: mockNavigator(vaultPath),
    };
  }
  return normalizeNavigatorRebuildReport(await invoke<unknown>("scan_and_rebuild_navigator", { vaultPath }));
}

export async function rebuildSearchIndex(vaultPath: string): Promise<SearchIndexSnapshot> {
  if (!isTauriRuntime()) {
    return mockSearch(vaultPath, "");
  }
  return normalizeSearchIndexSnapshot(await invoke<unknown>("rebuild_search_index", { vaultPath }));
}

export async function searchEntities(vaultPath: string, query: string): Promise<SearchIndexSnapshot> {
  if (!isTauriRuntime()) {
    return mockSearch(vaultPath, query);
  }
  const result = await safeInvoke("search_entities", { vaultPath, query }, normalizeSearchIndexSnapshot, emptySearchIndexSnapshot(vaultPath));
  return result.data;
}

export async function trashManagedEntity(vaultPath: string, relativePath: string): Promise<TrashResult> {
  if (!isTauriRuntime()) {
    const result = mockTrashResult(vaultPath, relativePath, `.bentolifelayout/trash/files/${relativePath}`);
    addMockLifecycleEntry(mockTrashEntriesStorageKey, vaultPath, trashLifecycleEntry(result));
    return result;
  }
  return invoke<TrashResult>("trash_managed_entity", {
    vaultPath,
    relativePath,
    confirmationToken: trashConfirmationToken(relativePath),
  });
}

export async function archiveManagedEntity(vaultPath: string, relativePath: string): Promise<ArchiveResult> {
  if (!isTauriRuntime()) {
    const result = mockArchiveResult(vaultPath, relativePath, `.bentolifelayout/archive/files/${relativePath}`);
    addMockLifecycleEntry(mockArchiveEntriesStorageKey, vaultPath, archiveLifecycleEntry(result));
    return result;
  }
  return invoke<ArchiveResult>("archive_managed_entity", {
    vaultPath,
    relativePath,
    confirmationToken: archiveConfirmationToken(relativePath),
  });
}

export async function listTrashEntries(vaultPath: string): Promise<FileLifecycleEntry[]> {
  if (!isTauriRuntime()) {
    return readMockLifecycleEntries(mockTrashEntriesStorageKey, vaultPath);
  }
  return invoke<FileLifecycleEntry[]>("list_trash_entries", { vaultPath });
}

export async function listArchiveEntries(vaultPath: string): Promise<FileLifecycleEntry[]> {
  if (!isTauriRuntime()) {
    return readMockLifecycleEntries(mockArchiveEntriesStorageKey, vaultPath);
  }
  return invoke<FileLifecycleEntry[]>("list_archive_entries", { vaultPath });
}

function mockCoreCache(vaultPath: string): CoreCacheSnapshot {
  return {
    schema_version: 1,
    vault_path: vaultPath,
    entities_by_path: {
      "modules/notes/data/daily-note.md": {
        document_id: "bl_doc_mock_daily_note",
        current_path: "modules/notes/data/daily-note.md",
        title: "Daily Note",
        entity_type: "note",
        metadata_path: ".bentolifelayout/documents/bl_doc_mock_daily_note.json",
        content_hash: "mock",
        tags: ["daily"],
        relationships: [],
        backlinks: [
          {
            source_path: "modules/notes/data/daily-note.md",
            target: "Launch Review",
            link_type: "todos",
            raw: "[[Todos:Launch Review]]",
            status: "broken",
            resolved_document_id: null,
            resolved_path: null,
          },
        ],
        unresolved_links: [],
      },
    },
    graph_links: [
      {
        source_path: "modules/notes/data/daily-note.md",
        target: "Launch Review",
        link_type: "todos",
        raw: "[[Todos:Launch Review]]",
        status: "broken",
        resolved_document_id: null,
        resolved_path: null,
      },
    ],
    health_warnings: [
      {
        code: "broken_link",
        message: "modules/notes/data/daily-note.md references unresolved entity [[Todos:Launch Review]].",
        document_id: null,
        path: "modules/notes/data/daily-note.md",
      },
    ],
    index: {
      schema_version: 1,
      path_policy: "vault_relative",
      documents_by_id: {},
      document_ids_by_path: {},
      orphaned_document_ids: [],
      duplicate_identity_conflicts: [],
      updated_at: "mock",
      rebuild_policy: {
        rebuild_from_documents_folder: true,
        rebuild_from_markdown_uuid_comments: true,
        treat_index_as_cache: true,
      },
    },
    updated_at: "mock",
    invalidation_reason: null,
  };
}

function mockSearch(vaultPath: string, query: string): SearchIndexSnapshot {
  const entries = [
    {
      path: "modules/notes/data/daily-note.md",
      document_id: "bl_doc_mock_daily_note",
      title: "Daily Note",
      entity_type: "note",
      tags: ["daily"],
      headings: ["Daily Note"],
      excerpt: "Plan the day and link future tasks.",
      searchable_text: "daily note daily plan the day and link future tasks",
    },
  ].filter((entry) => !query.trim() || entry.searchable_text.includes(query.toLowerCase()));
  return {
    schema_version: 1,
    vault_path: vaultPath,
    index_path: ".bentolifelayout/indexes/search.json",
    entries,
    updated_at: "mock",
    source_cache_updated_at: "mock",
  };
}

function mockNavigator(vaultPath: string): NavigatorSnapshot {
  const cache = mockCoreCache(vaultPath);
  return {
    schema_version: 1,
    vault_path: vaultPath,
    navigator_path: "modules/navigator/NAVIGATOR.md",
    index_path: "modules/navigator/INDEX.md",
    markdown: "# Navigator\n\nGraph health is shown in managed blocks.\n",
    module_summaries: [{ module_id: "note", entity_count: 1, index_path: "modules/notes/INDEX.md" }],
    health_warnings: cache.health_warnings,
    managed_block_warnings: [],
    backlinks: cache.graph_links,
    search_index_path: ".bentolifelayout/indexes/search.json",
    updated_at: "mock",
  };
}

function mockWorkspaceScan(vaultPath: string): WorkspaceScanResult {
  return {
    vault_path: vaultPath,
    documents: [
      {
        document_id: "bl_doc_mock_daily_note",
        title: "Daily Note",
        markdown_relative_path: "modules/notes/data/daily-note.md",
        metadata_path: ".bentolifelayout/documents/bl_doc_mock_daily_note.json",
        layout_path: ".bentolifelayout/layouts/bl_doc_mock_daily_note.layout.json",
        document_type: "note",
        status: "managed",
        markdown: "# Daily Note\n\nPlan the day and link future tasks.\n",
        markdown_body: "# Daily Note\n\nPlan the day and link future tasks.\n",
        layout_metadata: null,
        stale_layout_references: [],
      },
    ],
    issues: [],
  };
}

export async function restoreTrashedEntity(
  vaultPath: string,
  trashRelativePath: string,
  restoreRelativePath: string,
): Promise<TrashResult> {
  if (!isTauriRuntime()) {
    return mockTrashResult(vaultPath, restoreRelativePath, trashRelativePath);
  }
  return invoke<TrashResult>("restore_trashed_entity", { vaultPath, trashRelativePath, restoreRelativePath });
}

export async function restoreArchivedEntity(
  vaultPath: string,
  archiveRelativePath: string,
  restoreRelativePath: string,
): Promise<ArchiveResult> {
  if (!isTauriRuntime()) {
    return mockArchiveResult(vaultPath, restoreRelativePath, archiveRelativePath);
  }
  return invoke<ArchiveResult>("restore_archived_entity", { vaultPath, archiveRelativePath, restoreRelativePath });
}

export async function restoreTrashEntry(vaultPath: string, entryId: string): Promise<TrashResult> {
  if (!isTauriRuntime()) {
    const entry = removeMockLifecycleEntry(mockTrashEntriesStorageKey, vaultPath, entryId);
    return mockTrashResult(vaultPath, entry.original_path, entry.current_path);
  }
  return invoke<TrashResult>("restore_trash_entry", { vaultPath, entryId });
}

export async function restoreArchiveEntry(vaultPath: string, entryId: string): Promise<ArchiveResult> {
  if (!isTauriRuntime()) {
    const entry = removeMockLifecycleEntry(mockArchiveEntriesStorageKey, vaultPath, entryId);
    return mockArchiveResult(vaultPath, entry.original_path, entry.current_path);
  }
  return invoke<ArchiveResult>("restore_archive_entry", { vaultPath, entryId });
}

export async function deleteTrashEntryPermanently(vaultPath: string, entryId: string): Promise<FileLifecycleMutationReport> {
  if (!isTauriRuntime()) {
    removeMockLifecycleEntry(mockTrashEntriesStorageKey, vaultPath, entryId);
    const entries = await listTrashEntries(vaultPath);
    return {
      action: "delete_trash_entry_permanently",
      changed_count: 1,
      entries,
      message: `Permanently deleted Trash entry ${entryId}.`,
    };
  }
  return invoke<FileLifecycleMutationReport>("delete_trash_entry_permanently", {
    vaultPath,
    entryId,
    confirmationToken: deleteTrashConfirmationToken(entryId),
  });
}

export async function emptyTrash(vaultPath: string): Promise<FileLifecycleMutationReport> {
  if (!isTauriRuntime()) {
    const changedCount = readMockLifecycleEntries(mockTrashEntriesStorageKey, vaultPath).length;
    writeMockLifecycleEntries(mockTrashEntriesStorageKey, vaultPath, []);
    return {
      action: "empty_trash",
      changed_count: changedCount,
      entries: [],
      message: "Trash was emptied permanently.",
    };
  }
  return invoke<FileLifecycleMutationReport>("empty_trash", {
    vaultPath,
    confirmationToken: emptyTrashConfirmationToken(),
  });
}

function mockFolderImportPreview(sourcePath: string, vaultPath: string): FolderImportPreview {
  const trimmedSource = sourcePath.trim();
  if (!trimmedSource) {
    throw new Error("Import source folder is required.");
  }
  if (trimmedSource.toLowerCase().includes("missing")) {
    throw new Error("Import source folder was not found.");
  }

  const normalizedSource = trimmedSource.toLowerCase();
  const obsidianSource = normalizedSource.includes("obsidian") || normalizedSource.includes("second brain");
  const bentolifeSource = normalizedSource.includes(".bentolifevault") || normalizedSource.includes("bentolife");
  const conflictSource = normalizedSource.includes("conflict");
  const sourceKind = bentolifeSource ? "bentolife_vault" : obsidianSource ? "obsidian_markdown_folder" : "markdown_folder";
  const bentolifeModule = normalizedSource.includes("todos")
    ? "todos"
    : normalizedSource.includes("contacts")
      ? "contacts"
      : normalizedSource.includes("habits")
        ? "habits"
        : "notes";
  const bentolifeTitle = bentolifeModule === "todos"
    ? "Task Inbox"
    : bentolifeModule === "contacts"
      ? "Mina Park"
      : bentolifeModule === "habits"
        ? "Read Daily"
        : "Daily";
  const targetRoot = bentolifeSource
    ? ".bentolifelayout/imports/staged/bentolife-vault"
    : obsidianSource
    ? ".bentolifelayout/imports/staged/obsidian"
    : ".bentolifelayout/imports/staged/folder";
  const files: ExternalSourceFile[] = bentolifeSource ? [
    {
      source_relative_path: `modules/${bentolifeModule}/data/${bentolifeTitle}.md`,
      target_relative_path: `${targetRoot}/modules/${bentolifeModule}/data/${bentolifeTitle}.md`,
      file_kind: "markdown",
      document_id: "bl_doc_mock_import_daily",
      title: bentolifeTitle,
      content_hash: "mock-daily",
      copied: false,
      collision_renamed: false,
      skipped: false,
      reason: null,
    },
  ] : [
    {
      source_relative_path: "Daily.md",
      target_relative_path: `${targetRoot}/Daily.md`,
      file_kind: "markdown",
      document_id: "bl_doc_mock_import_daily",
      title: "Daily",
      content_hash: "mock-daily",
      copied: false,
      collision_renamed: false,
      skipped: false,
      reason: null,
    },
    {
      source_relative_path: "assets/banner.png",
      target_relative_path: `${targetRoot}/assets/banner.png`,
      file_kind: "asset",
      document_id: null,
      title: null,
      content_hash: "mock-asset",
      copied: false,
      collision_renamed: false,
      skipped: false,
      reason: null,
    },
  ];
  if (obsidianSource) {
    files.push({
      source_relative_path: "Project.md",
      target_relative_path: `${targetRoot}/Project.md`,
      file_kind: "markdown",
      document_id: null,
      title: "Project",
      content_hash: "mock-project",
      copied: false,
      collision_renamed: false,
      skipped: false,
      reason: null,
    });
  }
  if (conflictSource) {
    files[0] = { ...files[0], target_relative_path: `${targetRoot}/Daily-2.md`, collision_renamed: true };
  }

  return {
    source_path: trimmedSource,
    vault_path: vaultPath,
    scan: {
      source_path: trimmedSource,
      source_kind: sourceKind,
      markdown_count: files.filter((file) => file.file_kind === "markdown").length,
      asset_count: files.filter((file) => file.file_kind === "asset").length,
      ignored_count: bentolifeSource ? 4 : obsidianSource ? 1 : 0,
      files,
      warnings: bentolifeSource ? ["BentoLife runtime files were hidden during preview."] : obsidianSource ? [".obsidian/ was ignored during preview."] : [],
    },
    target_root: targetRoot,
    planned_files: files,
    conflicts: conflictSource ? ["Daily.md target existed and will be renamed."] : [],
    warnings: ["Browser fallback preview; desktop uses copy-only Tauri commands."],
  };
}

function mockVaultSnapshotPreview(sourceVaultPath: string, snapshotPath: string): VaultSnapshotPreview {
  if (!sourceVaultPath.trim() || !snapshotPath.trim()) {
    throw new Error("Source vault path and snapshot folder are required.");
  }
  if (sourceVaultPath.toLowerCase().includes("missing")) {
    throw new Error("Source vault was not found.");
  }
  return {
    source_vault_path: sourceVaultPath,
    snapshot_path: snapshotPath,
    file_count: 3,
    total_bytes: 680,
    warnings: ["Browser fallback preview; desktop creates a portable snapshot folder."],
  };
}

const mockStagedImportsPrefix = "bentolife:mockStagedImports:";
const mockNotesStorageKey = "bentolife:mockNotes";
const mockTodosStorageKey = "bentolife:mockTodos";
const mockContactsStorageKey = "bentolife:mockContacts";
const mockHabitsStorageKey = "bentolife:mockHabits";
const mockModuleRegistryStorageKey = "bentolife:mockModuleRegistry";

function stagedImportsKey(vaultPath: string) {
  return `${mockStagedImportsPrefix}${vaultPath}`;
}

function mockListStagedImports(vaultPath: string): StagedImportIndex {
  try {
    const records = JSON.parse(window.localStorage.getItem(stagedImportsKey(vaultPath)) ?? "[]") as StagedImportRecord[];
    return {
      schema_version: 1,
      vault_path: vaultPath,
      updated_at: "mock",
      records,
      hidden_system_count: 0,
      hidden_system_files: [],
      warnings: [],
    };
  } catch {
    return {
      schema_version: 1,
      vault_path: vaultPath,
      updated_at: "mock",
      records: [],
      hidden_system_count: 0,
      hidden_system_files: [],
      warnings: [],
    };
  }
}

function writeMockStagedImports(vaultPath: string, records: StagedImportRecord[]) {
  window.localStorage.setItem(stagedImportsKey(vaultPath), JSON.stringify(records));
}

function mergeMockStagedImports(vaultPath: string, records: StagedImportRecord[]) {
  const index = mockListStagedImports(vaultPath);
  const next = [...index.records];
  for (const record of records) {
    const existing = next.findIndex((candidate) => candidate.staged_file_path === record.staged_file_path);
    if (existing >= 0) {
      next[existing] = record;
    } else {
      next.push(record);
    }
  }
  next.sort((left, right) => left.staged_file_path.localeCompare(right.staged_file_path));
  writeMockStagedImports(vaultPath, next);
}

function mockRecordFromExternalFile(file: ExternalSourceFile, sourceKind: string): StagedImportRecord {
  const suggestedModule = mockSuggestedImportModule(file);
  return {
    staged_file_path: file.target_relative_path,
    original_source_path: file.source_relative_path,
    source_kind: sourceKind,
    detected_title: file.title ?? titleFromImportPath(file.source_relative_path),
    detected_tags: file.source_relative_path.toLowerCase().includes("project") ? ["project"] : [],
    detected_links: [],
    detected_checklists: file.source_relative_path.toLowerCase().includes("daily") ? 2 : 0,
    suggested_module: suggestedModule,
    accepted: false,
    ignored: false,
    conflict_status: file.file_kind === "markdown" ? null : "Assets stay staged and are not accepted directly into modules.",
  };
}

function mockSuggestedImportModule(file: ExternalSourceFile) {
  const source = file.source_relative_path.replace(/\\/g, "/").toLowerCase();
  for (const moduleId of ["notes", "todos", "contacts", "habits"]) {
    if (source.startsWith(`modules/${moduleId}/data/`) || source.includes(`/modules/${moduleId}/data/`)) {
      return moduleId;
    }
  }
  const text = `${file.source_relative_path} ${file.title ?? ""}`.toLowerCase();
  if (/status:|priority:|due date:|due:|tasks?|todos?|checklist|inbox|daily/.test(text)) return "todos";
  if (/email:|phone:|relationship:|people|person|contacts?|clients?|vendors?/.test(text)) return "contacts";
  if (/frequency:|check-in|streak|habits?|routines?/.test(text)) return "habits";
  return "notes";
}

function shouldShowInMockImportReview(relativePath: string, sourceKind: string) {
  const normalized = relativePath.trim().replace(/\\/g, "/");
  if (
    !normalized ||
    normalized.startsWith("/") ||
    normalized.includes("..") ||
    normalized.includes(".bentolifelayout/") ||
    normalized === "INDEX.md" ||
    normalized.startsWith("schemas/") ||
    normalized.startsWith(".git/") ||
    normalized.startsWith(".obsidian/") ||
    normalized.startsWith("node_modules/") ||
    normalized === ".DS_Store" ||
    normalized === "Thumbs.db" ||
    /^modules\/[^/]+\/(?:INDEX|MODULE)\.md$/.test(normalized) ||
    normalized.startsWith("modules/navigator/") ||
    normalized.startsWith("modules/trash/") ||
    normalized.startsWith("modules/archive/")
  ) {
    return false;
  }
  if (sourceKind === "bentolife_vault" || sourceKind === "snapshot") {
    return /^modules\/(?:notes|todos|contacts|habits)\/data\/.+\.md$/.test(normalized);
  }
  return normalized.endsWith(".md");
}

function mockStageSnapshotForImport(snapshotPath: string, targetVaultPath: string): SnapshotStageReport {
  const preview = mockSnapshotRestorePreview(snapshotPath, targetVaultPath);
  const stagedRoot = ".bentolifelayout/imports/staged/snapshots/mock";
  const stagedFiles: StagedImportRecord[] = [
    {
      staged_file_path: `${stagedRoot}/modules/notes/data/Legacy Daily.md`,
      original_source_path: "modules/notes/data/Legacy Daily.md",
      source_kind: "snapshot",
      detected_title: "Legacy Daily",
      detected_tags: ["imported"],
      detected_links: [],
      detected_checklists: 2,
      suggested_module: "todos",
      accepted: false,
      ignored: false,
      conflict_status: preview.direct_restore_allowed ? null : "Older snapshot structure requires review before accept.",
    },
  ];
  mergeMockStagedImports(targetVaultPath, stagedFiles);
  const index = mockListStagedImports(targetVaultPath);
  return {
    snapshot_path: snapshotPath,
    target_vault_path: targetVaultPath,
    staged_root: stagedRoot,
    staged_at: new Date().toISOString(),
    staged_files: stagedFiles,
    index,
    warnings: ["Snapshot files were staged for review and were not copied into active module folders."],
  };
}

function mockPreviewAcceptImport(
  vaultPath: string,
  stagedFilePath: string,
  targetModule: string,
  options: ImportAcceptanceOptions,
): ImportAcceptPreview {
  const record = mockListStagedImports(vaultPath).records.find((candidate) => candidate.staged_file_path === stagedFilePath);
  if (!record) {
    throw new Error("Staged import was not found.");
  }
  const moduleId = canonicalModuleId(targetModule);
  const title = options.target_filename ?? record.detected_title ?? titleFromImportPath(stagedFilePath);
  return {
    vault_path: vaultPath,
    staged_file_path: stagedFilePath,
    target_module: moduleId,
    target_relative_path: `modules/${moduleId}/data/${slugForImport(title)}.md`,
    detected_title: record.detected_title,
    conflicts: [],
    warnings: record.suggested_module !== moduleId ? [`Suggested module is ${record.suggested_module}; selected module is ${moduleId}.`] : [],
    will_preserve_unknown_content: true,
  };
}

async function mockAcceptImportIntoModule(
  vaultPath: string,
  stagedFilePath: string,
  targetModule: string,
  options: ImportAcceptanceOptions,
): Promise<ImportAcceptReport> {
  const index = mockListStagedImports(vaultPath);
  const record = index.records.find((candidate) => candidate.staged_file_path === stagedFilePath);
  if (!record) {
    throw new Error("Staged import was not found.");
  }
  if (record.accepted) {
    throw new Error("Staged import was already accepted.");
  }
  if (!shouldShowInMockImportReview(record.original_source_path ?? record.staged_file_path, record.source_kind)) {
    throw new Error("System/runtime staged files cannot be accepted into modules.");
  }
  const preview = mockPreviewAcceptImport(vaultPath, stagedFilePath, targetModule, options);
  record.accepted = true;
  record.ignored = false;
  record.conflict_status = null;
  writeMockStagedImports(vaultPath, index.records);
  addMockAcceptedModuleRecord(record, preview.target_module, preview.target_relative_path);
  return {
    vault_path: vaultPath,
    staged_file_path: stagedFilePath,
    target_module: preview.target_module,
    accepted_relative_path: preview.target_relative_path,
    accepted_manifest_path: ".bentolifelayout/imports/accepted/accepted-mock.json",
    accepted_at: new Date().toISOString(),
    warnings: preview.warnings,
    cache: mockCoreCache(vaultPath),
  };
}

function mockIgnoreStagedImport(vaultPath: string, stagedFilePath: string): IgnoredStagedImportReport {
  const index = mockListStagedImports(vaultPath);
  const record = index.records.find((candidate) => candidate.staged_file_path === stagedFilePath);
  if (!record) {
    throw new Error("Staged import was not found.");
  }
  record.ignored = true;
  writeMockStagedImports(vaultPath, index.records);
  return {
    vault_path: vaultPath,
    staged_file_path: stagedFilePath,
    ignored_at: new Date().toISOString(),
    index: mockListStagedImports(vaultPath),
  };
}

function addMockAcceptedNote(record: StagedImportRecord, markdownRelativePath: string) {
  const notes = (() => {
    try {
      return JSON.parse(window.localStorage.getItem(mockNotesStorageKey) ?? "[]") as Array<Record<string, unknown>>;
    } catch {
      return [];
    }
  })();
  const title = record.detected_title ?? titleFromImportPath(record.staged_file_path);
  notes.push({
    document_id: `bl_doc_mock_import_${Date.now().toString(36)}`,
    title,
    markdown_relative_path: markdownRelativePath,
    markdown_body: `# ${title}\n\nImported from ${record.original_source_path ?? record.staged_file_path}.\n`,
    status: "managed",
    updated_at: new Date().toISOString(),
    content_hash: "mock-import",
  });
  window.localStorage.setItem(mockNotesStorageKey, JSON.stringify(notes));
}

function addMockAcceptedModuleRecord(record: StagedImportRecord, targetModule: string, markdownRelativePath: string) {
  if (targetModule === "notes") {
    addMockAcceptedNote(record, markdownRelativePath);
    return;
  }
  const title = record.detected_title ?? titleFromImportPath(record.staged_file_path);
  const now = new Date().toISOString();
  if (targetModule === "todos") {
    const todos = readMockArray<Record<string, unknown>>(mockTodosStorageKey);
    todos.push({
      document_id: `bl_doc_mock_import_todo_${Date.now().toString(36)}_${todos.length}`,
      title,
      markdown_relative_path: markdownRelativePath,
      markdown_body: `# ${title}\n\nStatus: Not started\nPriority: Medium\nTags: imported\n\n- [ ] Review ${title}\n`,
      updated_at: now,
    });
    window.localStorage.setItem(mockTodosStorageKey, JSON.stringify(todos));
    return;
  }
  if (targetModule === "contacts") {
    const contacts = readMockArray<Record<string, unknown>>(mockContactsStorageKey);
    contacts.push({
      contact_id: `contact_import_${Date.now().toString(36)}_${contacts.length}`,
      name: title,
      relationship: "Other",
      organization: record.original_source_path?.includes("Work") ? "Workplace" : null,
      email: title.toLowerCase().includes("email") ? "imported@example.com" : null,
      phone: null,
      tags: ["imported", ...record.detected_tags],
      relationships: [],
      notes: `Imported from ${record.original_source_path ?? record.staged_file_path}.`,
    });
    window.localStorage.setItem(mockContactsStorageKey, JSON.stringify(contacts));
    return;
  }
  if (targetModule === "habits") {
    const habits = readMockArray<Record<string, unknown>>(mockHabitsStorageKey);
    habits.push({
      habit_id: `habit_import_${Date.now().toString(36)}_${habits.length}`,
      name: title,
      frequency: "Daily",
      target: "Imported routine",
      tags: ["imported", ...record.detected_tags],
      relationships: [],
      notes: `Imported from ${record.original_source_path ?? record.staged_file_path}.`,
      checkins: [],
    });
    window.localStorage.setItem(mockHabitsStorageKey, JSON.stringify(habits));
  }
}

function readMockArray<T>(storageKey: string): T[] {
  try {
    return JSON.parse(window.localStorage.getItem(storageKey) ?? "[]") as T[];
  } catch {
    return [];
  }
}

function titleFromImportPath(path: string) {
  return path.split(/[\\/]/).pop()?.replace(/\.md$/i, "").replace(/[-_]+/g, " ") || "Imported File";
}

function slugForImport(value: string) {
  return value.toLowerCase().replace(/\.md$/i, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "imported-file";
}

function mockSnapshotRestorePreview(snapshotPath: string, targetVaultPath: string): SnapshotRestorePreview {
  if (!snapshotPath.trim() || !targetVaultPath.trim()) {
    throw new Error("Snapshot folder and target vault path are required.");
  }
  if (snapshotPath.toLowerCase().includes("missing")) {
    throw new Error("Snapshot manifest was not found.");
  }
  const lowered = snapshotPath.toLowerCase();
  const legacy = lowered.includes("legacy") || lowered.includes("old");
  const mixed = lowered.includes("mixed");
  const direct = !legacy && !mixed;
  return {
    snapshot_path: snapshotPath,
    target_vault_path: targetVaultPath,
    file_count: 3,
    snapshot_shape: mixed ? "mixed_or_unknown_snapshot" : legacy ? "legacy_bentolife_snapshot" : "v3_active_snapshot",
    direct_restore_allowed: direct,
    blocked_reason: direct
      ? null
      : "This snapshot uses an older BentoLife structure. It will be staged for review instead of restored directly.",
    legacy_file_count: legacy || mixed ? 2 : 0,
    active_v3_file_count: mixed || direct ? 1 : 0,
    legacy_file_paths: legacy || mixed ? ["notes/Legacy.md", "modules/todos.md"] : [],
    active_v3_file_paths: mixed || direct ? ["modules/notes/data/Current.md"] : [],
    hidden_runtime_file_paths: [".bentolifelayout/index.json"],
    recommended_action: direct ? "Direct restore V3 snapshot" : "Stage snapshot for review",
    conflicts: snapshotPath.toLowerCase().includes("conflict") ? ["INDEX.md already exists in the target vault."] : [],
    warnings: ["Restore preview does not mutate the source snapshot."],
  };
}

function mockTrashResult(vaultPath: string, originalRelativePath: string, trashRelativePath: string): TrashResult {
  return {
    action: "browser_mock",
    entry: {
      original_relative_path: originalRelativePath,
      trash_relative_path: trashRelativePath,
      trashed_at: new Date().toISOString(),
      content_hash: "mock-trash",
    },
    cache: mockCoreCache(vaultPath),
  };
}

function trashLifecycleEntry(result: TrashResult): FileLifecycleEntry {
  return mockLifecycleEntry(
    `.bentolifelayout/trash/${slugFromPath(result.entry.original_relative_path)}.trash.json`,
    result.entry.original_relative_path,
    result.entry.trash_relative_path,
    result.entry.trashed_at,
  );
}

function mockArchiveResult(vaultPath: string, originalRelativePath: string, archiveRelativePath: string): ArchiveResult {
  return {
    action: "browser_mock",
    entry: {
      original_relative_path: originalRelativePath,
      archive_relative_path: archiveRelativePath,
      archived_at: new Date().toISOString(),
      content_hash: "mock-archive",
    },
    cache: mockCoreCache(vaultPath),
  };
}

function archiveLifecycleEntry(result: ArchiveResult): FileLifecycleEntry {
  return mockLifecycleEntry(
    `.bentolifelayout/archive/${slugFromPath(result.entry.original_relative_path)}.archive.json`,
    result.entry.original_relative_path,
    result.entry.archive_relative_path,
    result.entry.archived_at,
  );
}

function mockLifecycleEntry(id: string, originalPath: string, currentPath: string, timestamp: string): FileLifecycleEntry {
  return {
    id,
    original_path: originalPath,
    current_path: currentPath,
    file_name: fileNameFromPath(originalPath),
    module_id: moduleIdFromPath(originalPath),
    deleted_or_archived_at: timestamp,
    size_bytes: 128,
    can_restore: true,
  };
}

function readMockLifecycleEntries(storageKey: string, vaultPath: string): FileLifecycleEntry[] {
  try {
    const allEntries = JSON.parse(window.localStorage.getItem(storageKey) ?? "{}") as Record<string, FileLifecycleEntry[]>;
    return allEntries[vaultPath] ?? [];
  } catch {
    return [];
  }
}

function writeMockLifecycleEntries(storageKey: string, vaultPath: string, entries: FileLifecycleEntry[]) {
  let allEntries: Record<string, FileLifecycleEntry[]>;
  try {
    allEntries = JSON.parse(window.localStorage.getItem(storageKey) ?? "{}") as Record<string, FileLifecycleEntry[]>;
  } catch {
    allEntries = {};
  }
  allEntries[vaultPath] = entries;
  window.localStorage.setItem(storageKey, JSON.stringify(allEntries));
}

function addMockLifecycleEntry(storageKey: string, vaultPath: string, entry: FileLifecycleEntry) {
  const entries = readMockLifecycleEntries(storageKey, vaultPath).filter((candidate) => candidate.id !== entry.id);
  writeMockLifecycleEntries(storageKey, vaultPath, [entry, ...entries]);
}

function removeMockLifecycleEntry(storageKey: string, vaultPath: string, entryId: string): FileLifecycleEntry {
  const entries = readMockLifecycleEntries(storageKey, vaultPath);
  const entry = entries.find((candidate) => candidate.id === entryId);
  if (!entry) {
    throw new Error("Lifecycle entry was not found.");
  }
  writeMockLifecycleEntries(storageKey, vaultPath, entries.filter((candidate) => candidate.id !== entryId));
  return entry;
}

function slugFromPath(path: string) {
  return path.replace(/[^a-zA-Z0-9]/g, "-");
}

function fileNameFromPath(path: string) {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function moduleIdFromPath(path: string) {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[0] === "modules" ? parts[1] ?? null : null;
}

export type ModuleDefinition = {
  id: string;
  display_name: string;
  kind: string;
  document_type: string;
  default_path: string;
  schema_path: string | null;
  index_path: string;
  data_path: string | null;
  default_view: string;
  enabled_by_default: boolean;
  enabled: boolean;
  available: boolean;
  installed: boolean;
  storage_kind: string;
  capabilities: string[];
  implementation_status: string;
  schema_warnings: string[];
  schema_version: number | null;
  schema_migration_version: number | null;
};

export type RegistryState = {
  modules: ModuleDefinition[];
};

export type ScannedDocumentStatus =
  | "managed"
  | "plain_markdown"
  | "metadata_missing"
  | "layout_missing"
  | "metadata_path_mismatch"
  | "duplicate_identity";

export type MarkdownInline =
  | { type: "text"; text: string }
  | { type: "strong"; children: MarkdownInline[] }
  | { type: "emphasis"; children: MarkdownInline[] }
  | { type: "delete"; children: MarkdownInline[] }
  | { type: "inline_code"; text: string }
  | { type: "link"; href: string; children: MarkdownInline[] }
  | { type: "wiki_link"; target: string }
  | { type: "tag"; tag: string };

export type MarkdownListItem = {
  checked?: boolean | null;
  children: MarkdownInline[];
  nested?: MarkdownBlock[];
};

export type MarkdownTextValue = string | MarkdownInline[];

export type MarkdownBlock =
  | { type: "heading"; level: number; text: MarkdownTextValue }
  | { type: "paragraph"; text: MarkdownTextValue }
  | { type: "blockquote"; children: MarkdownBlock[] }
  | { type: "horizontal_rule" }
  | { type: "list"; ordered?: boolean; items: string[] | MarkdownListItem[] }
  | { type: "ordered_list"; items: string[] | MarkdownListItem[] }
  | { type: "checklist"; items: Array<{ text: MarkdownTextValue; checked: boolean }> }
  | { type: "code"; language: string; content: string }
  | { type: "image"; alt: string; source: string; raw: string }
  | { type: "table"; rows: string[][] }
  | { type: "tags"; tags: string[] }
  | { type: "relationships"; links: string[] }
  | { type: "managed"; name: string; content: string }
  | { type: "unknown"; raw: string };

export type ParsedEntityContract = {
  module_id: string | null;
  entity_type: string | null;
  fields: Record<string, string>;
  field_descriptors: ParsedFieldDescriptor[];
  blocks: MarkdownBlock[];
  unknown_blocks: MarkdownBlock[];
  relationships: string[];
  tags: string[];
  path: string;
  content_hash: string;
};

export type ParsedFieldDescriptor = {
  id: string;
  label: string;
  type: string;
  renderer_id: string;
  value: string;
  editable: boolean;
  aliases: string[];
  options?: string[];
  default_value?: string | null;
  warnings: string[];
};

export async function loadModuleRegistry(vaultPath: string): Promise<RegistryState> {
  if (!isTauriRuntime()) {
    return readMockRegistryState();
  }
  const result = await safeInvoke("load_module_registry", { vaultPath }, normalizeRegistryState, emptyRegistryState());
  return result.data;
}

export async function setModuleEnabled(vaultPath: string, moduleId: string, enabled: boolean): Promise<RegistryState> {
  if (!isTauriRuntime()) {
    const state = readMockRegistryState();
    const module = state.modules.find(m => m.id === canonicalModuleId(moduleId));
    if (module) {
      module.enabled = enabled;
    }
    writeMockRegistryState(state);
    return state;
  }
  return normalizeRegistryState(await invoke<unknown>("set_module_enabled", { vaultPath, moduleId, enabled }));
}

function readMockRegistryState(): RegistryState {
  try {
    const stored = window.localStorage.getItem(mockModuleRegistryStorageKey);
    if (!stored) {
      return mockRegistryState();
    }
    const overrides = JSON.parse(stored) as Record<string, boolean>;
    const state = mockRegistryState();
    state.modules = state.modules.map((module) =>
      Object.prototype.hasOwnProperty.call(overrides, module.id)
        ? { ...module, enabled: Boolean(overrides[module.id]) }
        : module,
    );
    return state;
  } catch {
    return mockRegistryState();
  }
}

function writeMockRegistryState(state: RegistryState) {
  const enabledByModule = Object.fromEntries(state.modules.map((module) => [module.id, module.enabled]));
  window.localStorage.setItem(mockModuleRegistryStorageKey, JSON.stringify(enabledByModule));
}

function mockRegistryState(): RegistryState {
  const module = (
    definition: Omit<ModuleDefinition, "available" | "installed" | "schema_warnings" | "schema_version" | "schema_migration_version"> &
      Partial<Pick<ModuleDefinition, "available" | "installed" | "schema_warnings" | "schema_version" | "schema_migration_version">>,
  ): ModuleDefinition => ({
    available: true,
    installed: true,
    schema_warnings: [],
    schema_version: definition.schema_path ? 2 : null,
    schema_migration_version: definition.schema_path ? 1 : null,
    ...definition,
    id: canonicalModuleId(definition.id),
  });
  return {
    modules: [
      module({ id: "navigator", display_name: "Navigator", kind: "system", document_type: "navigator", default_path: "modules/navigator/INDEX.md", schema_path: null, index_path: "modules/navigator/INDEX.md", data_path: null, default_view: "system", enabled_by_default: true, enabled: true, storage_kind: "hybrid_managed_markdown_document", capabilities: [], implementation_status: "implemented" }),
      module({ id: "trash", display_name: "Trash", kind: "system", document_type: "trash", default_path: "modules/trash/INDEX.md", schema_path: null, index_path: "modules/trash/INDEX.md", data_path: null, default_view: "system", enabled_by_default: true, enabled: true, storage_kind: "internal", capabilities: [], implementation_status: "implemented" }),
      module({ id: "archive", display_name: "Archive", kind: "system", document_type: "archive", default_path: "modules/archive/INDEX.md", schema_path: null, index_path: "modules/archive/INDEX.md", data_path: null, default_view: "system", enabled_by_default: true, enabled: true, storage_kind: "internal", capabilities: [], implementation_status: "implemented" }),
      module({ id: "notes", display_name: "Notes", kind: "starter", document_type: "note", default_path: "modules/notes/INDEX.md", schema_path: "modules/notes/module.schema.json", index_path: "modules/notes/INDEX.md", data_path: "modules/notes/data", default_view: "cards", enabled_by_default: true, enabled: true, storage_kind: "per_entity_markdown_documents", capabilities: [], implementation_status: "implemented" }),
      module({ id: "todos", display_name: "Todos", kind: "starter", document_type: "todos", default_path: "modules/todos/INDEX.md", schema_path: "modules/todos/module.schema.json", index_path: "modules/todos/INDEX.md", data_path: "modules/todos/data", default_view: "cards", enabled_by_default: true, enabled: true, storage_kind: "per_entity_markdown_documents", capabilities: [], implementation_status: "implemented" }),
      module({ id: "contacts", display_name: "Contacts", kind: "optional", document_type: "contact", default_path: "modules/contacts/INDEX.md", schema_path: "modules/contacts/module.schema.json", index_path: "modules/contacts/INDEX.md", data_path: "modules/contacts/data", default_view: "cards", enabled_by_default: false, enabled: false, storage_kind: "per_entity_markdown_documents", capabilities: [], implementation_status: "implemented" }),
      module({ id: "habits", display_name: "Habits", kind: "optional", document_type: "habit", default_path: "modules/habits/INDEX.md", schema_path: "modules/habits/module.schema.json", index_path: "modules/habits/INDEX.md", data_path: "modules/habits/data", default_view: "cards", enabled_by_default: false, enabled: false, storage_kind: "per_entity_markdown_documents", capabilities: [], implementation_status: "implemented" }),
      module({ id: "vault", display_name: "Vault", kind: "system", document_type: "vault", default_path: ".bentolifelayout/vault/INDEX.md", schema_path: null, index_path: ".bentolifelayout/vault/INDEX.md", data_path: null, default_view: "system", enabled_by_default: true, enabled: true, storage_kind: "internal", capabilities: [], implementation_status: "planned" }),
      module({ id: "settings", display_name: "Settings", kind: "system", document_type: "settings", default_path: ".bentolifelayout/settings/INDEX.md", schema_path: null, index_path: ".bentolifelayout/settings/INDEX.md", data_path: null, default_view: "system", enabled_by_default: true, enabled: true, storage_kind: "internal", capabilities: [], implementation_status: "planned" }),
      module({ id: "architect", display_name: "Architect Mode", kind: "system", document_type: "architect", default_path: ".bentolifelayout/architect/INDEX.md", schema_path: null, index_path: ".bentolifelayout/architect/INDEX.md", data_path: null, default_view: "system", enabled_by_default: true, enabled: true, storage_kind: "internal", capabilities: [], implementation_status: "planned" })
    ]
  };
}

function canonicalModuleId(moduleId: string) {
  return moduleId === "todo" ? "todos" : moduleId;
}
