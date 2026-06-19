import { BookOpen, CheckCircle2, ClipboardList, Copy, Flame, Pin, Tag, Users } from "lucide-react";
import { memo, type ComponentType } from "react";

import { Button } from "@/components/ui/button";
import {
  type WidgetDensity,
  WidgetContentScrollArea,
  WidgetEmptyIllustration,
  WidgetEmptyState,
  WidgetFrame,
  WidgetList,
  WidgetProgress,
  WidgetRow,
  WidgetSurface,
  WidgetTimeline,
  widgetDensity,
} from "@/components/widgets/WidgetPrimitives";
import { useI18n } from "@/i18n";
import type { ContactDocument } from "@/services/contacts";
import type { HabitDocument } from "@/services/habits";
import type { NoteSummary } from "@/services/notes";
import type { TodoDocument, TodoSummary } from "@/services/todo";
import { dashboardWidgetTitle, type DashboardWidgetInstance, type WidgetTypeDefinition } from "@/services/widgets";

export type WidgetRenderContext = {
  notes: NoteSummary[];
  todoSummaries: TodoSummary[];
  todos: TodoDocument | null;
  contacts: ContactDocument | null;
  habits: HabitDocument | null;
};

export type WidgetInteractionHandlers = {
  openEntity: (target: {
    moduleId: string;
    documentId?: string;
    entityId?: string;
    label?: string;
  }) => void;
  toggleTodoComplete?: (documentId: string, completed: boolean) => Promise<void>;
  recordHabitCheckin?: (habitId: string) => Promise<void>;
  copyText?: (value: string, label?: string) => Promise<void>;
  refreshWidgetData?: () => Promise<void>;
};

export type WidgetRendererProps = {
  actions: WidgetInteractionHandlers;
  context: WidgetRenderContext;
  density: WidgetDensity;
  instance: DashboardWidgetInstance;
  size: { width: number; height: number };
  widgetType?: WidgetTypeDefinition;
  warning?: string;
};

export type WidgetRendererDefinition = {
  id: string;
  render: ComponentType<WidgetRendererProps>;
};

const noopInteractions: WidgetInteractionHandlers = {
  openEntity: () => {},
};

const GenericWidget = memo(({ instance, widgetType, warning }: WidgetRendererProps) => {
  const { t } = useI18n();
  return (
    <WidgetSurface>
      <WidgetEmptyIllustration>
        <BookOpen className="size-4" aria-hidden="true" />
      </WidgetEmptyIllustration>
      <WidgetEmptyState
        title={dashboardWidgetTitle(instance, widgetType)}
        description={warning ?? t("widgets.generic.unrecognized")}
      />
    </WidgetSurface>
  );
});
GenericWidget.displayName = "GenericWidget";

const RecentNotesWidget = memo(({ actions, context, density, size }: WidgetRendererProps) => {
  const { t } = useI18n();
  const notes = context.notes;
  if (!notes.length) {
    return (
      <WidgetEmptyState
        title={t("widgets.empty.notes")}
        description={t("widgets.empty.notes.description")}
        illustration={<BookOpen className="size-4" aria-hidden="true" />}
      />
    );
  }
  return (
    <WidgetSurface>
      <WidgetList
        maxRows={visibleRowsForDensity(density, size)}
        items={notes.map((note) => ({
          detail: density === "expanded" ? note.excerpt || note.markdown_relative_path : note.markdown_relative_path,
          label: note.title,
          leading: <BookOpen className="size-4 text-muted-foreground" aria-hidden="true" />,
          onClick: () => actions.openEntity({ moduleId: "notes", documentId: note.document_id, label: note.title }),
        }))}
      />
    </WidgetSurface>
  );
});
RecentNotesWidget.displayName = "RecentNotesWidget";

const PinnedNotesWidget = memo(({ actions, context, density, size }: WidgetRendererProps) => {
  const { t } = useI18n();
  const notes = context.notes;
  if (!notes.length) {
    return (
      <WidgetEmptyState
        title={t("widgets.empty.pinned")}
        description={t("widgets.empty.pinned.description")}
        illustration={<Pin className="size-4" aria-hidden="true" />}
      />
    );
  }
  return (
    <WidgetFrame>
      <WidgetList
        maxRows={visibleRowsForDensity(density, size)}
        items={notes.map((note) => ({
          detail: note.excerpt || note.markdown_relative_path,
          label: note.title,
          leading: <Pin className="size-4 text-muted-foreground" aria-hidden="true" />,
          onClick: () => actions.openEntity({ moduleId: "notes", documentId: note.document_id, label: note.title }),
        }))}
      />
    </WidgetFrame>
  );
});
PinnedNotesWidget.displayName = "PinnedNotesWidget";

const NotesByTagWidget = memo(({ actions, context, density, instance, size }: WidgetRendererProps) => {
  const { t } = useI18n();
  const tag = String(instance.config.tag ?? "daily").toLowerCase().replace(/^#/, "");
  const notes = context.notes.filter((note) => `${note.title} ${note.excerpt}`.toLowerCase().includes(tag));
  if (!notes.length) {
    return <WidgetEmptyState title={t("widgets.empty.tag")} description={`#${tag}`} illustration={<Tag className="size-4" aria-hidden="true" />} />;
  }
  return (
    <WidgetFrame>
      <WidgetList
        maxRows={visibleRowsForDensity(density, size)}
        items={notes.map((note) => ({
          detail: note.excerpt,
          label: note.title,
          leading: <Tag className="size-4 text-muted-foreground" aria-hidden="true" />,
          onClick: () => actions.openEntity({ moduleId: "notes", documentId: note.document_id, label: note.title }),
        }))}
      />
    </WidgetFrame>
  );
});
NotesByTagWidget.displayName = "NotesByTagWidget";

const TodoListWidget = memo(({ actions, context, density, instance, widgetType }: WidgetRendererProps) => {
  const { t } = useI18n();
  const lowerTitle = dashboardWidgetTitle(instance, widgetType).toLowerCase();
  const isOverdue = lowerTitle.includes("overdue");
  const isUpcoming = lowerTitle.includes("upcoming");
  const todos = context.todoSummaries.filter((todo) => !todo.is_completed);
  if (!todos.length) {
    return (
      <WidgetEmptyState
        title={t("widgets.empty.todos")}
        description={t("widgets.empty.todos.description")}
        illustration={<ClipboardList className="size-4" aria-hidden="true" />}
      />
    );
  }
  const completeAction = (todo: TodoSummary) => (
    <Button
      aria-label={`${t("widgets.todo.complete")} ${todo.title}`}
      onClick={(event) => {
        event.stopPropagation();
        void actions.toggleTodoComplete?.(todo.document_id, true);
      }}
      size="icon"
      type="button"
      variant="outline"
    >
      <CheckCircle2 className="size-4" aria-hidden="true" />
    </Button>
  );
  return (
    <WidgetFrame>
      {isUpcoming && density === "expanded" ? (
        <WidgetContentScrollArea maxRows={visibleRowsForDensity(density, { width: 4, height: 2 })}>
          <WidgetTimeline
            items={todos.map((todo) => ({
              date: todo.updated_at ? `${t("widgets.labels.updated")} ${todo.updated_at.slice(0, 10)}` : t("widgets.labels.noDate"),
              label: todo.title,
              onClick: () => actions.openEntity({ moduleId: "todos", documentId: todo.document_id, label: todo.title }),
            }))}
          />
        </WidgetContentScrollArea>
      ) : (
        <WidgetList
          maxRows={visibleRowsForDensity(density)}
          items={todos.map((todo) => ({
            action: completeAction(todo),
            detail: todo.excerpt,
            label: todo.title,
            leading: <ClipboardList className="size-4 text-muted-foreground" aria-hidden="true" />,
            onClick: () => actions.openEntity({ moduleId: "todos", documentId: todo.document_id, label: todo.title }),
            tone: isOverdue ? "warning" : "default",
          }))}
        />
      )}
    </WidgetFrame>
  );
});
TodoListWidget.displayName = "TodoListWidget";

const HabitCheckinWidget = memo(({ actions, context, density, size }: WidgetRendererProps) => {
  const { t } = useI18n();
  const today = context.habits?.summary.summary_date ?? localDateKey();
  const habits = context.habits?.habits ?? [];
  if (!habits.length) {
    return (
      <WidgetEmptyState
        title={t("widgets.empty.habits")}
        description={t("widgets.empty.habits.description")}
        illustration={<Flame className="size-4" aria-hidden="true" />}
      />
    );
  }
  return (
    <WidgetFrame>
      <WidgetList
        maxRows={visibleRowsForDensity(density, size)}
        items={habits.map((habit) => {
          const checked = habit.checkins.includes(today);
          return {
            action: (
              <Button
                disabled={checked}
                onClick={(event) => {
                  event.stopPropagation();
                  void actions.recordHabitCheckin?.(habit.habit_id);
                }}
                size="sm"
                type="button"
                variant={checked ? "ghost" : "outline"}
              >
                {checked ? t("widgets.habit.done") : t("widgets.actions.checkIn")}
              </Button>
            ),
            detail: habit.frequency ?? t("widgets.labels.habit"),
            label: habit.name,
            leading: <Flame className="size-4 text-muted-foreground" aria-hidden="true" />,
            onClick: () => actions.openEntity({ moduleId: "habits", entityId: habit.habit_id, label: habit.name }),
          };
        })}
      />
    </WidgetFrame>
  );
});
HabitCheckinWidget.displayName = "HabitCheckinWidget";

const ProgressWidget = memo(({ actions, context, density }: WidgetRendererProps) => {
  const { t } = useI18n();
  const total = context.habits?.summary.total ?? 0;
  const checked = context.habits?.summary.checked_in_on_date ?? 0;
  const percent = total ? (checked / total) * 100 : 0;
  return (
    <WidgetFrame>
      <WidgetProgress label={`${checked} / ${total} ${t("widgets.labels.habitsCheckedToday")}`} percent={percent} />
      {density === "expanded" && context.habits?.summary.streaks.length ? (
        <WidgetList
          maxRows={4}
          items={context.habits.summary.streaks.slice(0, 4).map((streak) => ({
            detail: `${streak.current_streak} ${t("widgets.labels.dayStreak")}`,
            label: streak.name,
            leading: <Flame className="size-4 text-muted-foreground" aria-hidden="true" />,
            onClick: () => actions.openEntity({ moduleId: "habits", entityId: streak.habit_id, label: streak.name }),
          }))}
        />
      ) : null}
    </WidgetFrame>
  );
});
ProgressWidget.displayName = "ProgressWidget";

const RecentContactsWidget = memo(({ actions, context, density, size }: WidgetRendererProps) => {
  const { t } = useI18n();
  const contacts = context.contacts?.contacts ?? [];
  if (!contacts.length) {
    return (
      <WidgetEmptyState
        title={t("widgets.empty.contacts")}
        description={t("widgets.empty.contacts.description")}
        illustration={<Users className="size-4" aria-hidden="true" />}
      />
    );
  }
  return (
    <WidgetFrame>
      <WidgetContentScrollArea maxRows={visibleRowsForDensity(density, size)}>
        <div className="flex flex-col gap-2">
          {contacts.map((contact) => (
            <WidgetRow
              action={
                contact.email || contact.phone ? (
                  <Button
                    aria-label={`${t("widgets.actions.copy")} ${contact.name}`}
                    onClick={(event) => {
                      event.stopPropagation();
                      void actions.copyText?.(contact.email ?? contact.phone ?? "", contact.name);
                    }}
                    size="sm"
                    type="button"
                    variant="ghost"
                  >
                    <Copy className="size-4" aria-hidden="true" />
                  </Button>
                ) : undefined
              }
              detail={contact.organization ?? contact.relationship ?? t("widgets.labels.contact")}
              key={contact.contact_id}
              label={contact.name}
              leading={
                <span className="flex size-8 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-semibold text-primary">
                  {initials(contact.name)}
                </span>
              }
              onClick={() => actions.openEntity({ moduleId: "contacts", entityId: contact.contact_id, label: contact.name })}
            />
          ))}
        </div>
      </WidgetContentScrollArea>
    </WidgetFrame>
  );
});
RecentContactsWidget.displayName = "RecentContactsWidget";

const WidgetRendererRegistry: Record<string, WidgetRendererDefinition> = {
  generic_widget: { id: "generic_widget", render: GenericWidget },
  recent_notes: { id: "recent_notes", render: RecentNotesWidget },
  pinned_notes: { id: "pinned_notes", render: PinnedNotesWidget },
  notes_by_tag: { id: "notes_by_tag", render: NotesByTagWidget },
  todo_list: { id: "todo_list", render: TodoListWidget },
  habit_checkin: { id: "habit_checkin", render: HabitCheckinWidget },
  progress: { id: "progress", render: ProgressWidget },
  recent_contacts: { id: "recent_contacts", render: RecentContactsWidget },
};

function getWidgetRenderer(id?: string): WidgetRendererDefinition {
  return (id && WidgetRendererRegistry[id]) || WidgetRendererRegistry.generic_widget;
}

export function renderWidget(
  instance: DashboardWidgetInstance,
  widgetType: WidgetTypeDefinition | undefined,
  context: WidgetRenderContext,
  actions: WidgetInteractionHandlers = noopInteractions,
) {
  const renderer = getWidgetRenderer(widgetType?.renderer_id);
  const Renderer = renderer.render;
  const size = {
    width: Math.max(1, Math.min(7, instance.layout.width)),
    height: Math.max(1, Math.min(3, instance.layout.height)),
  };
  const warning = widgetType
    ? renderer.id === "generic_widget"
      ? undefined
      : undefined
    : undefined;
  return (
    <Renderer
      actions={actions}
      context={context}
      density={widgetDensity(size)}
      instance={instance}
      size={size}
      warning={warning}
      widgetType={widgetType}
    />
  );
}

function initials(name: string) {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  return (parts[0]?.[0] ?? "?").concat(parts[1]?.[0] ?? "").toUpperCase();
}

function visibleRowsForDensity(density: WidgetDensity, size?: { width: number; height: number }) {
  if (density === "compact") return size?.width && size.width > 1 ? 2 : 1;
  if (density === "expanded") return size?.width && size.width >= 4 ? 6 : 5;
  return 4;
}

function localDateKey(date = new Date()) {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}
