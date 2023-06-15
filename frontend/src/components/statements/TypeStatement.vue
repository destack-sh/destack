<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import InlineActions from "@/components/statements/InlineActionsCell.vue";
import TypedDeclarationCell from "@/components/statements/TypedDeclarationCell.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { TypeTag } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { TypeFlag } from "@/state/module";
import { makeField, useStatementContext, type Field } from "@/state/statement";
import { generateKeyBetween } from "@/utils/fractional";
import { PlusIcon, SquaresPlusIcon } from "@heroicons/vue/24/outline";
import CubeTransparentIcon from "@heroicons/vue/24/outline/CubeTransparentIcon";
import { computed, nextTick, ref, type Ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof TypedDeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);

const isEnum = computed(() => context.typeRootTag.value == TypeTag.Enum);
const selfFields = computed(
  () => context.fields.value.filter((n) => !(n.flags & TypeFlag.IsUnionWith)).map((n) => n as Field) ?? []
);
const fieldsLength = computed(() => selfFields.value?.length ?? 0);
const baseTypes = computed(
  () => context.fields.value.filter((n) => n.flags & TypeFlag.IsUnionWith).map((n) => n as Field) ?? []
);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const addingDescription = ref(false);

// dynamic field refs for names, values & descriptions for each field
type ColumnType = "type";
const grid = useNavigationGrid<ColumnType, InstanceType<typeof FieldInterface>>(
  ref(["type"] as ColumnType[]),
  selfFields,
  {
    gridNavigateUp: focusDescriptionFromBottom,
    gridNavigateDown,
  }
);
const isEditing = computed(() => grid.refs.value.find((n) => n.editing));

function createOption() {
  const lastField = context.fields.value?.[context.fields.value.length - 1];
  const orderKey = generateKeyBetween(lastField?.orderKey ?? null, null);
  const name = "Option " + (fieldsLength.value + 1);
  const newField = makeField({
    name,
    tag: TypeTag.Literal,
    orderKey,
  });
  context.createField(newField);
  nextTick(() => grid.focus(newField.id, "type"));
}

function createUnionField() {
  context.createUnionField();
  nextTick(() => declarationRef.value?.focusLastBase());
}

function duplicateField(fieldId: string) {
  const newField = context.duplicateField(fieldId);
  if (newField != null) {
    nextTick(() => grid.focus(newField?.id, "type"));
  }
}

function deleteField(fieldId: string) {
  const fieldIdx = selfFields.value?.findIndex((m) => m.id === fieldId);
  if (fieldIdx == null || fieldIdx < 0) {
    return;
  }
  const field = selfFields.value?.[fieldIdx];
  context.deleteField(field as any); // must exist
  grid.focus(fieldIdx - 1, "type"); // move focus above
}

function moveField(node: Field, position: "before" | "after", other: Field) {
  const otherIndex = selfFields.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(selfFields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    context.moveField(node, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, selfFields.value[otherIndex + 1]?.orderKey ?? null);
    context.moveField(node, orderKey);
  }
}

function dropField(droppedId: string, position: "above" | "below", fieldId: string) {
  const dropped = selfFields.value.find((n) => n.id == droppedId);
  const field = selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  moveField(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
  nextTick(() => grid.focus(droppedId, "type"));
}

function writeType(fieldId: string, newType: Field) {
  const oldType = selfFields.value?.find((m) => m.id === fieldId);
  if (!oldType) return;
  context.updateField(oldType, { ...oldType, ...newType, id: fieldId });
}

function focusDescriptionFromTop() {
  if (description.value?.length > 0 || addingDescription.value) {
    descriptionRef.value?.focus();
  } else {
    focusFirstIfExists();
  }
}

function focusDescriptionFromBottom() {
  if (description.value?.length > 0 || addingDescription.value) {
    descriptionRef.value?.focus();
  } else {
    declarationRef.value?.focus();
  }
}

function focusFirstIfExists() {
  if (fieldsLength.value == 0) {
    addFieldRef.value?.focus();
  } else {
    grid.focus(0, "type");
  }
}

function focusLast() {
  if (fieldsLength.value > 0) {
    grid.focus(fieldsLength.value - 1, "type");
  } else {
    focusDescriptionFromBottom();
  }
}

function gridNavigateDown() {
  addFieldRef.value?.focus();
}

const extraInlineActions = computed(() => {
  const inlineActions: StatementAction[] = [];
  inlineActions.push({
    label: "Add " + (isEnum.value ? "option" : "field"),
    icon: SquaresPlusIcon,
    action: () => (isEnum.value ? createOption() : createFieldRef.value?.show()),
  });
  if (!isEnum.value) {
    inlineActions.push({
      label: "Extend type",
      icon: CubeTransparentIcon,
      action: () => createUnionField(),
    });
  }
  return inlineActions;
});

defineExpose({
  focus: (position: "first" | "last" = "first") =>
    position == "first" ? declarationRef.value?.focus() : addFieldRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
    addFieldRef.value?.blur();
    grid.blur();
  },
});
</script>
<template>
  <!-- Declaration -->
  <div class="flex flex-row justify-between">
    <div class="flex flex-row items-baseline">
      <TypedDeclarationCell
        ref="declarationRef"
        @navigate-down="focusDescriptionFromTop"
        @navigate-up="context.navigateUp"
      />
    </div>
    <div class="flex flex-row">
      <InlineActions
        class="transition duration-150 group-hover/statement:opacity-100"
        :class="context.focused.value ? '' : 'opacity-0'"
        :extraActions="extraInlineActions"
      />
      <CreateFieldInterface ref="createFieldRef" :title="'New field on ' + context.statement.value.name" />
    </div>
  </div>
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    :class="addingDescription ? '' : 'h-0'"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-left="declarationRef?.focus()"
    @navigate-up="declarationRef?.focus()"
    @navigate-down="focusFirstIfExists"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0 && !context.readonly.value && addingDescription"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
  >
    +description
  </button>
  <!-- Fields (enum options or struct fields) -->
  <div v-if="fieldsLength > 0" class="my-0.5 flex w-full flex-col gap-0.5">
    <FieldInterface
      v-for="field of selfFields"
      :key="field.id"
      :model-value="field"
      @update:model-value="(val: any) => writeType(field.id, val)"
      :ref="(el: any) => grid.registerColumnRef(field.id, 'type', el)"
      :readonly="context.readonly.value"
      :isEnum="isEnum"
      :tupleName="isEnum ? 'option' : 'field'"
      orientation="vertical"
      @navigate-left="grid.navigateLeft(field.id, 'type')"
      @navigate-right="grid.navigateRight(field.id, 'type')"
      @navigate-up="grid.navigateUp(field.id, 'type')"
      @navigate-down="grid.navigateDown(field.id, 'type')"
      @delete-self="deleteField(field.id)"
      @duplicate-self="duplicateField(field.id)"
      @keydown.delete.exact="isEditing || deleteField(field.id)"
      @drop="(p, v) => dropField(v.id, p, field.id)"
      @enter="grid.navigateDown(field.id, 'type')"
      class="-mx-1 self-start px-1 py-0.5 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
      :class="isEnum ? 'w-fit' : 'w-full '"
    />
  </div>
  <div class="mb-1">
    <!-- Add a field -->
    <button
      v-show="!context.readonly.value"
      tabindex="-1"
      ref="addFieldRef"
      class="mt-1 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="isEnum ? createOption() : createFieldRef?.show()"
      @enter="isEnum ? createOption() : createFieldRef?.show()"
      @keydown.up.exact.prevent="focusLast"
      @keydown.down.exact.prevent="context.navigateDown"
    >
      <PlusIcon class="h-4 w-4" />{{ isEnum ? "Option" : "Field" }}
    </button>
  </div>
</template>
