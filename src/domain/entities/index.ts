export type {
  EntityDefinition,
  EntityFieldDefinition,
  EntityFieldRenderer,
  EntityFieldVisibility,
  EntityId,
  RelationshipPolicy,
} from "@/domain/entities/types";
export { TODO_PRIORITY_OPTIONS, TODO_STATUS_OPTIONS } from "@/domain/entities/todos.entity";
export { CONTACT_RELATIONSHIP_OPTIONS } from "@/domain/entities/contacts.entity";
export { HABIT_FREQUENCY_OPTIONS } from "@/domain/entities/habits.entity";

import { contactsEntity } from "@/domain/entities/contacts.entity";
import { habitsEntity } from "@/domain/entities/habits.entity";
import { notesEntity } from "@/domain/entities/notes.entity";
import { todosEntity } from "@/domain/entities/todos.entity";
import type { EntityDefinition, EntityId } from "@/domain/entities/types";

export const entityDefinitions = {
  note: notesEntity,
  todo: todosEntity,
  contact: contactsEntity,
  habit: habitsEntity,
} satisfies Record<EntityId, EntityDefinition>;

function getEntityDefinition(entityId: EntityId) {
  return entityDefinitions[entityId];
}

export function entityFieldOptions(entityId: EntityId, fieldId: string) {
  return getEntityDefinition(entityId).fields[fieldId]?.options ?? [];
}
