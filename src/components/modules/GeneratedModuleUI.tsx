import React, { useMemo, useState } from "react";

import { useI18n } from "@/i18n";
import { normalizeParsedEntityContract } from "@/services/contracts/parsedEntity.contract";
import type { ParsedEntityContract, ParsedFieldDescriptor } from "@/services/backendCore";
import { getRenderer, renderFieldValue } from "@/services/rendererRegistry";

export type GeneratedField = {
  id: string;
  label: string;
  renderer: string;
  aliases?: string[];
  editable?: boolean;
};

export type GeneratedModuleUIProps = {
  title: string;
  moduleLabel: string;
  entity: ParsedEntityContract;
  backlinks?: Array<{ source_path: string; target: string; link_type: string; status: string; raw: string }>;
  fields: GeneratedField[];
  documentId?: string | null;
  hideBody?: boolean;
  moduleId?: string | null;
  schemaWarnings?: string[];
  sourceMarkdown?: string;
  vaultPath?: string | null;
  onFieldDraftChange?: (fieldId: string, value: unknown) => void;
};

export function GeneratedModuleUI({
  title,
  moduleLabel,
  entity,
  backlinks = [],
  fields,
  documentId,
  hideBody = false,
  moduleId,
  schemaWarnings = [],
  sourceMarkdown,
  vaultPath,
  onFieldDraftChange,
}: GeneratedModuleUIProps) {
  const { t } = useI18n();
  void onFieldDraftChange;
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const safeEntity = useMemo(() => normalizeParsedEntityContract(entity), [entity]);
  const safeFields = useMemo(() => fields ?? [], [fields]);
  const displayFields = useMemo(() => {
    if (safeEntity.field_descriptors.length) {
      return safeEntity.field_descriptors
        .filter((field) => field.id !== "body" && field.id !== "tags" && field.id !== "relationships")
        .map((field) => descriptorToField(field));
    }
    return safeFields
      .filter((field) => field.id !== "body" && field.id !== "tags" && field.id !== "relationships")
      .map((field) => ({
        field,
        value: fieldValue(safeEntity, field),
        warnings: [] as string[],
      }));
  }, [safeEntity, safeFields]);
  const unknownFields = useMemo(() => {
    const staticFieldIds = safeFields.flatMap((field) => [field.id, ...(field.aliases ?? [])]);
    const descriptorFieldIds = safeEntity.field_descriptors.flatMap((field) => [field.id, ...field.aliases]);
    const known = new Set([...staticFieldIds, ...descriptorFieldIds].map(normalizeField));
    return Object.entries(safeEntity.fields).filter(([key]) => !known.has(normalizeField(key)));
  }, [safeEntity.field_descriptors, safeEntity.fields, safeFields]);
  const bodyBlocks = safeEntity.blocks;
  const renderContext = { documentId, moduleId: moduleId ?? safeEntity.module_id, vaultPath, markdownPath: safeEntity.path };
  const imported = Boolean(sourceMarkdown?.includes("bentolife:import_context"));
  const identityRows = [
    [t("nav.module"), safeEntity.module_id ?? moduleLabel],
    ["Type", safeEntity.entity_type ?? "entity"],
    [t("vault.path"), safeEntity.path],
    ["Hash", safeEntity.content_hash.slice(0, 12) || "pending"],
  ];

  return (
    <section className="grid min-h-0 gap-4">
      <div className="min-w-0 space-y-4">
        <div className="rounded-md border border-border bg-card p-5 shadow-soft">
          <div className="flex flex-wrap items-start justify-between gap-3 border-b border-border pb-4">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-primary">{moduleLabel}</p>
              <p className="mt-1 break-words text-2xl font-semibold leading-8">{title}</p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              {imported ? (
                <span className="rounded-md border border-sage/40 bg-sage/20 px-3 py-1.5 text-xs font-semibold text-sage-foreground">
                  {t("modules.editor.imported")}
                </span>
              ) : null}
              <span className="rounded-md border border-border bg-muted px-3 py-1.5 text-xs font-semibold text-muted-foreground">
                {t("modules.editor.readOnly")}
              </span>
              <button
                className="rounded-md border border-input bg-background px-3 py-1.5 text-xs font-semibold transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                onClick={() => setInspectorOpen((current) => !current)}
                type="button"
              >
                {inspectorOpen ? t("modules.editor.closeInspector") : t("modules.editor.openInspector")}
              </button>
            </div>
          </div>

          <div className="mt-5 grid gap-4 md:grid-cols-2">
            {displayFields
              .map(({ field, value, warnings }) => (
                <GeneratedFieldView
                  field={field}
                  key={field.id}
                  value={value}
                  warnings={warnings}
                />
              ))}
          </div>
        </div>

        {!hideBody ? (
        <div className="rounded-md border border-border bg-card p-5 shadow-soft">
          <div className="mb-4 flex items-center justify-between gap-3">
            <h2 className="text-sm font-semibold uppercase text-muted-foreground">{t("modules.editor.markdownEditor")}</h2>
            <span className="text-xs text-muted-foreground">{bodyBlocks.length} blocks</span>
          </div>
          {renderFieldValue("markdown", bodyBlocks, "body", undefined, renderContext)}
        </div>
        ) : null}

        {sourceMarkdown ? (
          <details className="rounded-md border border-border bg-muted p-4 text-sm text-muted-foreground">
            <summary className="cursor-pointer font-semibold text-foreground">{t("modules.editor.rawFallback")}</summary>
            <pre className="mt-3 max-h-72 overflow-auto whitespace-pre-wrap break-words text-xs">{sourceMarkdown}</pre>
          </details>
        ) : null}
      </div>

      {inspectorOpen ? (
      <aside className="min-w-0 space-y-4 rounded-md border border-border bg-background p-4" aria-label={`${moduleLabel} ${t("modules.editor.inspector")}`}>
        <Panel title={t("modules.editor.inspector")}>
          <dl className="space-y-3">
            {identityRows.map(([label, value]) => (
              <div key={label}>
                <dt className="text-xs font-semibold uppercase text-muted-foreground">{label}</dt>
                <dd className="mt-1 break-all text-sm">{value}</dd>
              </div>
            ))}
          </dl>
        </Panel>

        <Panel title={t("modules.editor.tagsAndLinks")}>
          <div className="space-y-4">
            {renderFieldValue("tags", safeEntity.tags, "tags", undefined, renderContext)}
            {renderFieldValue("relationships", safeEntity.relationships, "relationships", undefined, renderContext)}
          </div>
        </Panel>

        <Panel title={t("modules.editor.backlinks")}>
          {backlinks.length ? (
            <div className="space-y-2">
              {backlinks.map((link) => (
                <div className="rounded-md border border-border bg-muted/50 p-2" key={`${link.source_path}-${link.raw}`}>
                  <p className="break-all text-sm font-medium">{link.source_path}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{link.link_type} - {link.status}</p>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">{t("modules.editor.noBacklinks")}</p>
          )}
        </Panel>

        <Panel title={t("modules.editor.schemaWarnings")}>
          {schemaWarnings.length || unknownFields.length || safeEntity.unknown_blocks.length ? (
            <div className="space-y-2">
              <p className="text-sm text-muted-foreground">{t("modules.editor.unmappedPreserved")}</p>
              {schemaWarnings.map((warning) => <WarningLine key={warning} text={warning} />)}
              {unknownFields.map(([key, value]) => (
                <div className="rounded-md border border-border bg-amber-note/20 p-2" key={key}>
                  <div className="text-xs font-semibold uppercase text-amber-note-foreground">{t("modules.editor.unknownField")}: {key}</div>
                  <div className="mt-1 text-sm">{String(value)}</div>
                </div>
              ))}
              {safeEntity.unknown_blocks.map((block, index) => (
                <div className="rounded-md border border-border bg-amber-note/20 p-2" key={index}>
                  <div className="text-xs font-semibold uppercase text-amber-note-foreground">{t("modules.editor.unknownBlock")}</div>
                  <pre className="mt-1 whitespace-pre-wrap break-words text-xs">
                    {block.type === "unknown" ? block.raw : JSON.stringify(block)}
                  </pre>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">{t("modules.editor.noSchemaWarnings")}</p>
          )}
        </Panel>
      </aside>
      ) : null}
    </section>
  );
}

function GeneratedFieldView({
  field,
  value,
  warnings,
}: {
  field: GeneratedField;
  value: unknown;
  warnings: string[];
}) {
  const { t } = useI18n();
  const renderer = getRenderer(field.renderer);
  const Component = renderer.renderReadOnly;
  return (
    <div className="rounded-md border border-border bg-muted/60 p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <h3 className="text-xs font-semibold uppercase text-muted-foreground">{field.label}</h3>
        <span className="text-[11px] font-medium text-muted-foreground">{t("modules.editor.readOnly")}</span>
      </div>
      <Component fieldId={field.id} label={field.label} value={value} />
      {warnings.map((warning) => (
        <p className="mt-2 text-xs leading-5 text-amber-note-foreground" key={warning}>{warning}</p>
      ))}
    </div>
  );
}

function Panel({ children, title }: { children: React.ReactNode; title: string }) {
  return (
    <section className="rounded-md border border-border bg-card p-4 shadow-soft">
      <h2 className="mb-3 text-xs font-semibold uppercase text-muted-foreground">{title}</h2>
      {children}
    </section>
  );
}

function WarningLine({ text }: { text: string }) {
  return <p className="rounded-md border border-border bg-amber-note/20 p-2 text-sm text-amber-note-foreground">{text}</p>;
}

function fieldValue(entity: ParsedEntityContract, field: GeneratedField) {
  if (field.id === "body") return entity.blocks;
  if (field.id === "tags") return entity.tags;
  if (field.id === "relationships") return entity.relationships;
  const candidates = [field.id, ...(field.aliases ?? [])];
  for (const candidate of candidates) {
    const exact = entity.fields[candidate];
    if (exact) return exact;
    const normalized = Object.entries(entity.fields).find(([key]) => normalizeField(key) === normalizeField(candidate));
    if (normalized) return normalized[1];
  }
  return "";
}

function descriptorToField(descriptor: ParsedFieldDescriptor): { field: GeneratedField; value: unknown; warnings: string[] } {
  return {
    field: {
      id: descriptor.id,
      label: descriptor.label,
      renderer: descriptor.renderer_id || "generic",
      aliases: descriptor.aliases,
      editable: false,
    },
    value: descriptor.value,
    warnings: descriptor.warnings,
  };
}

function normalizeField(field: string) {
  return field.trim().toLowerCase().replace(/_/g, " ");
}
