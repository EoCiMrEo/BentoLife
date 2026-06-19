const knownPerfToggles = [
  "safe",
  "no-shadows",
  "no-gradient",
  "no-widget-grid",
  "fixed-widget-rows",
  "no-icons",
  "no-backdrop-blur",
  "no-sticky-shell",
  "no-widget-bodies",
] as const;

export type PerfToggle = (typeof knownPerfToggles)[number];

export function parsePerfToggles(search: string): PerfToggle[] {
  const params = new URLSearchParams(search.startsWith("?") ? search.slice(1) : search);
  const rawValues = params.getAll("perf").flatMap((value) => value.split(","));
  const toggles = new Set<PerfToggle>();
  for (const rawValue of rawValues) {
    const value = rawValue.trim();
    if (isPerfToggle(value)) {
      toggles.add(value);
    }
  }
  return [...toggles];
}

export function applyPerfToggles(root: HTMLElement, toggles: PerfToggle[]) {
  for (const toggle of knownPerfToggles) {
    const attribute = perfToggleAttribute(toggle);
    if (toggles.includes(toggle)) {
      root.setAttribute(attribute, "true");
    } else {
      root.removeAttribute(attribute);
    }
  }
}

export function perfToggleAttribute(toggle: PerfToggle) {
  return `data-perf-${toggle}`;
}

function isPerfToggle(value: string): value is PerfToggle {
  return (knownPerfToggles as readonly string[]).includes(value);
}
