<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import { makeField, useStatementContext } from "@/state/statement";
import { TypeTag, type Field } from "@/gql/graphql";
import { TypeFlag } from "@/state/module";
import { generateKeyBetween } from "@/utils/fractional";
import { ArrowLongDownIcon, ArrowLongRightIcon, PlusIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";
import { useEditorContext } from "@/state/bench";

const context = useStatementContext();

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateRight"): void;
  (e: "navigateLeft"): void;
}>();

const nodes = computed(() => context.fields.value ?? []);
const inputs = computed(
  () => context.allFields.value?.filter((n) => !(n.flags & TypeFlag.IsOutput)).map((n) => n as Field) ?? []
);
const outputs = computed(
  () => context.allFields.value?.filter((n) => n.flags & TypeFlag.IsOutput).map((n) => n as Field) ?? []
);

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
const editor = useEditorContext();
const isHorizontal = computed(() => editor.size.value.width > 700);

function readColumn(field: Field, column: ColumnType) {
  if (column == "type") {
    return field;
  } else {
    return field[column];
  }
}
function writeColumn(kind: "input" | "output", fieldId: string, column: ColumnType, value: any) {
  const field = nodes.value?.find((m) => m.id === fieldId);
  if (!field) {
    return;
  }
  const flags = value.flags | (kind == "output" ? TypeFlag.IsOutput : 0);
  if (column == "type") {
    context.updateField(field as Field, { ...value, flags } as Field);
  } else {
    context.updateField(field as Field, { ...field, [column]: value, flags } as Field);
  }
}

function insertBelow(
  kind: "input" | "output",
  template: Pick<Field, "tag" | "hint" | "flags" | "reference" | "metadata">
) {
  const lastField = nodes.value[nodes.value.length - 1];
  const orderKey = generateKeyBetween(lastField?.orderKey ?? null, null);
  const newFieldNode = makeField({
    ...template,
    reference: template.reference as any,
    orderKey,
    flags: (kind == "output" ? TypeFlag.IsOutput : 0) | (template.flags ?? 0),
  });
  context.createNewField(newFieldNode);
  nextTick(() => (kind == "input" ? inputGrid : outputGrid).focus(-1, "type"));
}

function moveField(node: Field, position: "before" | "after", other: Field) {
  const otherIndex = context.selfFields.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(context.selfFields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    context.moveField(node, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, context.selfFields.value[otherIndex + 1]?.orderKey ?? null);
    context.moveField(node, orderKey);
  }
}

function dropField(droppedId: string, position: "above" | "below", fieldId: string) {
  const dropped = context.selfFields.value.find((n) => n.id == droppedId);
  const field = context.selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  if ((dropped.flags & TypeFlag.IsOutput) != (field.flags & TypeFlag.IsOutput)) return; // ignore drops between input/output (requires transaction)
  moveField(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
}

function deleteField(kind: "input" | "output", fieldId: string) {
  const fields = kind == "input" ? inputs.value : outputs.value;
  const fieldIdx = fields.findIndex((m) => m.id === fieldId);
  if (fieldIdx == null || fieldIdx < 0) {
    return;
  }
  const field = fields[fieldIdx];
  context.deleteField(field as any); // must exist
  (kind == "input" ? inputGrid : outputGrid).focus(fieldIdx - 1, "type"); // move focus above
}

function focus(what: "first" | "last", kind: "input" | "output") {
  const nodes = kind == "input" ? inputs.value : outputs.value;
  if (nodes.length == 0) {
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
  const nodes = kind == "input" ? inputs.value : outputs.value;
  if (rowIdx < nodes.length) {
    (kind == "input" ? inputGrid : outputGrid).focus(rowIdx, columnsInOrder.value[columnIdx]);
  } else if (rowIdx == nodes.length) {
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
          :readonly="context.readonly.value"
          :ref-types="[TypeTag.Struct, TypeTag.Enum]"
          :inlined="context.inheritedFields.value.find((n) => n.key == field.key) != null"
          tuple-name="input"
          orientation="vertical"
          @navigate-left="inputGrid.navigateLeft(field.id, 'type')"
          @navigate-right="inputGrid.navigateRight(field.id, 'type')"
          @navigate-up="inputGrid.navigateUp(field.id, 'type')"
          @navigate-down="inputGrid.navigateDown(field.id, 'type')"
          @delete-left="deleteField('input', field.id)"
          @delete-self="deleteField('input', field.id)"
          @drop="(p, v) => dropField(v.id, p, field.id)"
          class="w-full self-start px-1 py-1 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
        />
      </template>
      <!-- Add a field -->
      <button
        v-show="!context.readonly.value"
        tabindex="-1"
        ref="addInputRef"
        class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="createInputRef?.show()"
        @enter="createInputRef?.show()"
        @keydown.up.exact.prevent="inputs.length > 0 ? focus('last', 'input') : $emit('navigateUp')"
        @keydown.down.exact.prevent="context.navigateDown"
        @keydown.right.exact.prevent="addOutputRef?.focus"
      >
        <PlusIcon class="h-4 w-4" /> Input
        <CreateFieldInterface
          ref="createInputRef"
          :title="'New input to ' + context.statement.value.name"
          @select="insertBelow('input', $event)"
        />
      </button>
    </div>
    <!-- Lil' arrow -->
    <component
      :is="isHorizontal ? ArrowLongRightIcon : ArrowLongDownIcon"
      class="mt-1 h-5 w-5 self-center text-gray-700"
    />
    <!-- Outputs -->
    <!-- TODO @Cleanup: outputs are almost exactly like inputs, much duplication -->
    <div class="-mx-1 flex h-fit w-fit flex-1 flex-shrink-0 flex-col gap-0.5">
      <template v-for="field of outputs" :key="field.id">
        <FieldInterface
          :ref="(el: any) => outputGrid.registerColumnRef(field.id, 'type', el)"
          :is="'type' == 'type' ? FieldInterface : ValueInterface"
          :model-value="readColumn(field as Field, 'type')"
          @update:model-value="(val: any) => writeColumn('output', field.id, 'type', val)"
          :readonly="context.readonly.value"
          :ref-types="[TypeTag.Struct, TypeTag.Enum]"
          :inlined="context.inheritedFields.value.find((n) => n.key == field.key) != null"
          tuple-name="output"
          orientation="vertical"
          @navigate-left="outputGrid.navigateLeft(field.id, 'type')"
          @navigate-right="outputGrid.navigateRight(field.id, 'type')"
          @navigate-up="outputGrid.navigateUp(field.id, 'type')"
          @navigate-down="outputGrid.navigateDown(field.id, 'type')"
          @delete-left="deleteField('output', field.id)"
          @delete-self="deleteField('output', field.id)"
          @drop="(p, v) => dropField(v.id, p, field.id)"
          class="w-full self-start px-1 py-1 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
        />
      </template>
      <!-- Add a field -->
      <button
        v-if="!context.readonly.value"
        tabindex="-1"
        ref="addOutputRef"
        class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="createOutputRef?.show()"
        @enter="createOutputRef?.show()"
        @keydown.up.exact.prevent="outputs.length > 0 ? focus('last', 'output') : $emit('navigateUp')"
        @keydown.down.exact.prevent="context.navigateDown"
        @keydown.left.exact.prevent="addInputRef?.focus"
      >
        <PlusIcon class="h-4 w-4" /> Output
        <CreateFieldInterface
          ref="createOutputRef"
          :title="'New output of ' + context.statement.value.name"
          @select="insertBelow('output', $event)"
        />
      </button>
    </div>
  </div>
</template>
