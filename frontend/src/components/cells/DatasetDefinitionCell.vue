<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import { useNavigationGrid } from "@/components/cells/grid";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { STRING_TYPE_NODE, useStatementContext } from "@/components/statement";
import { TypeTag, type TypeNodeData } from "@/gql/graphql";
import { generateKeyBetween } from "@/utils/fractional";
import { computed, ref, type Ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(description);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);

// assumes this is a struct dataset
if (context.typeNodeRoot.value?.tag != TypeTag.Struct) {
  throw new Error("unexpected type node tag: " + context.typeNodeRoot.value?.tag);
}
const fieldTypeNodes = computed(() => context.typeNodesChildren.value ?? []);
const records = computed(() =>
  context.statement.value.records.slice().sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1))
);
const columnsInOrder: Ref<string[]> = computed(() => fieldTypeNodes.value?.map((n) => n.name ?? "") ?? []);
const recordsLength = computed(() => records.value?.length ?? 0);
const typeGrid = useNavigationGrid<string, InstanceType<typeof InlineTypeCell>>(
  computed(() => ["name", "type"]),
  fieldTypeNodes
);
const recordGrid = useNavigationGrid<string, InstanceType<typeof InlineValueCell>>(columnsInOrder, records);

function focusFirstIfExists() {
  console.log("focus first");
}

function focusLastOrHeader() {
  console.log("focus last");
}

function insertField() {
  console.log("insert field");
}

function updateFieldName(fieldId: string, name: string) {
  console.log("update field name", fieldId, name);
}

function updateFieldType(field: TypeNodeData, changed: TypeNodeData) {
  console.log("update field type", field, changed);
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
  console.log("insert record at", orderKey);
}

function deleteRecord(recordId: string) {
  console.log("delete record", recordId);
}

function writeRecordField(recordId: string, column: string, value: any) {
  console.log("write record field", recordId, column, value);
}

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
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
    @navigate-down="focusFirstIfExists"
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
  <div class="grid w-fit min-w-fit grid-cols-2 gap-x-3">
    <!-- Field types -->
    <div v-for="fieldNode in fieldTypeNodes" :key="fieldNode?.id" class="inline-flex gap-1 focus-within:bg-orange-50">
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
        class="rounded-sm border border-transparent py-0.5 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50"
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
      @keydown.up.exact="focusLastOrHeader"
      @keydown.down.exact="context.navigateDown"
    >
      +record
    </button>
    <!-- Add field button -->
    <button
      tabindex="-1"
      ref="addRecordRef"
      class="w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
      @click="insertField()"
      @enter="insertField()"
      @keydown.up.exact="focusLastOrHeader"
      @keydown.down.exact="context.navigateDown"
    >
      +field
    </button>
  </div>
  <!-- TODO @Incomplete: dataset record editing -->
</template>
