<script lang="ts" setup>
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { StatementType, TypeTag } from "@/gql/graphql";
import { useFields } from "@/state/statement";
import { Bars3Icon, PlusIcon, SquaresPlusIcon, TagIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, toRef, type Ref } from "vue";
import type { Field } from "@/state/module";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useOperations } from "@/state/operations";
import type { StatementAction } from "@/state/bench";

const props = defineProps<Pick<StatementProps, "statement" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();

const isEnum = computed(() => props.statement.type == StatementType.Choice);
const fieldsX = useFields(toRef(props, "statement"));
const { moveFieldTo, selfFields, duplicateField } = fieldsX;
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

function createOption() {
  const name = "Option " + (fieldsLength.value + 1);
  const newField = fieldsX.createNewField({ name, tag: TypeTag.Literal, flags: 0 });
  nextTick(() => {
    grid.getRef(newField.id, "type").open("all");
  });
}

function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "value"> & Partial<Field>) {
  const field = fieldsX.createNewField(template);
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
  ops.symbol.softDeleteField(null, props.statement.id, field); // must exist
  grid.focus(fieldIdx - 1, "type"); // move focus above
}

function dropField(droppedId: string, position: "above" | "below" | "left" | "right", fieldId: string) {
  const dropped = selfFields.value.find((n) => n.id == droppedId);
  const field = selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  moveFieldTo(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
  nextTick(() => grid.focus(droppedId, "type"));
}

function writeType(fieldId: string, newType: Field) {
  const oldType = selfFields.value?.find((m) => m.id === fieldId);
  if (!oldType) return;
  fieldsX.updateField(oldType, { ...oldType, ...newType, id: fieldId });
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

const actions = computed(() => {
  const actions: StatementAction[] = [];
  actions.push({
    label: "Add " + (isEnum.value ? "option" : "field"),
    groupId: "edit",
    icon: SquaresPlusIcon,
    disabled: props.readonly,
    action: () => {
      if (isEnum.value) {
        createOption();
      } else {
        createFieldRef.value?.show();
        nextTick(() => createFieldRef.value?.focus());
      }
    },
  });
  return actions;
});

defineExpose({
  focus: (position: "first" | "last" = "first") =>
    position == "first" ? focusFirstIfExists() : addFieldRef.value?.focus(),
  blur: () => {
    addFieldRef.value?.blur();
    grid.blur();
  },
  actions,
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
      :ref-types="[TypeTag.Enum, TypeTag.Struct]"
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
      class="self-start py-0.5 pr-1 text-gray-700 focus-within:bg-amber-100 hover:bg-amber-100"
    />
    <div class="mb-1">
      <!-- Add a field -->
      <button
        v-show="!readonly"
        tabindex="-1"
        ref="addFieldRef"
        class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-amber-100 hover:text-gray-700 focus:bg-amber-100 group-focus-within/statement:text-gray-400"
        @click="isEnum ? createOption() : createFieldRef?.show()"
        @enter="isEnum ? createOption() : createFieldRef?.show()"
        @keydown.up.exact.prevent="selfFields?.length > 0 ? focusLast() : emit('navigateUp')"
        @keydown.down.exact.prevent="$emit('navigateDown')"
      >
        <PlusIcon class="h-4 w-4" />{{ isEnum ? "Option" : "Field" }}
      </button>
      <!-- Create popup right below button -->
      <CreateFieldInterface
        ref="createFieldRef"
        :title="'New field'"
        :ref-types="[TypeTag.Enum, TypeTag.Struct]"
        @select="createNewField"
      />
    </div>
  </div>
</template>
