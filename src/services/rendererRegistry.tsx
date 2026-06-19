import React from "react";
import { convertFileSrc } from "@tauri-apps/api/core";

import { useI18n } from "@/i18n";
import type { MarkdownBlock, MarkdownInline, MarkdownListItem, MarkdownTextValue } from "@/services/backendCore";
import { readMarkdownAsset } from "@/services/notes";

export type ValidationResult = { valid: boolean; error?: string };

export type RendererProps = {
  value: unknown;
  fieldId: string;
  label?: string;
  onChange?: (value: unknown) => void;
  context?: MarkdownRenderContext;
};

export type MarkdownRenderContext = {
  vaultPath?: string | null;
  markdownPath?: string | null;
  moduleId?: string | null;
  documentId?: string | null;
};

export type RendererDefinition = {
  id: string;
  supportedTypes: string[];
  renderReadOnly: React.ComponentType<RendererProps>;
  renderEditor?: React.ComponentType<RendererProps>;
  validate?: (value: unknown) => ValidationResult;
  serialize?: (value: unknown) => string;
};

const fieldShell = "rounded-md border border-border bg-card p-3 shadow-soft";

const GenericRenderer: React.FC<RendererProps> = ({ value }) => {
  const { t } = useI18n();
  return (
    <div className="rounded-md border border-border bg-amber-note/20 p-3 text-sm text-amber-note-foreground">
      <div className="mb-1 font-semibold">{t("renderer.unknownFormat")}</div>
      <pre className="max-h-48 overflow-auto whitespace-pre-wrap break-words text-xs">{stringify(value)}</pre>
    </div>
  );
};

const TextRenderer: React.FC<RendererProps> = ({ value }) => (
  <span className="break-words text-sm leading-6 text-foreground">{String(value ?? "")}</span>
);

const StatusRenderer: React.FC<RendererProps> = ({ value }) => {
  const label = String(value ?? "open");
  const lowered = label.toLowerCase();
  const tone = lowered.includes("done") || lowered.includes("complete")
    ? "border-sage/40 bg-sage/20 text-sage-foreground"
    : lowered.includes("blocked") || lowered.includes("stale")
      ? "border-amber-note/50 bg-amber-note/20 text-amber-note-foreground"
      : "border-soft-blue/40 bg-soft-blue/20 text-soft-blue-foreground";
  return <span className={`inline-flex rounded-md border px-2 py-1 text-xs font-semibold ${tone}`}>{label}</span>;
};

const DateRenderer: React.FC<RendererProps> = ({ value }) => {
  const { t } = useI18n();
  return <span className="font-mono text-sm text-foreground">{String(value ?? t("widgets.labels.noDate"))}</span>;
};

const TagsRenderer: React.FC<RendererProps> = ({ value }) => {
  const { t } = useI18n();
  const tags = asArray(value);
  if (!tags.length) return <span className="text-sm text-muted-foreground">{t("renderer.noTags")}</span>;
  return (
    <div className="flex flex-wrap gap-1.5">
      {tags.map((tag) => (
        <span key={tag} className="rounded-md border border-sage/40 bg-sage/20 px-2 py-1 text-xs font-medium text-sage-foreground">
          #{tag}
        </span>
      ))}
    </div>
  );
};

const RelationRenderer: React.FC<RendererProps> = ({ value }) => {
  const { t } = useI18n();
  const relations = asArray(value);
  if (!relations.length) return <span className="text-sm text-muted-foreground">{t("renderer.noRelationships")}</span>;
  return (
    <div className="flex flex-wrap gap-1.5">
      {relations.map((relation) => (
        <span key={relation} className="rounded-md border border-border bg-muted px-2 py-1 text-xs font-medium text-foreground">
          {relation}
        </span>
      ))}
    </div>
  );
};

const ChecklistRenderer: React.FC<RendererProps> = ({ value }) => {
  const { t } = useI18n();
  const items = Array.isArray(value) ? value as Array<{ text: MarkdownTextValue; checked: boolean }> : [];
  if (!items.length) return <span className="text-sm text-muted-foreground">{t("renderer.noChecklistItems")}</span>;
  return (
    <ul className="space-y-2">
      {items.map((item, index) => (
        <li className="flex min-h-6 items-start gap-2 text-sm text-foreground" key={`${plainInlineText(item.text)}-${index}`}>
          <input checked={item.checked} className="mt-1 h-4 w-4 accent-[var(--primary)]" readOnly type="checkbox" />
          <span className={item.checked ? "text-muted-foreground line-through" : ""}>{renderInlineValue(item.text)}</span>
        </li>
      ))}
    </ul>
  );
};

const CodeRenderer: React.FC<RendererProps> = ({ value, label }) => (
  <pre className="max-h-72 overflow-auto rounded-md bg-foreground p-3 text-xs leading-5 text-background">
    {label ? <code className="mb-2 block text-background/60">{label}</code> : null}
    <code>{String(value ?? "")}</code>
  </pre>
);

const ImageRenderer: React.FC<RendererProps> = ({ value, context }) => {
  const { t } = useI18n();
  const image = value as { alt?: string; source?: string; raw?: string } | null;
  const fallbackSrc = safeImageSrc(image?.source ?? "", context);
  const [src, setSrc] = React.useState<string | null>(fallbackSrc);
  const [failed, setFailed] = React.useState(false);

  React.useEffect(() => {
    let objectUrl: string | null = null;
    let cancelled = false;
    setFailed(false);
    setSrc(fallbackSrc);

    const source = image?.source ?? "";
    if (
      !source ||
      !context?.vaultPath ||
      !context.moduleId ||
      !context.documentId ||
      !("__TAURI_INTERNALS__" in window)
    ) {
      return undefined;
    }

    readMarkdownAsset(context.vaultPath, context.moduleId, context.documentId, source)
      .then((asset) => {
        if (cancelled) return;
        objectUrl = URL.createObjectURL(new Blob([new Uint8Array(asset.bytes)], { type: asset.mime_type }));
        setFailed(false);
        setSrc(objectUrl);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });

    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [context?.documentId, context?.markdownPath, context?.moduleId, context?.vaultPath, fallbackSrc, image?.source]);

  if (!src || failed) {
    return (
      <div className="rounded-md border border-border bg-muted p-3 text-sm text-muted-foreground">
        {t("renderer.imageUnavailable")}: {image?.alt || image?.source || t("renderer.missingLocalImage")}
      </div>
    );
  }
  return (
    <figure className="space-y-2">
      <img
        alt={image?.alt || t("renderer.markdownImage")}
        className="max-h-96 max-w-full rounded-md border border-border object-contain"
        onError={() => setFailed(true)}
        src={src}
      />
      {image?.alt ? <figcaption className="text-xs text-muted-foreground">{image.alt}</figcaption> : null}
    </figure>
  );
};

const TableRenderer: React.FC<RendererProps> = ({ value }) => {
  const { t } = useI18n();
  const rows = Array.isArray(value) ? value as string[][] : [];
  if (!rows.length) return <span className="text-sm text-muted-foreground">{t("renderer.emptyTable")}</span>;
  const [header, ...body] = rows;
  return (
    <div className="overflow-x-auto rounded-md border border-border">
      <table className="min-w-full border-collapse text-left text-sm">
        <thead className="bg-muted text-xs uppercase text-muted-foreground">
          <tr>{header.map((cell, index) => <th className="border-b border-border px-3 py-2" key={`${cell}-${index}`}>{cell}</th>)}</tr>
        </thead>
        <tbody>
          {body.map((row, rowIndex) => (
            <tr className="odd:bg-card even:bg-muted/50" key={rowIndex}>
              {row.map((cell, index) => <td className="border-b border-border px-3 py-2 text-foreground" key={`${cell}-${index}`}>{cell}</td>)}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const MarkdownRenderer: React.FC<RendererProps> = ({ value, context }) => {
  const blocks = Array.isArray(value) ? value as MarkdownBlock[] : [];
  if (!blocks.length) return <GenericRenderer fieldId="raw-markdown" value={value} />;
  return (
    <div className="space-y-4">
      {blocks.map((block, index) => <MarkdownBlockRenderer block={block} context={context} key={`${block.type}-${index}`} />)}
    </div>
  );
};

const TextEditor: React.FC<RendererProps> = ({ value, onChange }) => (
  <input
    className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm text-foreground outline-none focus:border-ring"
    onChange={(event) => onChange?.(event.target.value)}
    value={String(value ?? "")}
  />
);

const RendererRegistry: Record<string, RendererDefinition> = {
  generic: { id: "generic", supportedTypes: ["*"], renderReadOnly: GenericRenderer },
  text: { id: "text", supportedTypes: ["text", "string"], renderReadOnly: TextRenderer, renderEditor: TextEditor },
  textarea: { id: "textarea", supportedTypes: ["text", "markdown"], renderReadOnly: TextRenderer, renderEditor: TextEditor },
  status: { id: "status", supportedTypes: ["status", "enum", "text"], renderReadOnly: StatusRenderer, renderEditor: TextEditor },
  date: { id: "date", supportedTypes: ["date", "text"], renderReadOnly: DateRenderer, renderEditor: TextEditor },
  tags: { id: "tags", supportedTypes: ["tags", "array"], renderReadOnly: TagsRenderer },
  relation: { id: "relation", supportedTypes: ["relationship", "array"], renderReadOnly: RelationRenderer },
  relationships: { id: "relationships", supportedTypes: ["relationship", "array"], renderReadOnly: RelationRenderer },
  checklist: { id: "checklist", supportedTypes: ["checklist"], renderReadOnly: ChecklistRenderer },
  code: { id: "code", supportedTypes: ["code"], renderReadOnly: CodeRenderer },
  image: { id: "image", supportedTypes: ["image"], renderReadOnly: ImageRenderer },
  table: { id: "table", supportedTypes: ["table"], renderReadOnly: TableRenderer },
  markdown: { id: "markdown", supportedTypes: ["markdown", "blocks"], renderReadOnly: MarkdownRenderer },
};

export function getRenderer(id: string): RendererDefinition {
  return RendererRegistry[id] || RendererRegistry.generic;
}

export function renderFieldValue(rendererId: string, value: unknown, fieldId: string, label?: string, context?: MarkdownRenderContext) {
  const RendererComponent = getRenderer(rendererId).renderReadOnly;
  return <RendererComponent context={context} fieldId={fieldId} label={label} value={value} />;
}

function MarkdownBlockRenderer({ block, context }: { block: MarkdownBlock; context?: MarkdownRenderContext }) {
  if (block.type === "heading") {
    const className = "text-lg font-semibold leading-7 text-foreground";
    const content = renderInlineValue(block.text);
    if (block.level === 1) return <h1 className={className}>{content}</h1>;
    if (block.level === 2) return <h2 className={className}>{content}</h2>;
    if (block.level === 3) return <h3 className={className}>{content}</h3>;
    if (block.level === 4) return <h4 className={className}>{content}</h4>;
    if (block.level === 5) return <h5 className={className}>{content}</h5>;
    return <h6 className={className}>{content}</h6>;
  }
  if (block.type === "paragraph") return <p className="whitespace-pre-wrap text-sm leading-6 text-muted-foreground">{renderInlineValue(block.text)}</p>;
  if (block.type === "blockquote") {
    return (
      <blockquote className="space-y-3 border-l-4 border-border pl-4 text-muted-foreground">
        {block.children.map((child, index) => <MarkdownBlockRenderer block={child} context={context} key={`${child.type}-${index}`} />)}
      </blockquote>
    );
  }
  if (block.type === "horizontal_rule") return <hr className="border-border" />;
  if (block.type === "list" || block.type === "ordered_list") {
    const items = normalizeListItems(block.items);
    const ordered = block.type === "ordered_list" || block.ordered;
    const ListTag = ordered ? "ol" : "ul";
    return (
      <ListTag className={`${ordered ? "list-decimal" : "list-disc"} space-y-1 pl-5 text-sm text-muted-foreground`}>
        {items.map((item, index) => (
          <li key={`${plainInlineText(item.children)}-${index}`}>
            <div className="flex min-h-6 items-start gap-2">
              {typeof item.checked === "boolean" ? <input checked={item.checked} className="mt-0.5 h-4 w-4 accent-[var(--primary)]" readOnly type="checkbox" /> : null}
              <span className={item.checked ? "text-muted-foreground line-through" : ""}>{renderInlineValue(item.children)}</span>
            </div>
            {item.nested?.length ? (
              <div className="mt-2 space-y-2">
                {item.nested.map((child, childIndex) => <MarkdownBlockRenderer block={child} context={context} key={`${child.type}-${childIndex}`} />)}
              </div>
            ) : null}
          </li>
        ))}
      </ListTag>
    );
  }
  if (block.type === "checklist") return <ChecklistRenderer fieldId="checklist" value={block.items} />;
  if (block.type === "code") return <CodeRenderer fieldId="code" label={block.language} value={block.content} />;
  if (block.type === "image") return <ImageRenderer context={context} fieldId="image" value={block} />;
  if (block.type === "table") return <TableRenderer fieldId="table" value={block.rows} />;
  if (block.type === "tags") return <TagsRenderer fieldId="tags" value={block.tags} />;
  if (block.type === "relationships") return <RelationRenderer fieldId="relationships" value={block.links} />;
  if (block.type === "managed") {
    return (
      <div className={`${fieldShell} border-l-4 border-l-primary`}>
        <div className="mb-2 text-xs font-semibold uppercase text-muted-foreground">Managed: {block.name}</div>
        <p className="whitespace-pre-wrap text-sm text-muted-foreground">{block.content}</p>
      </div>
    );
  }
  return <GenericRenderer fieldId="unknown" value={block.raw} />;
}

function renderInlineValue(value: MarkdownTextValue): React.ReactNode {
  if (typeof value === "string") return renderTextTokens(value);
  return value.map((node, index) => <React.Fragment key={`${node.type}-${index}`}>{renderInlineNode(node)}</React.Fragment>);
}

function renderInlineNode(node: MarkdownInline): React.ReactNode {
  if (node.type === "text") return renderTextTokens(node.text);
  if (node.type === "strong") return <strong className="font-semibold text-foreground">{renderInlineValue(node.children)}</strong>;
  if (node.type === "emphasis") return <em>{renderInlineValue(node.children)}</em>;
  if (node.type === "delete") return <del>{renderInlineValue(node.children)}</del>;
  if (node.type === "inline_code") return <code className="rounded bg-muted px-1 py-0.5 font-mono text-xs text-foreground">{node.text}</code>;
  if (node.type === "link") {
    return (
      <a className="font-medium text-primary underline-offset-2 hover:underline" href={safeHref(node.href)} rel="noreferrer" target={isExternalHref(node.href) ? "_blank" : undefined}>
        {renderInlineValue(node.children)}
      </a>
    );
  }
  if (node.type === "wiki_link") return <span className="rounded bg-muted px-1.5 py-0.5 text-xs font-medium text-foreground">[[{node.target}]]</span>;
  if (node.type === "tag") return <span className="rounded-md border border-sage/40 bg-sage/20 px-1.5 py-0.5 text-xs font-medium text-sage-foreground">#{node.tag}</span>;
  return null;
}

function renderTextTokens(text: string): React.ReactNode {
  const tokens: React.ReactNode[] = [];
  const pattern = /(\[\[[^\]\r\n]+]])|(^|[\s(])#([A-Za-z0-9][\w/-]*)/g;
  let index = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text))) {
    if (match.index > index) tokens.push(text.slice(index, match.index));
    if (match[1]) {
      tokens.push(<span className="rounded bg-muted px-1.5 py-0.5 text-xs font-medium text-foreground" key={`wiki-${match.index}`}>{match[1]}</span>);
    } else {
      const prefix = match[2] ?? "";
      if (prefix) tokens.push(prefix);
      tokens.push(<span className="rounded-md border border-sage/40 bg-sage/20 px-1.5 py-0.5 text-xs font-medium text-sage-foreground" key={`tag-${match.index}`}>#{match[3]}</span>);
    }
    index = match.index + match[0].length;
  }

  if (index < text.length) tokens.push(text.slice(index));
  return tokens.length ? tokens : text;
}

function normalizeListItems(items: string[] | MarkdownListItem[]): MarkdownListItem[] {
  return items.map((item) => typeof item === "string" ? { children: [{ type: "text", text: item }] } : item);
}

function plainInlineText(value: MarkdownTextValue): string {
  if (typeof value === "string") return value;
  return value.map((node) => {
    if (node.type === "text") return node.text;
    if (node.type === "inline_code") return node.text;
    if (node.type === "wiki_link") return node.target;
    if (node.type === "tag") return node.tag;
    if ("children" in node) return plainInlineText(node.children);
    return "";
  }).join("");
}

function asArray(value: unknown) {
  if (Array.isArray(value)) return value.map(String).filter(Boolean);
  if (typeof value === "string") return value.split(",").map((part) => part.trim()).filter(Boolean);
  return [];
}

function safeImageSrc(source: string, context?: MarkdownRenderContext) {
  const resolved = resolveVaultRelativeImage(source, context?.markdownPath ?? "");
  if (!resolved) return null;
  if (context?.vaultPath && "__TAURI_INTERNALS__" in window) {
    const base = context.vaultPath.replace(/\\/g, "/").replace(/\/$/, "");
    return convertFileSrc(`${base}/${resolved}`);
  }
  return source;
}

function resolveVaultRelativeImage(source: string, markdownPath: string) {
  const normalized = source.trim().replace(/\\/g, "/");
  const lowered = normalized.toLowerCase();
  if (
    !normalized ||
    lowered.startsWith("http://") ||
    lowered.startsWith("https://") ||
    lowered.startsWith("data:") ||
    lowered.startsWith("javascript:") ||
    lowered.startsWith("file:") ||
    lowered.endsWith(".svg") ||
    normalized.startsWith("/")
  ) {
    return null;
  }

  const base = markdownPath
    .replace(/\\/g, "/")
    .split("/")
    .filter(Boolean)
    .slice(0, -1);
  const stack = normalized.startsWith("assets/") ? [] : [...base];
  for (const part of normalized.split("/").filter(Boolean)) {
    if (part === ".") continue;
    if (part === "..") {
      if (!stack.length) return null;
      stack.pop();
      continue;
    }
    stack.push(part);
  }
  if (stack[0] !== "assets" && stack.join("/") !== base.concat(stack.slice(base.length)).join("/")) return null;
  return stack.join("/");
}

function safeHref(href: string) {
  const lowered = href.trim().toLowerCase();
  if (lowered.startsWith("javascript:") || lowered.startsWith("data:") || lowered.startsWith("file:")) return "#";
  return href;
}

function isExternalHref(href: string) {
  const lowered = href.trim().toLowerCase();
  return lowered.startsWith("http://") || lowered.startsWith("https://");
}

function stringify(value: unknown) {
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}
