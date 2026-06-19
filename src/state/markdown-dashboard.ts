import type { LayoutMetadata } from "@/services/notes";

export type DashboardCardWidth = "single" | "double" | "full";

export type DashboardBlock =
  | { type: "paragraph"; text: string }
  | { type: "subheading"; text: string; level: number }
  | { type: "list"; items: string[] }
  | { type: "checklist"; items: Array<{ checked: boolean; text: string }> };

export type MarkdownDashboardCard = {
  id: string;
  title: string;
  heading: string;
  level: 1 | 2;
  order: number;
  width: DashboardCardWidth;
  stale: boolean;
  blocks: DashboardBlock[];
};

export type MarkdownDashboard = {
  title: string;
  cards: MarkdownDashboardCard[];
  staleLayoutMatches: string[];
  repairNeeded: boolean;
};

export function parseMarkdownDashboard(markdown: string, layoutMetadata?: LayoutMetadata | null): MarkdownDashboard {
  const lines = stripBentoLifeMetadata(markdown).split(/\r?\n/);
  const cards: MarkdownDashboardCard[] = [];
  let documentTitle = "Untitled";
  let currentCard: MarkdownDashboardCard | undefined;

  const ensureCard = () => {
    if (!currentCard) {
      currentCard = createCard("Overview", "## Overview", 2, cards.length);
      cards.push(currentCard);
    }
    return currentCard;
  };

  for (let index = 0; index < lines.length; index += 1) {
    const rawLine = lines[index] ?? "";
    const line = rawLine.trim();
    if (!line) {
      continue;
    }

    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      const title = heading[2].trim();
      if (level === 1 && documentTitle === "Untitled") {
        documentTitle = title;
      }
      if (level <= 2) {
        currentCard = createCard(title, `${"#".repeat(level)} ${title}`, level as 1 | 2, cards.length);
        cards.push(currentCard);
      } else {
        ensureCard().blocks.push({ type: "subheading", text: title, level });
      }
      continue;
    }

    const checklist = /^[-*]\s+\[([ xX])]\s+(.+)$/.exec(line);
    if (checklist) {
      const items: Array<{ checked: boolean; text: string }> = [];
      while (index < lines.length) {
        const item = /^[-*]\s+\[([ xX])]\s+(.+)$/.exec(lines[index]?.trim() ?? "");
        if (!item) {
          index -= 1;
          break;
        }
        items.push({ checked: item[1].toLowerCase() === "x", text: item[2].trim() });
        index += 1;
      }
      ensureCard().blocks.push({ type: "checklist", items });
      continue;
    }

    const list = /^(?:[-*]|\d+\.)\s+(.+)$/.exec(line);
    if (list) {
      const items: string[] = [];
      while (index < lines.length) {
        const item = /^(?:[-*]|\d+\.)\s+(.+)$/.exec(lines[index]?.trim() ?? "");
        if (!item || /^[-*]\s+\[[ xX]\]\s+/.test(lines[index]?.trim() ?? "")) {
          index -= 1;
          break;
        }
        items.push(item[1].trim());
        index += 1;
      }
      ensureCard().blocks.push({ type: "list", items });
      continue;
    }

    const paragraphLines = [line];
    while (index + 1 < lines.length) {
      const next = lines[index + 1]?.trim() ?? "";
      if (!next || /^#{1,6}\s+/.test(next) || /^[-*]\s+/.test(next) || /^\d+\.\s+/.test(next)) {
        break;
      }
      paragraphLines.push(next);
      index += 1;
    }
    ensureCard().blocks.push({ type: "paragraph", text: paragraphLines.join(" ") });
  }

  if (cards.length === 0) {
    cards.push(createCard("Empty note", "## Empty note", 2, 0));
  }

  const layoutApplied = applyLayout(cards, layoutMetadata);

  return {
    title: documentTitle,
    cards: layoutApplied.cards,
    staleLayoutMatches: layoutApplied.staleLayoutMatches,
    repairNeeded: !layoutMetadata || layoutApplied.staleLayoutMatches.length > 0,
  };
}

export function stripBentoLifeMetadata(markdown: string) {
  return markdown
    .replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, "")
    .replace(/<!--\s*bentolife:document_id=.*?-->/g, "")
    .trim();
}

function applyLayout(cards: MarkdownDashboardCard[], layoutMetadata?: LayoutMetadata | null) {
  if (!layoutMetadata) {
    return {
      cards: cards.map((card) => ({ ...card, width: "single" as DashboardCardWidth })),
      staleLayoutMatches: [],
    };
  }

  const cardsByHeading = new Map(cards.map((card) => [card.heading, card]));
  const layoutByHeading = new Map(layoutMetadata.cards.map((card) => [card.section_match, card]));
  const staleLayoutMatches = layoutMetadata.cards
    .filter((card) => !cardsByHeading.has(card.section_match))
    .map((card) => card.section_match);

  const appliedCards = cards.map((card) => {
    const layout = layoutByHeading.get(card.heading);
    return {
      ...card,
      order: layout?.order ?? card.order,
      stale: false,
      width: normalizeWidth(layout?.width ?? layoutMetadata.fallback_layout.default_width),
    };
  });

  appliedCards.sort((left, right) => left.order - right.order);

  return { cards: appliedCards, staleLayoutMatches };
}

function createCard(title: string, heading: string, level: 1 | 2, order: number): MarkdownDashboardCard {
  return {
    id: `${heading.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "card"}-${order}`,
    title,
    heading,
    level,
    order,
    width: "single",
    stale: false,
    blocks: [],
  };
}

function normalizeWidth(width: string): DashboardCardWidth {
  return width === "full" || width === "double" ? width : "single";
}
