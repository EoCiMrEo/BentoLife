// Generated from the Rust bentolife_core contract surface.
// Keep this file aligned by running the core crate ts-rs export test after Rust contract changes.

export type ExternalSourceFile = {
  source_relative_path: string;
  target_relative_path: string;
  file_kind: string;
  document_id: string | null;
  title: string | null;
  content_hash: string;
  copied: boolean;
  collision_renamed: boolean;
  skipped: boolean;
  reason: string | null;
};

export type ExternalSourceScan = {
  source_path: string;
  source_kind: string;
  markdown_count: number;
  asset_count: number;
  ignored_count: number;
  files: ExternalSourceFile[];
  warnings: string[];
};

export type FolderImportPreview = {
  source_path: string;
  vault_path: string;
  scan: ExternalSourceScan;
  target_root: string;
  planned_files: ExternalSourceFile[];
  conflicts: string[];
  warnings: string[];
};

export type FolderImportManifest = {
  schema_version: number;
  source_path: string;
  vault_path: string;
  target_root: string;
  imported_at: string;
  files: ExternalSourceFile[];
  conflicts: string[];
  warnings: string[];
};

export type SnapshotVaultShape = "v3_active_snapshot" | "legacy_bentolife_snapshot" | "mixed_or_unknown_snapshot";

export type StagedImportRecord = {
  staged_file_path: string;
  original_source_path: string | null;
  source_kind: string;
  detected_title: string | null;
  detected_tags: string[];
  detected_links: string[];
  detected_checklists: number;
  suggested_module: string;
  accepted: boolean;
  ignored: boolean;
  conflict_status: string | null;
};

export type StagedImportIndex = {
  schema_version: number;
  vault_path: string;
  updated_at: string;
  records: StagedImportRecord[];
  hidden_system_count: number;
  hidden_system_files: string[];
  warnings: string[];
};

export type ImportAcceptanceOptions = {
  target_filename: string | null;
  tags: string[];
  preserve_source_path: boolean;
  batch_tag: string | null;
};

export type ImportAcceptPreview = {
  vault_path: string;
  staged_file_path: string;
  target_module: string;
  target_relative_path: string;
  detected_title: string | null;
  conflicts: string[];
  warnings: string[];
  will_preserve_unknown_content: boolean;
};

export type ImportAcceptReport = {
  vault_path: string;
  staged_file_path: string;
  target_module: string;
  accepted_relative_path: string;
  accepted_manifest_path: string;
  accepted_at: string;
  warnings: string[];
  cache: CoreCacheSnapshot;
};

export type IgnoredStagedImportReport = {
  vault_path: string;
  staged_file_path: string;
  ignored_at: string;
  index: StagedImportIndex;
};

export type BulkImportAcceptReport = {
  vault_path: string;
  accepted: ImportAcceptReport[];
  errors: string[];
};

export type SnapshotStageReport = {
  snapshot_path: string;
  target_vault_path: string;
  staged_root: string;
  staged_at: string;
  staged_files: StagedImportRecord[];
  index: StagedImportIndex;
  warnings: string[];
};

export type GraphLink = {
  source_path: string;
  target: string;
  link_type: string;
  raw: string;
  status: string;
  resolved_document_id: string | null;
  resolved_path: string | null;
};

export type GraphRelationship = {
  source_path: string;
  target: string;
  relationship_type: string;
  raw: string;
};

export type EntityMetadata = {
  document_id: string | null;
  current_path: string;
  title: string;
  entity_type: string;
  metadata_path: string | null;
  content_hash: string;
  backlinks: GraphLink[];
  tags: string[];
  relationships: GraphRelationship[];
  unresolved_links: GraphLink[];
};

export type GraphHealthWarning = {
  code: string;
  message: string;
  document_id: string | null;
  path: string | null;
};

export type DocumentIndexEntry = {
  current_path: string;
  metadata_path: string;
  layout_path: string;
};

export type RebuildPolicy = {
  rebuild_from_documents_folder: boolean;
  rebuild_from_markdown_uuid_comments: boolean;
  treat_index_as_cache: boolean;
};

export type IndexSnapshot = {
  schema_version: number;
  path_policy: string;
  documents_by_id: Record<string, DocumentIndexEntry>;
  document_ids_by_path: Record<string, string>;
  orphaned_document_ids: string[];
  duplicate_identity_conflicts: string[];
  updated_at: string;
  rebuild_policy: RebuildPolicy;
};

export type CoreCacheSnapshot = {
  schema_version: number;
  vault_path: string;
  entities_by_path: Record<string, EntityMetadata>;
  graph_links: GraphLink[];
  health_warnings: GraphHealthWarning[];
  index: IndexSnapshot;
  updated_at: string;
  invalidation_reason: string | null;
};

export type SearchIndexEntry = {
  path: string;
  document_id: string | null;
  title: string;
  entity_type: string;
  tags: string[];
  headings: string[];
  excerpt: string;
  searchable_text: string;
};

export type SearchIndexSnapshot = {
  schema_version: number;
  vault_path: string;
  index_path: string;
  entries: SearchIndexEntry[];
  updated_at: string;
  source_cache_updated_at: string;
};

export type NavigatorModuleSummary = {
  module_id: string;
  entity_count: number;
  index_path: string;
};

export type NavigatorSnapshot = {
  schema_version: number;
  vault_path: string;
  navigator_path: string;
  index_path: string;
  markdown: string;
  module_summaries: NavigatorModuleSummary[];
  health_warnings: GraphHealthWarning[];
  managed_block_warnings: string[];
  backlinks: GraphLink[];
  search_index_path: string;
  updated_at: string;
};

export type EntityUpgradeChange = {
  source_path: string;
  target_path: string;
  entity_type: string;
  title: string;
  document_id: string;
  markdown_body: string;
  action: string;
};

export type EntityUpgradePreview = {
  schema_version: number;
  vault_path: string;
  changes: EntityUpgradeChange[];
  legacy_paths: string[];
  warnings: string[];
};

export type EntityUpgradeReport = {
  schema_version: number;
  vault_path: string;
  upgraded_at: string;
  manifest_path: string;
  backup_root: string;
  changes: EntityUpgradeChange[];
  trashed_legacy_paths: string[];
  cache: CoreCacheSnapshot;
};

export type SnapshotFileEntry = {
  relative_path: string;
  file_kind: string;
  content_hash: string;
  byte_len: number;
};

export type VaultSnapshotManifest = {
  schema_version: number;
  source_vault_path: string;
  snapshot_path: string;
  created_at: string;
  source_machine: string;
  files: SnapshotFileEntry[];
  warnings: string[];
};

export type VaultSnapshotPreview = {
  source_vault_path: string;
  snapshot_path: string;
  file_count: number;
  total_bytes: number;
  warnings: string[];
};

export type SnapshotRestorePreview = {
  snapshot_path: string;
  target_vault_path: string;
  file_count: number;
  snapshot_shape: SnapshotVaultShape;
  direct_restore_allowed: boolean;
  blocked_reason: string | null;
  legacy_file_count: number;
  active_v3_file_count: number;
  legacy_file_paths: string[];
  active_v3_file_paths: string[];
  hidden_runtime_file_paths: string[];
  recommended_action: string;
  conflicts: string[];
  warnings: string[];
};

export type SnapshotRestoreReport = {
  snapshot_path: string;
  target_vault_path: string;
  restored_at: string;
  restored_files: SnapshotFileEntry[];
  conflicts: string[];
  warnings: string[];
  cache: CoreCacheSnapshot;
};

export type TrashEntry = {
  original_relative_path: string;
  trash_relative_path: string;
  trashed_at: string;
  content_hash: string;
};

export type TrashResult = {
  action: string;
  entry: TrashEntry;
  cache: CoreCacheSnapshot;
};

export type ArchiveEntry = {
  original_relative_path: string;
  archive_relative_path: string;
  archived_at: string;
  content_hash: string;
};

export type ArchiveResult = {
  action: string;
  entry: ArchiveEntry;
  cache: CoreCacheSnapshot;
};

export type FileLifecycleEntry = {
  id: string;
  original_path: string;
  current_path: string;
  file_name: string;
  module_id: string | null;
  deleted_or_archived_at: string | null;
  size_bytes: number | null;
  can_restore: boolean;
};

export type FileLifecycleMutationReport = {
  action: string;
  message: string;
  changed_count: number;
  entries: FileLifecycleEntry[];
};

export type WorkspaceSchema = {
  schema_version: number;
  vault_folder: string;
  app_folder: string;
  index_path: string;
};

export type WorkspaceStateContract = {
  schema_version: number;
  workspace_name: string;
  index_path: string;
  default_theme: string;
};

export type ThemeManifest = {
  schema_version: number;
  theme_id: string;
  scope: string;
  css_path: string | null;
  json_path: string | null;
  active: boolean;
};
