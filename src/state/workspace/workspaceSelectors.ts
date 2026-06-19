import type { AppView, FocusTarget } from "@/state/navigation";
import type { WorkspaceResourceErrors } from "@/state/workspace/workspaceResources";
import type { ContactDocument, ContactEntry } from "@/services/contacts";
import type { DashboardWidgetState } from "@/services/widgets";
import type { HabitDocument, HabitEntry } from "@/services/habits";
import type { NoteDocument, NoteSummary, WorkspaceScanResult } from "@/services/notes";
import type { TodoDocument, TodoSummary } from "@/services/todo";

export function selectActiveModuleId(activeView: AppView, focusTarget: FocusTarget): string | null {
  return activeView === "module" ? focusTarget.moduleId ?? null : viewToModuleId(activeView);
}

export function selectTodoSummaries(todos: TodoSummary[] | null | undefined): TodoSummary[] {
  return todos ?? [];
}

export function selectContactDocument(document: ContactDocument | null | undefined): ContactDocument | null {
  return document ?? null;
}

export function selectHabitDocument(document: HabitDocument | null | undefined): HabitDocument | null {
  return document ?? null;
}

export function selectDashboardWidgets(state: DashboardWidgetState | null | undefined): DashboardWidgetState | null {
  return state ?? null;
}

export function selectWorkspaceWarnings(scan: WorkspaceScanResult | null | undefined): string[] {
  return scan?.issues
    .filter((issue) => issue.classification === "schema_warning" || issue.classification === "preserved_unknown_content")
    .map((issue) => issue.message)
    .filter(Boolean) ?? [];
}

export function selectModuleErrors(errors: WorkspaceResourceErrors, workspaceError: string | null = null) {
  return {
    contacts: errors.contacts ?? workspaceError,
    dashboard: errors.dashboardHub ?? errors.dashboardWidgets ?? workspaceError,
    habits: errors.habits ?? workspaceError,
    navigator: errors.navigator ?? workspaceError,
    notes: errors.notes ?? workspaceError,
    todos: errors.todos ?? workspaceError,
  };
}

export function selectContactById(document: ContactDocument | null | undefined, contactId: string | null): ContactEntry | null {
  return document?.contacts.find((contact) => contact.contact_id === contactId) ?? null;
}

export function selectHabitById(document: HabitDocument | null | undefined, habitId: string | null): HabitEntry | null {
  return document?.habits.find((habit) => habit.habit_id === habitId) ?? null;
}

export function resolveSelectedTodoId(
  todos: TodoSummary[],
  currentTodo: TodoDocument | null,
  nextDocumentId: string | null,
  preferredNoteId?: string,
): string | null {
  const preferredTodoId = preferredNoteId && todos.some((todo) => todo.document_id === preferredNoteId) ? preferredNoteId : null;
  return (
    preferredTodoId ??
    currentTodo?.document_id ??
    todos.find((todo) => todo.document_id === nextDocumentId)?.document_id ??
    todos[0]?.document_id ??
    null
  );
}

export function resolveSelectedNoteId(
  notes: NoteSummary[],
  currentNote: NoteDocument | null,
  nextDocumentId: string | null,
  preferredNoteId?: string,
): string | null {
  const preferredSelectedNoteId = preferredNoteId && notes.some((note) => note.document_id === preferredNoteId) ? preferredNoteId : null;
  return (
    preferredSelectedNoteId ??
    currentNote?.document_id ??
    notes.find((note) => note.document_id === nextDocumentId)?.document_id ??
    notes[0]?.document_id ??
    null
  );
}

export function resolveSelectedContactId(document: ContactDocument, currentContactId: string | null): string | null {
  return currentContactId && document.contacts.some((contact) => contact.contact_id === currentContactId)
    ? currentContactId
    : document.contacts[0]?.contact_id ?? null;
}

export function resolveSelectedHabitId(document: HabitDocument, currentHabitId: string | null): string | null {
  return currentHabitId && document.habits.some((habit) => habit.habit_id === currentHabitId)
    ? currentHabitId
    : document.habits[0]?.habit_id ?? null;
}

function viewToModuleId(view: AppView): string | null {
  switch (view) {
    case "notes":
    case "todos":
    case "contacts":
    case "habits":
    case "navigator":
      return view;
    case "trash":
    case "archive":
    case "vault":
    case "architect":
    case "settings":
      return view;
    case "dashboard":
    case "module":
      return null;
  }
}
