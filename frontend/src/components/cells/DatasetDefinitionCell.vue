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
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";
import { computed, nextTick, ref, type Ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(description);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);

const fieldTypeNodes = computed(() => context.typeNodes.value ?? []);
const lastFieldTypeNode = computed(() => fieldTypeNodes.value?.[fieldTypeNodes.value?.length - 1]);
const records = computed(() =>
  context.statement.value.records.slice().sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1))
);
const columnsInOrder: Ref<string[]> = computed(() => fieldTypeNodes.value?.map((n) => n.name ?? "") ?? []);
const recordsLength = computed(() => records.value?.length ?? 0);
const typeGrid = useNavigationGrid<"name" | "type", InstanceType<typeof InlineTypeCell>>(
  computed(() => ["name", "type"]),
  fieldTypeNodes,
  {
    gridNavigateUp: focusLastRecord,
    gridNavigateDown: focusFirstRecord,
  }
);
const recordGrid = useNavigationGrid<string, InstanceType<typeof InlineValueCell>>(columnsInOrder, records, {
  gridNavigateUp: () => {
    if (typeGrid.refs.value.length > 0) {
      typeGrid.focus(-1, "name");
    } else {
      descriptionRef.value?.focus();
    }
  },
  gridNavigateDown: () => addRecordRef.value?.focus(),
});

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

async function insertField() {
  await context.createTypeNode(
    makeTypeNode({
      name: "field " + fieldTypeNodes.value?.length,
      tag: TypeTag.String,
      orderKey: generateKeyBetween(lastFieldTypeNode.value?.orderKey ?? INTEGER_ZERO, null),
    })
  );
  nextTick(() => typeGrid.focus(-1, "name"));
}

function updateFieldName(node: SimpleType, name: string) {
  const updatedNode = { ...node, name };
  context.updateTypeNode(updatedNode);
}

function updateFieldType(node: SimpleType, changed: SimpleType) {
  const updatedMember = { ...node, tag: changed.tag, reference: changed.reference };
  context.updateTypeNode(updatedMember);
}

function insertRecord(belowRecordId?: string) {
  let orderKey;
  if (belowRecordId == null) {
    const lastRecord = records.value?.[recordsLength.value - 1];
    orderKey = generateKeyBetween(lastRecord?.orderKey ?? null, null);
  } else {
    const record = records.value?.find((r) => r.id === belowRecordId);
    orderKey = generateKeyBetween(record?.orderKey ?? null, null);
  }
  operations.symbol.createRecord(newDatasetRecordId(), context.statement.value.id, orderKey, {});
}

function writeRecordField(recordId: string, column: string, value: any) {
  const record = context.statement.value.records.find((r) => r.id === recordId);
  if (!record) throw new Error("record not found: " + recordId);
  const oldData = record?.data;
  const newData = { ...oldData, [column]: value };
  operations.symbol.updateRecord(recordId, context.statement.value.id, oldData, newData);
}

function deleteRecord(recordId: string) {
  const record = context.statement.value.records.find((r) => r.id === recordId);
  if (!record) throw new Error("record not found: " + recordId);
  const oldData = record?.data;
  operations.symbol.deleteRecord(recordId, context.statement.value.id, record.orderKey, oldData);
}

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
  <!-- TODO @Incomplete: set dataset type to reference -->
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
    v-if="description.length == 0"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
  >
    +description
  </button>
  <!-- Dataset type and records -->
  <div
    class="grid w-fit min-w-fit gap-x-3"
    :style="{
      'grid-template-columns': `repeat(${columnsInOrder.length}, minmax(40px, 1fr))`,
    }"
  >
    <!-- Field types -->
    <div v-for="fieldNode in fieldTypeNodes" :key="fieldNode?.id" class="flex flex-row gap-1 focus-within:bg-orange-50">
      <InlineValueCell
        :ref="(el: any) => typeGrid.registerColumnRef(fieldNode?.id, 'name', el)"
        :immediate="false"
        :type="STRING_TYPE_NODE"
        :model-value="fieldNode.name"
        :readonly="context.readonly.value"
        @update:model-value="(val: any) => updateFieldName(fieldNode, val)"
        class="rounded-sm border border-transparent py-0.5 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50"
        @navigate-left="typeGrid.navigateLeft(fieldNode?.id, 'name')"
        @navigate-right="typeGrid.navigateRight(fieldNode?.id, 'name')"
        @navigate-up="typeGrid.navigateUp(fieldNode?.id, 'name')"
        @navigate-down="typeGrid.navigateDown(fieldNode?.id, 'name')"
      />
      <InlineTypeCell
        :ref="(el: any) => typeGrid.registerColumnRef(fieldNode?.id, 'type', el)"
        :type="fieldNode"
        :readonly="context.readonly.value"
        class="rounded-sm border border-transparent py-0.5 text-gray-400 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50"
        :model-value="fieldNode"
        @update:model-value="(node: any) => updateFieldType(fieldNode, node)"
        @navigate-left="typeGrid.navigateLeft(fieldNode?.id, 'type')"
        @navigate-right="typeGrid.navigateRight(fieldNode?.id, 'type')"
        @navigate-up="typeGrid.navigateUp(fieldNode?.id, 'type')"
        @navigate-down="typeGrid.navigateDown(fieldNode?.id, 'type')"
      />
    </div>
    <!-- Records -->
    <template v-for="record in records" :key="record.id">
      <template v-for="fieldNode in fieldTypeNodes" :key="record.id + '.' + fieldNode?.id">
        <InlineValueCell
          :ref="(el: any) => recordGrid.registerColumnRef(record.id, fieldNode.name, el)"
          :model-value="record.data?.[fieldNode.name]"
          @update:model-value="(val) => writeRecordField(record.id, fieldNode.name, val)"
          :type="fieldNode"
          :readonly="context.readonly.value"
          immediate
          @navigate-left="recordGrid.navigateLeft(record.id, fieldNode.name)"
          @navigate-right="recordGrid.navigateRight(record.id, fieldNode.name)"
          @navigate-up="recordGrid.navigateUp(record.id, fieldNode.name)"
          @navigate-down="recordGrid.navigateDown(record.id, fieldNode.name)"
          @delete-left="deleteRecord(record.id)"
          class="w-full self-start rounded-sm border border-transparent py-0.5 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50"
        />
        <!-- :EditableCellStyle -->
      </template>
    </template>
    <!-- Insert button -->
    <button
      tabindex="-1"
      ref="addRecordRef"
      class="w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
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
      tabindex="-1"
      ref="addFieldRef"
      class="w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
      @click="insertField()"
      @enter="insertField()"
      @keydown.up.exact="focusLastRecord"
      @keydown.left.exact="addRecordRef?.focus"
      @keydown.down.exact="context.navigateDown"
    >
      +field
    </button>
  </div>
  <!-- TODO @Incomplete: dataset record editing -->
</template>
