<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import { makeField, useFields } from "@/state/statement";
import { TypeTag, type Field } from "@/gql/graphql";
import { TypeFlag, useCurrentModule } from "@/state/module";
import { generateKeyBetween } from "@/utils/fractional";
import {
  ArrowDownRightIcon,
  ArrowLongDownIcon,
  ArrowLongRightIcon,
  ArrowUpRightIcon,
  PlusIcon,
} from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, toRef, type Ref } from "vue";
import { usePanelContext } from "@/state/bench";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { useOperations } from "@/state/operations";

const props = defineProps<Pick<StatementProps, "statement" | "readonly">>();
const emit = defineEmits<StatementEmit>();

const ops = useOperations();

const fieldsX = useFields(toRef(props, "statement"));
const { selfInputs: inputs, selfOutputs: outputs, fields, selfFields, inheritedFields } = fieldsX;

type ColumnType = "type";
const columnsInOrder: Ref<ColumnType[]> = ref(["type"] as ColumnType[]);
const inputGrid = useNavigationGrid<string, InstanceType<typeof FieldInterface>>(columnsInOrder, inputs, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => addInputRef.value?.focus(),
  gridNavigateRight: (rowIdx) => focusColumn("output", rowIdx, 0),
  nowrapLeft: true,
  nowrapRight: true,
});
const outputGrid = useNavigationGrid<string, InstanceType<typeof FieldInterface>>(columnsInOrder, outputs, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => addOutputRef.value?.focus(),
  gridNavigateLeft: (rowIdx) => focusColumn("input", rowIdx, -1),
  nowrapLeft: true,
  nowrapRight: true,
});

const addInputRef: Ref<HTMLButtonElement | null> = ref(null);
const createInputRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const addOutputRef: Ref<HTMLButtonElement | null> = ref(null);
const createOutputRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const panel = usePanelContext();
const isHorizontal = computed(() => panel.size.value.width > 700);

function readColumn(field: Field, column: ColumnType) {
  if (column == "type") {
    return field;
  } else {
    return field[column];
  }
}
function writeColumn(kind: "input" | "output", fieldId: string, column: ColumnType, value: any) {
  const field = fields.value?.find((m) => m.id === fieldId);
  if (!field) {
    return;
  }
  const flags = value.flags | (kind == "output" ? TypeFlag.IS_OUTPUT : 0);
  if (column == "type") {
    fieldsX.updateField(field as Field, { ...value, flags } as Field);
  } else {
    fieldsX.updateField(field as Field, { ...field, [column]: value, flags } as Field);
  }
}

function insertBelow(
  kind: "input" | "output",
  template: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "value">
) {
  // function fields are required by default
  const flags = (kind == "output" ? TypeFlag.IS_OUTPUT : 0) | ((template.flags ?? 0) & ~TypeFlag.IS_OPTIONAL);
  const newField = fieldsX.createNewField({ ...template, flags });
  nextTick(() => {
    const grid = kind == "input" ? inputGrid : outputGrid;
    grid.getRef(newField.id, "type").open("all");
  });
}

function moveField(field: Field, position: "before" | "after", other: Field) {
  const otherIndex = selfFields.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(selfFields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    fieldsX.moveField(field, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, selfFields.value[otherIndex + 1]?.orderKey ?? null);
    fieldsX.moveField(field, orderKey);
  }
}

function dropField(droppedId: string, position: "left" | "right" | "above" | "below", fieldId: string) {
  const dropped = selfFields.value.find((n) => n.id == droppedId);
  const field = selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  if ((dropped.flags & TypeFlag.IS_OUTPUT) != (field.flags & TypeFlag.IS_OUTPUT)) return; // ignore drops between input/output (requires transaction)
  moveField(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
}

function deleteField(kind: "input" | "output", fieldId: string) {
  const fields = kind == "input" ? inputs.value : outputs.value;
  const fieldIdx = fields.findIndex((m) => m.id === fieldId);
  if (fieldIdx == null || fieldIdx < 0) {
    return;
  }
  const field = fields[fieldIdx];
  ops.symbol.softDeleteField(null, props.statement.id, field); // must exist
  (kind == "input" ? inputGrid : outputGrid).focus(fieldIdx - 1, "type"); // move focus above
}

function focus(what: "first" | "last", kind: "input" | "output") {
  const fields = kind == "input" ? inputs.value : outputs.value;
  if (fields.length == 0) {
    // focus add button
    (kind == "input" ? addInputRef : addOutputRef).value?.focus();
  } else {
    // focus first/last
    (kind == "input" ? inputGrid : outputGrid).focus(what == "first" ? 0 : -1, "type");
  }
}

function focusColumn(kind: "input" | "output", rowIdx: number, columnIdx: number) {
  if (columnIdx < 0) {
    // wrap
    columnIdx = columnIdx + columnsInOrder.value.length;
  }
  const fields = kind == "input" ? inputs.value : outputs.value;
  if (rowIdx < fields.length) {
    (kind == "input" ? inputGrid : outputGrid).focus(rowIdx, columnsInOrder.value[columnIdx]);
  } else if (rowIdx == fields.length) {
    // focus add button
    (kind == "input" ? addInputRef : addOutputRef).value?.focus();
  } else {
    // ignore?
  }
}

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    if (position == "first") {
      focus("first", "input");
    } else if (addInputRef.value != null) {
      addInputRef.value.focus();
    } else {
      focus("last", "input");
    }
  },
  blur: () => {
    inputGrid.blur();
    outputGrid.blur();
    addInputRef.value?.blur();
    addOutputRef.value?.blur();
  },
  createInput: () => {
    createInputRef.value?.show();
  },
  createOutput: () => {
    createOutputRef.value?.show();
  },
  actions: [
    {
      label: "Add input",
      groupId: "edit",
      icon: ArrowDownRightIcon,
      hideInline: true,
      disabled: props.readonly,
      action: () => {
        createInputRef.value?.show();
        nextTick(() => createInputRef.value?.focus());
      },
    },
    {
      label: "Add output",
      groupId: "edit",
      icon: ArrowUpRightIcon,
      disabled: props.readonly,
      hideInline: true,
      action: () => {
        createOutputRef.value?.show();
        nextTick(() => createOutputRef.value?.focus());
      },
    },
  ],
});
</script>
<template>
  <div class="flex w-full" :class="isHorizontal ? 'flex-row items-start gap-4' : 'flex-col items-start gap-2'">
    <!-- Inputs -->
    <div class="-mx-1 flex h-fit w-fit flex-1 flex-shrink-0 flex-col gap-0.5">
      <template v-for="field of inputs" :key="field.id">
        <FieldInterface
          :ref="(el: any) => inputGrid.registerColumnRef(field.id, 'type', el)"
          :model-value="readColumn(field as Field, 'type')"
          @update:model-value="(val: any) => writeColumn('input', field.id, 'type', val)"
          :readonly="readonly"
          :ref-types="[TypeTag.Struct, TypeTag.Enum]"
          :inlined="inheritedFields.find((n) => n.key == field.key) != null"
          tuple-name="input"
          orientation="vertical"
          @navigate-left="inputGrid.navigateLeft(field.id, 'type')"
          @navigate-right="inputGrid.navigateRight(field.id, 'type')"
          @navigate-up="inputGrid.navigateUp(field.id, 'type')"
          @navigate-down="inputGrid.navigateDown(field.id, 'type')"
          @enter="inputGrid.navigateDown(field.id, 'type')"
          @delete-left="deleteField('input', field.id)"
          @delete-self="deleteField('input', field.id)"
          @drop="(p, v) => dropField(v.id, p, field.id)"
          class="w-full self-start px-1 py-0.5 text-gray-700 focus-within:bg-amber-100 hover:bg-amber-100"
        />
      </template>
      <!-- Add a field -->
      <button
        v-show="!readonly"
        tabindex="-1"
        ref="addInputRef"
        class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-amber-100 hover:text-gray-700 focus:bg-amber-100 group-focus-within/statement:text-gray-400"
        @click="createInputRef?.show()"
        @enter="createInputRef?.show()"
        @keydown.up.exact.prevent="inputs.length > 0 ? focus('last', 'input') : $emit('navigateUp')"
        @keydown.down.exact.prevent="emit('navigateDown')"
        @keydown.right.exact.prevent="addOutputRef?.focus"
      >
        <PlusIcon class="h-4 w-4" /> Input
      </button>
      <CreateFieldInterface ref="createInputRef" title="Add input to" @select="insertBelow('input', $event)" />
    </div>
    <!-- Lil' arrow -->
    <component
      :is="isHorizontal ? ArrowLongRightIcon : ArrowLongDownIcon"
      class="mt-2 h-5 w-5 self-start text-gray-700"
    />
    <!-- Outputs -->
    <!-- TODO @Cleanup: outputs are almost exactly like inputs, much duplication (but the UI is not great anyway) -->
    <div class="-mx-1 flex h-fit w-fit flex-1 flex-shrink-0 flex-col gap-0.5">
      <template v-for="field of outputs" :key="field.id">
        <FieldInterface
          :ref="(el: any) => outputGrid.registerColumnRef(field.id, 'type', el)"
          :model-value="readColumn(field as Field, 'type')"
          @update:model-value="(val: any) => writeColumn('output', field.id, 'type', val)"
          :readonly="readonly"
          :ref-types="[TypeTag.Struct, TypeTag.Enum]"
          :inlined="inheritedFields.find((n) => n.key == field.key) != null"
          tuple-name="output"
          orientation="vertical"
          @navigate-left="outputGrid.navigateLeft(field.id, 'type')"
          @navigate-right="outputGrid.navigateRight(field.id, 'type')"
          @navigate-up="outputGrid.navigateUp(field.id, 'type')"
          @navigate-down="outputGrid.navigateDown(field.id, 'type')"
          @enter="outputGrid.navigateDown(field.id, 'type')"
          @delete-left="deleteField('output', field.id)"
          @delete-self="deleteField('output', field.id)"
          @drop="(p, v) => dropField(v.id, p, field.id)"
          class="w-full self-start px-1 py-0.5 text-gray-700 focus-within:bg-amber-100 hover:bg-amber-100"
        />
      </template>
      <!-- Add a field -->
      <button
        v-if="!readonly"
        tabindex="-1"
        ref="addOutputRef"
        class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-amber-100 hover:text-gray-700 focus:bg-amber-100 group-focus-within/statement:text-gray-400"
        @click="createOutputRef?.show()"
        @enter="createOutputRef?.show()"
        @keydown.up.exact.prevent="outputs.length > 0 ? focus('last', 'output') : $emit('navigateUp')"
        @keydown.down.exact.prevent="emit('navigateDown')"
        @keydown.left.exact.prevent="addInputRef?.focus"
      >
        <PlusIcon class="h-4 w-4" /> Output
      </button>
      <CreateFieldInterface
        ref="createOutputRef"
        title="Add output"
        :ref-types="[TypeTag.Enum, TypeTag.Struct]"
        @select="insertBelow('output', $event)"
      />
    </div>
  </div>
</template>
