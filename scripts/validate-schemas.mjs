import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { readdirSync, statSync } from "node:fs";

const repoRoot = join(fileURLToPath(new URL("..", import.meta.url)));
const schemasRoot = join(repoRoot, "schemas");
const expectedSchemaFiles = [
  "modules/notes.schema.json",
  "modules/todos.schema.json",
  "modules/contacts.schema.json",
  "modules/habits.schema.json",
  "modules/module.schema.v2.schema.json",
  "widgets/dashboard-widget.schema.json",
  "metadata/module-registry.schema.json",
  "metadata/workspace-ui-state.schema.json",
  "metadata/dashboard-widgets.schema.json",
  "metadata/layout-metadata.schema.json",
  "metadata/theme-state.schema.json",
];

function jsonFiles(root) {
  const entries = readdirSync(root, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const fullPath = join(root, entry.name);
    if (entry.isDirectory()) {
      return jsonFiles(fullPath);
    }
    return entry.isFile() && entry.name.endsWith(".json") ? [fullPath] : [];
  });
}

if (!statSync(schemasRoot, { throwIfNoEntry: false })?.isDirectory()) {
  throw new Error("schemas/ directory does not exist.");
}

const files = jsonFiles(schemasRoot);
if (!files.length) {
  throw new Error("schemas/ does not contain any JSON files.");
}

for (const relativePath of expectedSchemaFiles) {
  if (!statSync(join(schemasRoot, relativePath), { throwIfNoEntry: false })?.isFile()) {
    throw new Error(`Expected schema file is missing: schemas/${relativePath}`);
  }
}

for (const file of files) {
  const content = await readFile(file, "utf8");
  JSON.parse(content);
}

console.log(`Validated ${files.length} schema JSON file(s).`);
