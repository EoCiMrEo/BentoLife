import { invoke } from "@tauri-apps/api/core";

import { safeInvoke } from "@/services/contracts/invokeClient";
import { normalizeTodoDocument, normalizeTodoSummaries } from "@/services/contracts/todos.contract";
import type { LayoutMetadata } from "@/services/notes";
import type { ParsedEntityContract, ScannedDocumentStatus } from "@/services/backendCore";
import { isTauriRuntime } from "@/services/vault";

export type TodoSummary = {
  document_id: string;
  title: string;
  markdown_relative_path: string;
  excerpt: string;
  is_completed: boolean;
  status: ScannedDocumentStatus;
  updated_at: string | null;
};

export type TodoDocument = {
  document_id: string;
  title: string;
  markdown_relative_path: string;
  markdown_body: string;
  parsed_entity: ParsedEntityContract;
  schema_warnings: string[];
  document_metadata: any;
  layout_metadata: LayoutMetadata | null;
};

const mockTodosStorageKey = "bentolife:mockTodos";

export async function listTodos(vaultPath: string) {
  if (!isTauriRuntime()) {
    return readMockTodos().map(toTodoSummary);
  }
  const result = await safeInvoke("list_todos", { vaultPath }, normalizeTodoSummaries, []);
  return result.data;
}

export async function readTodo(vaultPath: string, documentId: string) {
  if (!isTauriRuntime()) {
    const todos = readMockTodos().find((candidate) => candidate.document_id === documentId);
    if (!todos) throw new Error("Todos was not found.");
    return toTodoDocument(todos);
  }
  return normalizeTodoDocument(await invoke<unknown>("read_todo", { vaultPath, documentId }));
}

export async function createTodo(vaultPath: string, title: string, markdownBody?: string) {
  if (!isTauriRuntime()) {
    const todos = readMockTodos();
    const cleanTitle = title.trim() || "Untitled task";
    const todo: MockTodo = {
      document_id: `bl_doc_mock_todo_${Date.now().toString(36)}_${todos.length}`,
      title: cleanTitle,
      markdown_relative_path: uniqueTodoPath(todos, cleanTitle),
      markdown_body: normalizeTodoBody(cleanTitle, markdownBody ?? ""),
      updated_at: new Date().toISOString(),
    };
    writeMockTodos([...todos, todo]);
    return toTodoDocument(todo);
  }
  return normalizeTodoDocument(await invoke<unknown>("create_todo", { vaultPath, title, markdownBody }));
}

export async function updateTodo(vaultPath: string, documentId: string, markdownBody: string) {
  if (!isTauriRuntime()) {
    const todos = readMockTodos();
    const todo = todos.find((candidate) => candidate.document_id === documentId);
    if (!todo) throw new Error("Todos was not found.");
    todo.markdown_body = markdownBody.trim() ? `${markdownBody.trim()}\n` : `# ${todo.title}\n`;
    todo.title = titleFromMarkdown(todo.markdown_body, todo.title);
    todo.updated_at = new Date().toISOString();
    writeMockTodos(todos);
    return toTodoDocument(todo);
  }
  return normalizeTodoDocument(await invoke<unknown>("update_todo", { vaultPath, documentId, markdownBody }));
}

export async function renameTodo(vaultPath: string, documentId: string, newTitle: string) {
  if (!isTauriRuntime()) {
    const todos = readMockTodos();
    const todo = todos.find((candidate) => candidate.document_id === documentId);
    if (!todo) throw new Error("Todos was not found.");
    todo.title = newTitle.trim() || "Untitled task";
    todo.markdown_relative_path = uniqueTodoPath(todos.filter((candidate) => candidate.document_id !== documentId), todo.title);
    todo.markdown_body = replaceOrInsertTitle(todo.markdown_body, todo.title);
    todo.updated_at = new Date().toISOString();
    writeMockTodos(todos);
    return toTodoDocument(todo);
  }
  return normalizeTodoDocument(await invoke<unknown>("rename_todo", { vaultPath, documentId, newTitle }));
}

type MockTodo = {
  document_id: string;
  title: string;
  markdown_relative_path: string;
  markdown_body: string;
  updated_at: string | null;
};

function readMockTodos(): MockTodo[] {
  try {
    return JSON.parse(window.localStorage.getItem(mockTodosStorageKey) ?? "[]") as MockTodo[];
  } catch {
    return [];
  }
}

function writeMockTodos(todos: MockTodo[]) {
  window.localStorage.setItem(mockTodosStorageKey, JSON.stringify(todos));
}

function toTodoSummary(todos: MockTodo): TodoSummary {
  const status = parseFields(todos.markdown_body).status ?? "";
  return {
    document_id: todos.document_id,
    title: todos.title,
    markdown_relative_path: todos.markdown_relative_path,
    excerpt: todos.markdown_body.split(/\r?\n/).find((line) => line.trim() && !line.startsWith("#")) ?? "No preview yet.",
    is_completed: isCompletedStatus(status),
    status: "managed",
    updated_at: todos.updated_at,
  };
}

function toTodoDocument(todos: MockTodo): TodoDocument {
  const fields = parseFields(todos.markdown_body);
  return {
    document_id: todos.document_id,
    title: todos.title,
    markdown_relative_path: todos.markdown_relative_path,
    markdown_body: todos.markdown_body,
    parsed_entity: {
      module_id: "todos",
      entity_type: "todos",
      fields: { title: todos.title, ...fields },
      field_descriptors: Object.entries({ title: todos.title, ...fields }).map(([id, value]) => ({
        id,
        label: id.replace(/_/g, " ").replace(/\b\w/g, (character) => character.toUpperCase()),
        type: "text",
        renderer_id: id === "status" || id === "priority" ? "status" : id.includes("date") ? "date" : "text",
        value: id === "status" && !value ? "Not started" : id === "priority" && !value ? "Medium" : value,
        editable: false,
        aliases: [],
        options: id === "status"
          ? ["Not started", "In progress", "Waiting", "Done"]
          : id === "priority"
            ? ["Low", "Medium", "High", "Urgent"]
            : [],
        default_value: id === "status" ? "Not started" : id === "priority" ? "Medium" : null,
        warnings: [],
      })),
      blocks: parseBlocks(todos.markdown_body),
      unknown_blocks: [],
      relationships: fields.relationships ? fields.relationships.split(",").map((item) => item.trim()).filter(Boolean) : [],
      tags: fields.tags ? fields.tags.split(",").map((item) => item.trim()).filter(Boolean) : [],
      path: todos.markdown_relative_path,
      content_hash: "mock",
    },
    schema_warnings: Object.keys(fields)
      .filter((field) => !["status", "priority", "due date", "due_date", "tags", "relationships", "related"].includes(field))
      .map((field) => `Unknown todos field '${field}' is preserved as fallback content.`),
    document_metadata: null,
    layout_metadata: null,
  };
}

function parseFields(markdown: string) {
  const fields: Record<string, string> = {};
  for (const line of markdown.split(/\r?\n/)) {
    const match = line.match(/^([A-Za-z][A-Za-z0-9 _/-]*):\s*(.+)$/);
    if (match) fields[match[1].trim().toLowerCase()] = match[2].trim();
  }
  return fields;
}

function parseBlocks(markdown: string): TodoDocument["parsed_entity"]["blocks"] {
  return markdown.split(/\r?\n/).filter(Boolean).map((line) => {
    if (line.startsWith("# ")) return { type: "heading", level: 1, text: line.replace(/^#\s+/, "") };
    const checklist = line.match(/^[-*]\s+\[([ xX])\]\s+(.+)$/);
    if (checklist) return { type: "checklist", items: [{ checked: checklist[1].toLowerCase() === "x", text: checklist[2] }] };
    return { type: "paragraph", text: line };
  });
}

function isCompletedStatus(status: string) {
  return ["done", "completed"].includes(status.trim().toLowerCase());
}

function normalizeTodoBody(title: string, markdownBody: string) {
  const body = markdownBody.trim();
  if (!body) return `# ${title}\n\nStatus: Not started\nPriority: Medium\n`;
  if (body.split(/\r?\n/).some((line) => line.trimStart().startsWith("# "))) return `${body}\n`;
  return `# ${title}\n\n${body}\n`;
}

function titleFromMarkdown(markdownBody: string, fallback: string) {
  return markdownBody.split(/\r?\n/).find((line) => line.startsWith("# "))?.replace(/^#\s+/, "").trim() || fallback;
}

function replaceOrInsertTitle(markdownBody: string, title: string) {
  const lines = markdownBody.trim().split(/\r?\n/);
  const index = lines.findIndex((line) => line.startsWith("# "));
  if (index >= 0) lines[index] = `# ${title}`;
  else lines.unshift(`# ${title}`);
  return `${lines.join("\n")}\n`;
}

function uniqueTodoPath(todos: MockTodo[], title: string) {
  const base = slugify(title);
  let candidate = `modules/todos/data/${base}.md`;
  let index = 1;
  while (todos.some((todos) => todos.markdown_relative_path === candidate)) {
    candidate = `modules/todos/data/${base}-${index}.md`;
    index += 1;
  }
  return candidate;
}

function slugify(value: string) {
  return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "untitled-task";
}
