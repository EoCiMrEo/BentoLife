import { unified } from "unified";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";

import type { MarkdownBlock, MarkdownInline, MarkdownListItem } from "@/services/backendCore";

type MarkdownNode = {
  type: string;
  value?: string;
  url?: string;
  alt?: string;
  lang?: string;
  depth?: number;
  ordered?: boolean;
  checked?: boolean | null;
  children?: MarkdownNode[];
};

const previewParser = unified().use(remarkParse).use(remarkGfm);

export function parseMarkdownPreview(markdown: string): MarkdownBlock[] {
  const root = previewParser.parse(markdown) as MarkdownNode;
  return toBlocks(root.children ?? []);
}

function toBlocks(nodes: MarkdownNode[]): MarkdownBlock[] {
  const blocks: MarkdownBlock[] = [];

  for (const node of nodes) {
    const block = toBlock(node);
    if (block) blocks.push(block);
  }

  return blocks;
}

function toBlock(node: MarkdownNode): MarkdownBlock | null {
  if (node.type === "heading") {
    return { type: "heading", level: clampHeading(node.depth ?? 1), text: toInline(node.children ?? []) };
  }
  if (node.type === "paragraph") {
    const standaloneImage = imageFromParagraph(node);
    if (standaloneImage) return standaloneImage;
    return { type: "paragraph", text: toInline(node.children ?? []) };
  }
  if (node.type === "list") {
    return {
      type: "list",
      ordered: Boolean(node.ordered),
      items: (node.children ?? []).filter((child) => child.type === "listItem").map(toListItem),
    };
  }
  if (node.type === "code") {
    return { type: "code", language: node.lang ?? "", content: node.value ?? "" };
  }
  if (node.type === "image") {
    const source = node.url ?? "";
    if (isUnsafeAssetSource(source)) return { type: "unknown", raw: `![${node.alt ?? ""}](${source})` };
    return { type: "image", alt: node.alt ?? source, source, raw: `![${node.alt ?? ""}](${source})` };
  }
  if (node.type === "blockquote") {
    return { type: "blockquote", children: toBlocks(node.children ?? []) };
  }
  if (node.type === "thematicBreak") {
    return { type: "horizontal_rule" };
  }
  if (node.type === "table") {
    const rows = (node.children ?? [])
      .filter((row) => row.type === "tableRow")
      .map((row) =>
        (row.children ?? [])
          .filter((cell) => cell.type === "tableCell")
          .map((cell) => inlinePlainText(toInline(cell.children ?? []))),
      );
    return rows.length ? { type: "table", rows } : null;
  }
  if (node.type === "html") {
    if (isBentoLifeImportContext(node.value ?? "")) return null;
    return { type: "unknown", raw: node.value ?? "" };
  }

  const fallback = node.value ?? inlinePlainText(toInline(node.children ?? []));
  return fallback ? { type: "unknown", raw: fallback } : null;
}

function isBentoLifeImportContext(value: string) {
  return value.trim().startsWith("<!-- bentolife:import_context");
}

function toListItem(node: MarkdownNode): MarkdownListItem {
  const children = node.children ?? [];
  const firstParagraph = children.find((child) => child.type === "paragraph");
  const nested = children.filter((child) => child.type !== "paragraph").map(toBlock).filter(Boolean) as MarkdownBlock[];

  return {
    checked: node.checked ?? null,
    children: firstParagraph ? toInline(firstParagraph.children ?? []) : toInline(children),
    nested,
  };
}

function toInline(nodes: MarkdownNode[]): MarkdownInline[] {
  const inline: MarkdownInline[] = [];

  for (const node of nodes) {
    if (node.type === "text") {
      inline.push(...tokenizeText(node.value ?? ""));
    } else if (node.type === "strong") {
      inline.push({ type: "strong", children: toInline(node.children ?? []) });
    } else if (node.type === "emphasis") {
      inline.push({ type: "emphasis", children: toInline(node.children ?? []) });
    } else if (node.type === "delete") {
      inline.push({ type: "delete", children: toInline(node.children ?? []) });
    } else if (node.type === "inlineCode") {
      inline.push({ type: "inline_code", text: node.value ?? "" });
    } else if (node.type === "link") {
      const href = node.url ?? "";
      inline.push(
        isUnsafeLink(href)
          ? { type: "text", text: inlinePlainText(toInline(node.children ?? [])) || href }
          : { type: "link", href, children: toInline(node.children ?? []) },
      );
    } else if (node.type === "break") {
      inline.push({ type: "text", text: "\n" });
    } else if (node.type === "image") {
      inline.push({ type: "text", text: node.alt ?? node.url ?? "" });
    } else if (node.children?.length) {
      inline.push(...toInline(node.children));
    } else if (node.value) {
      inline.push({ type: "text", text: node.value });
    }
  }

  return inline;
}

function tokenizeText(text: string): MarkdownInline[] {
  if (!text) return [];
  const tokens: MarkdownInline[] = [];
  const pattern = /(\[\[[^\]\r\n]+]])|(^|[\s(])#([A-Za-z0-9][\w/-]*)/g;
  let index = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text))) {
    if (match.index > index) {
      tokens.push({ type: "text", text: text.slice(index, match.index) });
    }
    if (match[1]) {
      tokens.push({ type: "wiki_link", target: match[1].slice(2, -2).trim() });
    } else {
      const prefix = match[2] ?? "";
      if (prefix) tokens.push({ type: "text", text: prefix });
      tokens.push({ type: "tag", tag: match[3] });
    }
    index = match.index + match[0].length;
  }

  if (index < text.length) {
    tokens.push({ type: "text", text: text.slice(index) });
  }
  return tokens;
}

function imageFromParagraph(node: MarkdownNode): MarkdownBlock | null {
  if (node.children?.length === 1 && node.children[0].type === "image") {
    const image = node.children[0];
    const source = image.url ?? "";
    if (isUnsafeAssetSource(source)) return { type: "unknown", raw: `![${image.alt ?? ""}](${source})` };
    return { type: "image", alt: image.alt ?? source, source, raw: `![${image.alt ?? ""}](${source})` };
  }

  const raw = inlinePlainText(toInline(node.children ?? [])).trim();
  const match = raw.match(/^!\[\[([^\]\r\n]+)]]$/);
  if (!match) return null;
  const source = match[1].trim();
  if (isUnsafeAssetSource(source)) return { type: "unknown", raw };
  return { type: "image", alt: source, source, raw };
}

function inlinePlainText(nodes: MarkdownInline[]): string {
  return nodes.map((node) => {
    if (node.type === "text") return node.text;
    if (node.type === "inline_code") return node.text;
    if (node.type === "wiki_link") return `[[${node.target}]]`;
    if (node.type === "tag") return `#${node.tag}`;
    if ("children" in node) return inlinePlainText(node.children);
    return "";
  }).join("");
}

function clampHeading(level: number) {
  return Math.min(6, Math.max(1, level));
}

function isUnsafeAssetSource(source: string) {
  const trimmed = source.trim();
  const lowered = trimmed.toLowerCase();
  return (
    !trimmed ||
    lowered.startsWith("http://") ||
    lowered.startsWith("https://") ||
    lowered.startsWith("data:") ||
    lowered.startsWith("javascript:") ||
    lowered.startsWith("file:") ||
    lowered.endsWith(".svg") ||
    trimmed.startsWith("/") ||
    trimmed.includes("\\")
  );
}

function isUnsafeLink(href: string) {
  const lowered = href.trim().toLowerCase();
  return lowered.startsWith("javascript:") || lowered.startsWith("data:") || lowered.startsWith("file:");
}
