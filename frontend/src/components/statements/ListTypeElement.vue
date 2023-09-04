<script lang="ts" setup>
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { TypeTag } from "@/gql/graphql";
import { makeField, useFields } from "@/state/statement";
import { generateKeyBetween } from "@/utils/fractional";
import { Bars3Icon, PlusIcon, SquaresPlusIcon, TagIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, toRef, type Ref } from "vue";
import { useCurrentModule, type Field } from "@/state/module";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useOperations } from "@/state/operations";

const props = defineProps<Pick<StatementProps, "statement" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const module = useCurrentModule();
const ops = useOperations();

const isEnum = computed(() => props.statement.tag == TypeTag.Enum);
const { fields, selfFields, duplicateField } = useFields(toRef(props, "statement"));
const fieldsLength = computed(() => selfFields.value?.length ?? 0);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);

// dynamic field refs for names, values & texts for each field
type ColumnType = "type";
const grid = useNavigationGrid<ColumnType, InstanceType<typeof FieldInterface>>(
  ref(["type"] as ColumnType[]),
  selfFields,
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown,
  }
);
const isEditing = computed(() => grid.refs.value.find((n) => n.editing));

function nextOrderKey() {
  const lastField = fields.value?.[fields.value.length - 1];
  const orderKey = generateKeyBetween(lastField?.orderKey ?? null, null);
  return orderKey;
}

function createOption() {
  const name = "Option " + (fieldsLength.value + 1);
  const newField = makeField({
    projectVersionId: module.id.value,
    name,
    tag: TypeTag.Literal,
    orderKey: nextOrderKey(),
  });
  ops.symbol.createField(null, props.statement.id, newField);
  nextTick(() => {
    grid.getRef(newField.id, "type").open("all");
  });
}

function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "metadata"> & Partial<Field>) {
  const field = makeField({ ...template, orderKey: nextOrderKey() });
  ops.symbol.createField(null, props.statement.id, field);
  nextTick(() => {
    grid.getRef(field.id, "type").open("all");
  });
}

function duplicateFieldAndFocus(fieldId: string) {
  const newField = duplicateField(fieldId);
  if (newField != null) {
    nextTick(() => {
      grid.getRef(newField?.id, "type").open("all");
    });
  }
}

function deleteField(fieldId: string) {
  const fieldIdx = selfFields.value?.findIndex((m) => m.id === fieldId);
  if (fieldIdx == null || fieldIdx < 0) {
    return;
  }
  const field = selfFields.value?.[fieldIdx];
  ops.symbol.softDeleteField(null, props.statement.id, field.id); // must exist
  grid.focus(fieldIdx - 1, "type"); // move focus above
}

function moveField(field: Field, position: "before" | "after", other: Field) {
  const otherIndex = selfFields.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(selfFields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    ops.symbol.moveField(null, field.id, field.orderKey, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, selfFields.value[otherIndex + 1]?.orderKey ?? null);
    ops.symbol.moveField(null, field.id, field.orderKey, orderKey);
  }
}

function dropField(droppedId: string, position: "above" | "below" | "left" | "right", fieldId: string) {
  const dropped = selfFields.value.find((n) => n.id == droppedId);
  const field = selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  moveField(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
  nextTick(() => grid.focus(droppedId, "type"));
}

function writeType(fieldId: string, newType: Field) {
  const oldType = selfFields.value?.find((m) => m.id === fieldId);
  if (!oldType) return;
  ops.symbol.updateField(null, oldType, { ...oldType, ...newType, id: fieldId });
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
    emit("navigateDown");
  }
}

function gridNavigateDown() {
  addFieldRef.value?.focus();
}

defineExpose({
  focus: (position: "first" | "last" = "first") =>
    position == "first" ? focusFirstIfExists() : addFieldRef.value?.focus(),
  blur: () => {
    addFieldRef.value?.blur();
    grid.blur();
  },
});
</script>
<template>
  <!-- Fields (enum options or struct fields) -->
  <div>
    <FieldInterface
      v-for="field of selfFields"
      :key="field.id"
      :model-value="field"
      @update:model-value="(val: any) => writeType(field.id, val)"
      :ref="(el: any) => grid.registerColumnRef(field.id, 'type', el)"
      :readonly="readonly"
      :isEnum="isEnum"
      :tupleName="isEnum ? 'option' : 'field'"
      orientation="vertical"
      @navigate-left="grid.navigateLeft(field.id, 'type')"
      @navigate-right="grid.navigateRight(field.id, 'type')"
      @navigate-up="grid.navigateUp(field.id, 'type')"
      @navigate-down="grid.navigateDown(field.id, 'type')"
      @delete-self="deleteField(field.id)"
      @duplicate-self="duplicateFieldAndFocus(field.id)"
      @keydown.delete.exact="isEditing || deleteField(field.id)"
      @drop="(p, v) => dropField(v.id, p, field.id)"
      @enter="grid.navigateDown(field.id, 'type')"
      class="-mx-1 self-start px-1 py-1 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
    />
    <div class="mb-1">
      <!-- Add a field -->
      <button
        v-show="!readonly"
        tabindex="-1"
        ref="addFieldRef"
        class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="isEnum ? createOption() : createFieldRef?.show()"
        @enter="isEnum ? createOption() : createFieldRef?.show()"
        @keydown.up.exact.prevent="focusLast"
        @keydown.down.exact.prevent="$emit('navigateDown')"
      >
        <PlusIcon class="h-4 w-4" />{{ isEnum ? "Option" : "Field" }}
      </button>
      <!-- Create popup right below button -->
      <CreateFieldInterface ref="createFieldRef" :title="'New field'" @select="createNewField" />
    </div>
  </div>
</template>
