import { invoke } from "@tauri-apps/api/core";

import { emptyHabitDocument, normalizeHabitDocument } from "@/services/contracts/habits.contract";
import { safeInvoke } from "@/services/contracts/invokeClient";
import type { LayoutMetadata } from "@/services/notes";
import type { ParsedEntityContract } from "@/services/backendCore";
import { isTauriRuntime } from "@/services/vault";

export type HabitInput = {
  name: string;
  frequency?: string | null;
  target?: string | null;
  tags?: string[];
  relationships?: string[];
  notes?: string | null;
};

export type HabitEntry = Required<Omit<HabitInput, "frequency" | "target" | "notes">> & {
  habit_id: string;
  frequency: string | null;
  target: string | null;
  notes: string | null;
  checkins: string[];
  line_index: number;
  raw_markdown: string;
  parsed_entity: ParsedEntityContract;
  schema_warnings: string[];
};

export type HabitSummary = {
  total: number;
  summary_date: string | null;
  checked_in_on_date: number;
  recent_checkins: Array<{ habit_id: string; name: string; date: string }>;
  streaks: Array<{ habit_id: string; name: string; current_streak: number }>;
};

export type HabitDocument = {
  document_id: string;
  markdown_relative_path: string;
  markdown_body: string;
  habits: HabitEntry[];
  summary: HabitSummary;
  warnings: string[];
  layout_metadata: LayoutMetadata | null;
};

const mockHabitsStorageKey = "bentolife:mockHabits";

export async function readHabits(vaultPath: string, summaryDate?: string) {
  if (!isTauriRuntime()) {
    return mockHabitDocument(summaryDate ?? null);
  }

  const result = await safeInvoke("read_habits", { vaultPath, summaryDate: summaryDate ?? null }, normalizeHabitDocument, emptyHabitDocument());
  return result.data;
}

export async function createHabit(vaultPath: string, habit: HabitInput, summaryDate?: string) {
  if (!isTauriRuntime()) {
    return mockCreateHabit(habit, summaryDate ?? null);
  }

  return normalizeHabitDocument(await invoke<unknown>("create_habit", { vaultPath, habit, summaryDate: summaryDate ?? null }));
}

export async function updateHabit(vaultPath: string, habitId: string, habit: HabitInput, summaryDate?: string) {
  if (!isTauriRuntime()) {
    return mockUpdateHabit(habitId, habit, summaryDate ?? null);
  }

  return normalizeHabitDocument(await invoke<unknown>("update_habit", { vaultPath, habitId, habit, summaryDate: summaryDate ?? null }));
}

export async function recordHabitCheckin(vaultPath: string, habitId: string, date: string) {
  if (!isTauriRuntime()) {
    return mockRecordCheckin(habitId, date);
  }

  return normalizeHabitDocument(await invoke<unknown>("record_habit_checkin", { vaultPath, habitId, date }));
}

function mockCreateHabit(input: HabitInput, summaryDate: string | null): HabitDocument {
  const habits = readMockHabits();
  const habit = normalizeHabitInput(input, `habit_${Date.now().toString(36)}_${habits.length}`, []);
  writeMockHabits([...habits, habit]);
  return mockHabitDocument(summaryDate);
}

function mockUpdateHabit(habitId: string, input: HabitInput, summaryDate: string | null): HabitDocument {
  const habits = readMockHabits();
  const index = habits.findIndex((habit) => habit.habit_id === habitId);
  if (index < 0) {
    throw new Error("Habit was not found or was changed outside BentoLife.");
  }
  habits[index] = normalizeHabitInput(input, habitId, habits[index].checkins);
  writeMockHabits(habits);
  return mockHabitDocument(summaryDate);
}

function mockRecordCheckin(habitId: string, date: string): HabitDocument {
  validateDate(date);
  const habits = readMockHabits();
  const habit = habits.find((candidate) => candidate.habit_id === habitId);
  if (!habit) {
    throw new Error("Habit was not found or was changed outside BentoLife.");
  }
  if (!habit.checkins.includes(date)) {
    habit.checkins = [...habit.checkins, date].sort();
  }
  writeMockHabits(habits);
  return mockHabitDocument(date);
}

function mockHabitDocument(summaryDate: string | null): HabitDocument {
  if (summaryDate) {
    validateDate(summaryDate);
  }
  const habits = readMockHabits().map((habit, index) => ({
    ...habit,
    line_index: index * 8 + 2,
    raw_markdown: renderHabit(habit),
    parsed_entity: parsedHabitEntity(habit),
    schema_warnings: [],
  }));

  return {
    document_id: "bl_doc_mock_habits",
    markdown_relative_path: "modules/habits/INDEX.md",
    markdown_body: `# Habits\n\n${habits.map(renderHabit).join("\n")}`.trimEnd() + "\n",
    habits,
    summary: summarizeHabits(habits, summaryDate),
    warnings: [],
    layout_metadata: null,
  };
}

type MockHabit = Omit<HabitEntry, "line_index" | "raw_markdown" | "parsed_entity" | "schema_warnings">;

function readMockHabits(): MockHabit[] {
  const serialized = window.localStorage.getItem(mockHabitsStorageKey);
  if (!serialized) {
    return [];
  }
  try {
    return JSON.parse(serialized) as MockHabit[];
  } catch {
    return [];
  }
}

function writeMockHabits(habits: MockHabit[]) {
  window.localStorage.setItem(mockHabitsStorageKey, JSON.stringify(habits));
}

function normalizeHabitInput(input: HabitInput, habitId: string, checkins: string[]): MockHabit {
  const name = collapseText(input.name);
  if (!name) {
    throw new Error("Habit name is required.");
  }
  return {
    habit_id: habitId,
    name,
    frequency: cleanOptional(input.frequency),
    target: cleanOptional(input.target),
    tags: normalizeTags(input.tags ?? []),
    relationships: normalizeTags(input.relationships ?? []),
    notes: cleanOptionalMarkdown(input.notes),
    checkins: [...new Set(checkins)].sort(),
  };
}

function summarizeHabits(habits: MockHabit[], summaryDate: string | null): HabitSummary {
  const recent_checkins = habits
    .flatMap((habit) => habit.checkins.map((date) => ({ habit_id: habit.habit_id, name: habit.name, date })))
    .sort((left, right) => right.date.localeCompare(left.date) || left.name.localeCompare(right.name))
    .slice(0, 10);

  return {
    total: habits.length,
    summary_date: summaryDate,
    checked_in_on_date: summaryDate
      ? habits.filter((habit) => habit.checkins.includes(summaryDate)).length
      : 0,
    recent_checkins,
    streaks: habits.map((habit) => ({
      habit_id: habit.habit_id,
      name: habit.name,
      current_streak: currentStreak(habit.checkins, summaryDate),
    })),
  };
}

function renderHabit(habit: MockHabit) {
  const lines = [`## ${habit.name}`];
  if (habit.frequency) lines.push(`- Frequency: ${habit.frequency}`);
  if (habit.target) lines.push(`- Target: ${habit.target}`);
  if (habit.tags.length) lines.push(`- Tags: ${habit.tags.join(", ")}`);
  if (habit.relationships.length) lines.push(`- Relationships: ${habit.relationships.join(", ")}`);
  if (habit.notes) lines.push("### Notes", habit.notes);
  lines.push("### Check-ins");
  lines.push(...habit.checkins.map((date) => `- ${date}`));
  return `${lines.join("\n")}\n`;
}

function parsedHabitEntity(habit: MockHabit): ParsedEntityContract {
  const rawMarkdown = renderHabit(habit);
  const fields: Record<string, string> = {
    name: habit.name,
    title: habit.name,
  };
  if (habit.frequency) fields.frequency = habit.frequency;
  if (habit.target) fields.target = habit.target;
  if (habit.relationships.length) fields.relationships = habit.relationships.join(", ");
  if (habit.notes) fields.notes = habit.notes;
  if (habit.checkins.length) fields.checkins = habit.checkins.join(", ");
  return {
    module_id: "habits",
    entity_type: "habit",
    fields,
    field_descriptors: [
      { id: "name", label: "Name", type: "text", renderer_id: "text", value: habit.name, editable: false, aliases: ["title"], warnings: [] },
      { id: "frequency", label: "Frequency", type: "enum", renderer_id: "status", value: habit.frequency ?? "Daily", editable: false, aliases: [], options: ["Daily", "Weekly", "Monthly", "Custom"], default_value: "Daily", warnings: [] },
      { id: "target", label: "Target", type: "text", renderer_id: "text", value: habit.target ?? "", editable: false, aliases: [], warnings: [] },
      { id: "tags", label: "Tags", type: "tags", renderer_id: "tags", value: habit.tags.join(", "), editable: false, aliases: [], warnings: [] },
      { id: "relationships", label: "Relationships", type: "relationship", renderer_id: "relationships", value: habit.relationships.join(", "), editable: false, aliases: ["related"], warnings: [] },
      { id: "checkins", label: "Check-ins", type: "list", renderer_id: "generic", value: habit.checkins.join(", "), editable: false, aliases: ["check-ins"], warnings: [] },
    ],
    blocks: [{ type: "paragraph", text: rawMarkdown }],
    unknown_blocks: [],
    relationships: habit.relationships,
    tags: habit.tags,
    path: "modules/habits/INDEX.md",
    content_hash: habit.habit_id,
  };
}

function currentStreak(checkins: string[], summaryDate: string | null) {
  const days = new Set(checkins.map(dayNumber));
  let day = summaryDate ? dayNumber(summaryDate) : Math.max(...days);
  let streak = 0;
  while (days.has(day)) {
    streak += 1;
    day -= 1;
  }
  return streak;
}

function validateDate(date: string) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) {
    throw new Error("Habit check-in dates must use YYYY-MM-DD.");
  }
  const parsed = new Date(`${date}T00:00:00Z`);
  if (Number.isNaN(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== date) {
    throw new Error("Habit check-in date is not a valid calendar date.");
  }
}

function dayNumber(date: string) {
  validateDate(date);
  return Math.floor(new Date(`${date}T00:00:00Z`).getTime() / 86_400_000);
}

function normalizeTags(tags: string[]) {
  return [...new Set(tags.flatMap((tag) => tag.split(",")).map(collapseText).filter(Boolean))].sort();
}

function cleanOptional(value?: string | null) {
  const cleaned = collapseText(value ?? "");
  return cleaned || null;
}

function cleanOptionalMarkdown(value?: string | null) {
  const cleaned = (value ?? "")
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .join("\n")
    .trim();
  return cleaned || null;
}

function collapseText(value: string) {
  return value.trim().split(/\s+/).filter(Boolean).join(" ");
}
