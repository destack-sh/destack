<script lang="ts" setup>
import InlineActions from "@/components/basic/InlineActions.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import { useNavigationGrid } from "@/components/cells/grid";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { useMagicActions } from "@/components/file";
import {
  makeTypeNode,
  STRING_TYPE_NODE,
  useStatementContext,
  type InlineAction,
  type SimpleType,
} from "@/components/statement";
import { humanizeNumber } from "@/composables/useNow";
import { graphql } from "@/gql";
import { TypeTag } from "@/gql/graphql";
import type { StatementHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newDatasetRecordId } from "@/state/operations/statement";
import { symbolOf } from "@/state/runtime";
import { generateKeyBetween, generateNKeysBetween, INTEGER_ZERO } from "@/utils/fractional";
import { ArrowDownIcon, ArrowPathIcon, PlusIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useMouseInElement } from "@vueuse/core";
import { computed, nextTick, ref, type Ref } from "vue";

const PAGE_SIZE = 10;
const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);
const gridRef: Ref<HTMLDivElement | null> = ref(null);
const loadMoreRef: Ref<HTMLButtonElement | null> = ref(null);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);
const addFieldRef: Ref<HTMLButtonElement | null> = ref(null);
const addingDescription = ref(false);

const {
  loading,
  result: fetchedRecords,
  refetch,
  fetchMore,
} = useQuery(
  graphql(/* GraphQL */ `
    query records($statementId: GlobalID!, $after: String, $first: Int) {
      statement(id: $statementId) {
        id
        records(filters: { isVisible: true }, after: $after, first: $first) {
          totalCount
          pageInfo {
            hasNextPage
            hasPreviousPage
            startCursor
            endCursor
          }
          edges {
            cursor
            node {
              id
              revision
              createdAt
              updatedAt
              deletedAt
              orderKey
              data
            }
          }
        }
      }
    }
  `),
  {
    statementId: computed(() => context.statement.value.id),
    after: null,
    first: PAGE_SIZE + 1, // overfetch by one to get order key for next page
  }
);
const pageInfo = computed(() => fetchedRecords.value?.statement?.records.pageInfo);
const recordsInView = computed(
  () =>
    fetchedRecords.value?.statement?.records.edges
      .slice(0, pageInfo.value?.hasNextPage ? -1 : undefined)
      .map((e) => e.node)
      .filter((n) => n.deletedAt == null)
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? []
);
const lastRecordInView = computed(() => recordsInView.value?.[recordsInView.value.length - 1]);
const overfetchedRecord = computed(() =>
  pageInfo.value?.hasNextPage ? fetchedRecords.value?.statement?.records.edges.slice(-1)[0]?.node : null
);

function loadMore() {
  if (!pageInfo.value?.hasNextPage) {
    return;
  }
  fetchMore({
    variables: {
      after: fetchedRecords.value?.statement?.records.edges.slice(-1)[0]?.cursor,
      first: PAGE_SIZE, // no need to overfetch again, already have 1 extra
    },
  });
}

const fieldTypeNodes = computed(() => context.typeNodes.value?.map((n) => n as SimpleType) ?? []);
const lastField = computed(() => fieldTypeNodes.value?.[fieldTypeNodes.value?.length - 1]);
const columnsInOrder: Ref<string[]> = computed(() => fieldTypeNodes.value?.map((n) => n.name ?? "") ?? []);

const typeGrid = useNavigationGrid<"name" | "type", InstanceType<typeof InlineTypeCell>>(
  computed(() => ["name", "type"]),
  fieldTypeNodes,
  {
    gridNavigateUp: focusDescriptionFromTop,
    gridNavigateDown: focusFirstRecord,
  }
);
const recordGrid = useNavigationGrid<string, InstanceType<typeof InlineValueCell>>(columnsInOrder, recordsInView, {
  gridNavigateUp: () => {
    if (typeGrid.refs.value.length > 0) {
      typeGrid.focus(-1, "name");
    } else {
      focusDescriptionFromBottom();
    }
  },
  gridNavigateDown: () => (loadMoreRef.value ?? addRecordRef.value)?.focus(),
});
const isEditing = computed(
  () => typeGrid.refs.value.find((n) => n.editing) || recordGrid.refs.value.find((n) => n.editing)
);

function focusDescriptionFromTop() {
  if (description.value?.length > 0 || addingDescription.value) {
    descriptionRef.value?.focus();
  } else {
    focusFirst();
  }
}

function focusDescriptionFromBottom() {
  if (description.value?.length > 0 || addingDescription.value) {
    descriptionRef.value?.focus();
  } else {
    declarationRef.value?.focus();
  }
}

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

const ops = useOperations();

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
    if (overfetchedRecord.value == null) {
      // end of dataset
      console.debug("insert record at end of dataset", lastRecordInView.value);
      orderKey = generateKeyBetween(lastRecordInView.value?.orderKey ?? null, null);
    } else {
      // end of page but not end of dataset
      console.debug("insert record at end of page", lastRecordInView.value, overfetchedRecord.value);
      orderKey = generateKeyBetween(
        lastRecordInView.value?.orderKey ?? null,
        overfetchedRecord.value?.orderKey ?? null
      );
    }
  } else {
    const record = recordsInView.value?.find((r) => r.id === belowRecordId);
    if (record == null) {
      throw new Error("record not found: " + belowRecordId);
    }
    orderKey = generateKeyBetween(record?.orderKey ?? null, null);
  }
  ops.symbol.createRecord(null, newDatasetRecordId(), context.statement.value.id, orderKey, {} as any);
  nextTick(() => recordGrid.focus(-1, columnsInOrder.value[0]));
}

function writeRecordField(recordId: string, key: string, value: any) {
  const record = recordsInView.value.find((r) => r.id === recordId);
  if (record == null) throw new Error("record not found: " + recordId);
  const oldData = record?.data;
  const newData = { ...oldData, [key]: value };
  ops.symbol.updateRecord(null, recordId, oldData, newData);
}

function deleteRecordField(recordId: string, key: string) {
  const record = recordsInView.value.find((r) => r.id === recordId);
  if (record == null) throw new Error("record not found: " + recordId);
  const oldData = record?.data;
  const newData = { ...oldData };
  delete newData[key];
  ops.symbol.updateRecord(null, recordId, oldData, newData);
}

function deleteRecord(recordId: string) {
  const recordIdx = recordsInView.value.findIndex((r) => r.id === recordId);
  if (recordIdx < 0) throw new Error("record not found: " + recordId);
  ops.symbol.softDeleteRecord(null, recordId);
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
const magic = useMagicActions(context.statement as Ref<StatementHeader>);
async function onDropFiles(recordId: string, column: string, position: "above" | "below", files: File[]) {
  const key = context.typeNodesByName.value?.[column]?.key;
  console.log("drop insert files into dataset", recordId, column, key, position, files);
  const recordIdx = recordsInView.value.findIndex((r) => r.id === recordId);
  const record = recordsInView.value[recordIdx];
  const above = recordsInView.value[recordIdx - 1];
  const below = recordsInView.value[recordIdx + 1];
  let orderKeys;
  if (position == "above") {
    orderKeys = generateNKeysBetween(above?.orderKey ?? null, record.orderKey, files.length);
  } else {
    orderKeys = generateNKeysBetween(record.orderKey, below?.orderKey ?? null, files.length);
  }
  await magic.insertFilesAsRecords(key, orderKeys, files);
}
const position = useMouseInElement(gridRef);

const extraActions = computed(() => {
  const inlineActions: InlineAction[] = [
    {
      label: "Reload view",
      icon: ArrowPathIcon,
      active: loading.value,
      action: () => refetch(),
    },
    {
      label: "Add record",
      icon: PlusIcon,
      action: () => insertRecord(),
    },
  ];
  return inlineActions;
});

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
  <div class="flex flex-row justify-between">
    <div class="flex flex-row items-baseline">
      <DeclarationCell ref="declarationRef" @navigate-down="focusDescriptionFromTop" />
      <!-- Table vs value selector -->
      <span class="ml-1 inline-flex" :class="context.focused.value ? 'text-gray-400' : 'text-gray-300'">table</span>
      <button
        tabindex="-1"
        v-if="description.length == 0 && !context.readonly.value && !addingDescription"
        @click="
          addingDescription = true;
          descriptionRef?.focus();
        "
        class="ml-2 w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
      >
        +description
      </button>
    </div>
    <div
      class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
    >
      <span v-if="(fetchedRecords?.statement?.records.totalCount ?? -1) > 0" class="text-gray-400">
        {{ humanizeNumber(fetchedRecords?.statement?.records.totalCount ?? 0) }}
      </span>
      <InlineActions :extraActions="extraActions" />
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
    @navigate-down="focusFirst"
    @enter="context.insertBelow"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0 && !context.readonly.value && addingDescription"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
  >
    +description
  </button>
  <!-- Dataset type and records -->
  <table ref="gridRef" class="-mx-1 w-full table-fixed">
    <!-- Field types -->
    <tr class="border-b border-orange-900 border-opacity-[12%]">
      <td v-for="field in fieldTypeNodes" :key="field?.id" class="">
        <div class="flex flex-row gap-0.5 whitespace-nowrap p-1 focus-within:bg-orange-100">
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
      </td>
    </tr>
    <!-- Records -->
    <tr
      v-for="record in recordsInView"
      :key="record.id"
      class="border-collapse border-b border-orange-900 border-opacity-[12%] align-top"
    >
      <td v-for="field in fieldTypeNodes" :key="record.id + '.' + field?.id" class="h-full">
        <InlineValueCell
          :ref="(el: any) => recordGrid.registerColumnRef(record.id, field.name as string, el)"
          :model-value="record.data?.[field.key as string]"
          @update:model-value="(val) => writeRecordField(record.id, field.key as string, val)"
          :type="runtimeTypeOf(field)"
          :readonly="context.readonly.value"
          :active="context.editing.value || context.focused.value"
          :placeholder-value="context.editing.value ? field.name : undefined"
          immediate
          debounced
          :supports-drop="!context.readonly.value"
          @drop-files="(p, v) => onDropFiles(record.id, field.name as string, p, v)"
          @navigate-left="recordGrid.navigateLeft(record.id, field.name as string)"
          @navigate-right="recordGrid.navigateRight(record.id, field.name as string)"
          @navigate-up="recordGrid.navigateUp(record.id, field.name as string)"
          @navigate-down="recordGrid.navigateDown(record.id, field.name as string)"
          @delete-left="deleteRecord(record.id)"
          class="h-full w-full self-start border border-transparent border-opacity-[12%] px-1 py-0.5 focus-within:border-solid focus-within:border-gray-700 focus-within:bg-orange-100 hover:bg-orange-100"
        />
        <!-- :EditableCellStyle -->
      </td>
    </tr>
  </table>
  <!-- Load more -->
  <div class="my-1 flex flex-row gap-2">
    <button
      v-if="pageInfo?.hasNextPage"
      @click="loadMore()"
      @keydown.up.exact="focusLastRecord"
      @keydown.right.exact="addRecordRef?.focus"
      @keydown.down.exact="context.navigateDown"
      :disabled="loading"
      ref="loadMoreRef"
      class="flex w-fit select-none flex-row items-center rounded-sm px-0.5 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
    >
      <template v-if="loading">
        <ArrowPathIcon class="mr-0.5 h-3 w-3" :class="loading ? 'animate-spin' : ''" />
        loading
      </template>
      <template v-else>
        <ArrowDownIcon class="h-3 w-3" />
        load {{ PAGE_SIZE }} more
      </template>
    </button>
    <!-- Insert button -->
    <button
      v-if="!context.readonly.value"
      tabindex="-1"
      ref="addRecordRef"
      class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="insertRecord()"
      @enter="insertRecord()"
      @keydown.up.exact="focusLastRecord"
      @keydown.right.exact="addFieldRef?.focus"
      @keydown.left.exact="loadMoreRef?.focus"
      @keydown.down.exact="context.navigateDown"
    >
      +record
    </button>
    <!-- Add field button -->
    <button
      v-if="!context.readonly.value"
      tabindex="-1"
      ref="addFieldRef"
      class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="insertField()"
      @enter="insertField()"
      @keydown.up.exact="focusLastRecord"
      @keydown.left.exact="addRecordRef?.focus"
      @keydown.down.exact="context.navigateDown"
    >
      +field
    </button>
  </div>
</template>
