<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import { useNavigationGrid } from "@/components/cells/grid";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { makeTypeNode, STRING_TYPE_NODE, useStatementContext, type SimpleType } from "@/components/statement";
import { TypeTag } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { newDatasetRecordId } from "@/state/operations/statement";
import { symbolOf } from "@/state/runtime";
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";
import { useMouseInElement } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);
const gridRef: Ref<HTMLDivElement | null> = ref(null);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);

const fieldTypeNodes = computed(() => context.typeNodes.value?.map((n) => n as SimpleType) ?? []);
const lastField = computed(() => fieldTypeNodes.value?.[fieldTypeNodes.value?.length - 1]);
const columnsInOrder: Ref<string[]> = computed(() => fieldTypeNodes.value?.map((n) => n.name ?? "") ?? []);
const recordsLength = computed(() => context.records.value?.length ?? 0);
const typeGrid = useNavigationGrid<"name" | "type", InstanceType<typeof InlineTypeCell>>(
  computed(() => ["name", "type"]),
  fieldTypeNodes,
  {
    gridNavigateUp: () => descriptionRef.value?.focus(),
    gridNavigateDown: focusFirstRecord,
  }
);
const recordGrid = useNavigationGrid<string, InstanceType<typeof InlineValueCell>>(columnsInOrder, context.records, {
  gridNavigateUp: () => {
    if (typeGrid.refs.value.length > 0) {
      typeGrid.focus(-1, "name");
    } else {
      descriptionRef.value?.focus();
    }
  },
  gridNavigateDown: () => addRecordRef.value?.focus(),
});
const isEditing = computed(
  () => typeGrid.refs.value.find((n) => n.editing) || recordGrid.refs.value.find((n) => n.editing)
);

function focusFirst() {
  if (typeGrid.refs.value.length > 0) {
    typeGrid.focus(0, "name");
  } else {
    focusFirstRecord();
  }
}

function focusFirstRecord() {
  if (recordGrid.refs.value.length > 0) {
    recordGrid.focus(0, columnsInOrder.value[0]);
  } else {
    addRecordRef.value?.focus();
  }
}

function focusLastRecord() {
  if (recordGrid.refs.value.length > 0) {
    recordGrid.focus(-1, columnsInOrder.value[0]);
  } else if (typeGrid.refs.value.length > 0) {
    typeGrid.focus(0, "name");
  } else {
    descriptionRef.value?.focus();
  }
}

const operations = useOperations();

function insertField() {
  const nextOrderKey = generateKeyBetween(lastField.value?.orderKey ?? INTEGER_ZERO, null);
  context.createTypeNode(
    makeTypeNode({
      name: "field " + fieldTypeNodes.value?.length,
      tag: TypeTag.String,
      orderKey: nextOrderKey,
    })
  );
  nextTick(() => typeGrid.focus(-1, "name"));
}

function updateFieldName(node: SimpleType, name: string) {
  context.updateTypeNode(node, { ...node, name });
}

function updateFieldType(node: SimpleType, changed: SimpleType) {
  context.updateTypeNode(node, changed);
}

function deleteField(node: SimpleType) {
  const fieldIdx = fieldTypeNodes.value?.findIndex((n) => n.id === node.id);
  context.deleteTypeNode(node);
  typeGrid.focus(fieldIdx - 1, "name");
}

function insertRecord(belowRecordId?: string) {
  let orderKey;
  if (belowRecordId == null) {
    const lastRecord = context.records.value?.[recordsLength.value - 1];
    orderKey = generateKeyBetween(lastRecord?.orderKey ?? null, null);
  } else {
    const record = context.records.value?.find((r) => r.id === belowRecordId);
    orderKey = generateKeyBetween(record?.orderKey ?? null, null);
  }
  operations.symbol.createRecord(newDatasetRecordId(), context.statement.value.id, orderKey, {} as any);
  nextTick(() => recordGrid.focus(-1, columnsInOrder.value[0]));
}

function writeRecordField(recordId: string, column: string, value: any) {
  const recordIdx = context.records.value.findIndex((r) => r.id === recordId);
  if (recordIdx < 0) throw new Error("record not found: " + recordId);
  const record = context.records.value[recordIdx];
  const oldData = record?.data;
  const newData = { ...oldData, [column]: value };
  operations.symbol.updateRecord(recordId, oldData, newData);
}

function deleteRecord(recordId: string) {
  const recordIdx = context.records.value.findIndex((r) => r.id === recordId);
  if (recordIdx < 0) throw new Error("record not found: " + recordId);
  operations.symbol.softDeleteRecord(recordId);
  // move focus up
  recordGrid.focus(recordIdx - 1, columnsInOrder.value[0]);
}

// map the field type to its actual runtime type
// children cannot be imputed into SimpleTypeNode but we still want to know the actual type
const datasetSymbol = computed(() => symbolOf(context.statement.value.id));
function runtimeTypeOf(field: SimpleType) {
  return datasetSymbol.value?.typeNodes?.find((n) => n.name == field.name) ?? field;
}

// drag & drop
function onDropFiles(files: File[]) {
  console.log("drop it! data", files);
}
const position = useMouseInElement(gridRef);

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
    addRecordRef.value?.blur();
    addFieldRef.value?.blur();
    typeGrid.blur();
    recordGrid.blur();
  },
  // prevent outer drag and drop while inside grid
  innerDrag: computed(() => !position.isOutside.value),
});
</script>
<template>
  <!-- Declaration -->
  <DeclarationCell
    ref="declarationRef"
    @navigate-down="descriptionRef?.focus"
    @navigate-right="descriptionRef?.focus"
  />
  <!-- Reference type -->
  <!-- TODO @Incomplete: set dataset type to type reference -->
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-left="declarationRef?.focus()"
    @navigate-up="declarationRef?.focus()"
    @navigate-down="focusFirst"
    @enter="context.insertBelow"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0 && !context.readonly.value"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
  >
    +description
  </button>
  <!-- Dataset type and records -->
  <div
    ref="gridRef"
    class="grid min-w-fit"
    :style="{
      'grid-template-columns': `repeat(${columnsInOrder.length}, minmax(40px, 1fr))`,
    }"
  >
    <!-- Field types -->
    <div
      v-for="field in fieldTypeNodes"
      :key="field?.id"
      class="flex flex-row gap-1 border-b border-orange-900 border-opacity-[12%] focus-within:bg-orange-100"
    >
      <InlineValueCell
        :ref="(el: any) => typeGrid.registerColumnRef(field?.id, 'name', el)"
        immediate
        debounced
        :type="STRING_TYPE_NODE"
        slim
        :model-value="field.name"
        :readonly="context.readonly.value"
        :active="context.editing.value || context.focused.value"
        @update:model-value="(val: any) => updateFieldName(field, val)"
        class="border border-transparent py-0.5 focus-within:border-solid focus-within:border-gray-700 focus-within:bg-orange-100 hover:bg-orange-100"
        @navigate-left="typeGrid.navigateLeft(field?.id, 'name')"
        @navigate-right="typeGrid.navigateRight(field?.id, 'name')"
        @navigate-up="typeGrid.navigateUp(field?.id, 'name')"
        @navigate-down="typeGrid.navigateDown(field?.id, 'name')"
        @keydown.delete.exact="isEditing || deleteField(field)"
      />
      <InlineTypeCell
        :ref="(el: any) => typeGrid.registerColumnRef(field?.id, 'type', el)"
        :type="field"
        :readonly="context.readonly.value"
        class="border border-transparent py-0.5 text-gray-400 focus-within:border-solid focus-within:border-gray-700 focus-within:bg-orange-100 hover:bg-orange-100"
        :model-value="field"
        @update:model-value="(node: any) => updateFieldType(field, node)"
        @navigate-left="typeGrid.navigateLeft(field?.id, 'type')"
        @navigate-right="typeGrid.navigateRight(field?.id, 'type')"
        @navigate-up="typeGrid.navigateUp(field?.id, 'type')"
        @navigate-down="typeGrid.navigateDown(field?.id, 'type')"
        @keydown.delete.exact="isEditing || deleteField(field)"
      />
    </div>
    <!-- Records -->
    <template v-for="record in context.records.value" :key="record.id">
      <template v-for="field in fieldTypeNodes" :key="record.id + '.' + field?.id">
        <InlineValueCell
          :ref="(el: any) => recordGrid.registerColumnRef(record.id, field.name as string, el)"
          :model-value="record.data?.[field.name as string]"
          @update:model-value="(val) => writeRecordField(record.id, field.name as string, val)"
          :type="runtimeTypeOf(field)"
          :readonly="context.readonly.value"
          :active="context.editing.value || context.focused.value"
          :placeholder-value="context.editing.value ? field.name : undefined"
          immediate
          debounced
          :supports-drop="!context.readonly.value"
          @drop-files="onDropFiles"
          @navigate-left="recordGrid.navigateLeft(record.id, field.name as string)"
          @navigate-right="recordGrid.navigateRight(record.id, field.name as string)"
          @navigate-up="recordGrid.navigateUp(record.id, field.name as string)"
          @navigate-down="recordGrid.navigateDown(record.id, field.name as string)"
          @delete-left="deleteRecord(record.id)"
          class="w-full self-start rounded-sm border border-transparent py-0.5 focus-within:border-solid focus-within:border-gray-700 focus-within:bg-orange-100 hover:bg-orange-100"
        />
        <!-- :EditableCellStyle -->
      </template>
    </template>
    <!-- Insert button -->
  </div>
  <button
    v-if="!context.readonly.value"
    tabindex="-1"
    ref="addRecordRef"
    class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
    @click="insertRecord()"
    @enter="insertRecord()"
    @keydown.up.exact="focusLastRecord"
    @keydown.right.exact="addFieldRef?.focus"
    @keydown.down.exact="context.navigateDown"
  >
    +record
  </button>
  <!-- Add field button -->
  <button
    v-if="!context.readonly.value"
    tabindex="-1"
    ref="addFieldRef"
    class="ml-1 w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
    @click="insertField()"
    @enter="insertField()"
    @keydown.up.exact="focusLastRecord"
    @keydown.left.exact="addRecordRef?.focus"
    @keydown.down.exact="context.navigateDown"
  >
    +field
  </button>
  <!-- TODO @Incomplete: dataset record editing -->
</template>
