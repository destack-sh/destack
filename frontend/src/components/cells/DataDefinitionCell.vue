<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import InlineActions from "@/components/basic/InlineActions.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import { useElementRefs, useNavigationGrid } from "@/components/cells/grid";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineTypeTupleCell from "@/components/cells/InlineTypeTupleCell.vue";
import InlineValueCell2 from "@/components/cells/InlineValueCell2.vue";
import { getInterface } from "@/components/cells/interfaces";
import EditableSpan from "@/components/EditableSpan.vue";
import { useMagicActions } from "@/components/file";
import {
  makeTypeNode,
  useStatementContext,
  type RecordAction,
  type SimpleType,
  type StatementAction,
} from "@/components/statement";
import { humanizeNumber } from "@/composables/useNow";
import { graphql } from "@/gql";
import { TypeTag } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import type { StatementHeader } from "@/state/editor";
import { useOperations } from "@/state/operations";
import { newDatasetRecordId, newTypeNodeId } from "@/state/operations/statement";
import { symbolOf, TypeFlag } from "@/state/runtime";
import { generateKeyBetween, generateNKeysBetween, INTEGER_ZERO } from "@/utils/fractional";
import {
  ArrowDownIcon,
  ArrowPathIcon,
  CubeTransparentIcon,
  PlusIcon,
  Square2StackIcon,
  Squares2X2Icon,
  SquaresPlusIcon,
  TrashIcon,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useMouseInElement } from "@vueuse/core";
import { computed, nextTick, ref, watchEffect, type Ref } from "vue";

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
const isTable = computed(() => (context.statement.value.rootTypeFlags ?? 0) & TypeFlag.IsArray);
const appearance = useAppearance();

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
    first: !isTable.value ? 1 : PAGE_SIZE + 1, // overfetch by one to get order key for next page
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
const mainRecord = computed(() => recordsInView.value?.[0]);

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

const selfFields = computed(
  () => context.typeNodes.value?.filter((n) => !(n.flags & TypeFlag.IsUnionWith)).map((n) => n as SimpleType) ?? []
);
const extendedTypes = computed(
  () => context.typeNodes.value?.filter((n) => n.flags & TypeFlag.IsUnionWith).map((n) => n as SimpleType) ?? []
);
const columnsInOrder: Ref<string[]> = computed(() => {
  if (isTable.value) {
    return allFields.value?.map((n) => n.name ?? "") ?? [];
  } else {
    return ["type", "value"];
  }
});
// map the field type to its actual runtime type
// children cannot be imputed into SimpleTypeNode but we still want to know the actual type
const selfSymbol = computed(() => symbolOf(context.statement.value.id));
function runtimeTypeOf(field: SimpleType) {
  return selfSymbol.value?.typeNodes?.find((n) => n.name == field.name) ?? field;
}
const extendedFields = computed(() => {
  if (extendedTypes.value.length == 0) {
    // shouldn't be needed but because interp state and module state are separate right now,
    // this prevents flickering changes at least if you're not using unions
    return [];
  }
  return (
    selfSymbol.value?.typeNodes
      ?.filter((n) => !selfFields.value.find((f) => f.name == n.name))
      .map((n) => n as SimpleType) ?? []
  );
});
const allFields = computed(() => [...selfFields.value, ...extendedFields.value]);

// grid & grid sizing
const grid = useNavigationGrid<string, InstanceType<typeof InlineTypeCell> | InstanceType<typeof InlineValueCell2>>(
  columnsInOrder,
  computed(() => {
    if (isTable.value) {
      // one row for fields, then values
      return [{ id: "" }, ...recordsInView.value];
    } else {
      // one row for every field
      return allFields.value;
    }
  }),
  {
    gridNavigateUp: focusDescriptionFromBottom,
    gridNavigateDown: () => (loadMoreRef.value ?? addRecordRef.value ?? addFieldRef.value)?.focus(),
  }
);

const verticalBorders = true;
const maxRowHeight = 250;
const growColumns = true;
const columnWidths: Ref<number[]> = ref([]);
// auto size columns
watchEffect(() => {
  const minTotalWidth = appearance.contentWidth;
  const ifaces = allFields.value.map((f) => getInterface(f));

  // init width to minimum widths as min(header, iface_min)
  const widths: number[] = [];
  for (let i = 0; i < allFields.value.length; i++) {
    const iface = ifaces[i];
    const headerWidth = (grid.getRef("", allFields.value[i].name ?? "")?.previewSize?.width.value ?? 50) + 16; // little padding
    const minWidth = Math.max(headerWidth, iface?.minWidth ?? 50);
    widths.push(minWidth);
  }

  // if the total width is too small, scale up by the grow factors
  const totalWidth = widths.reduce((a, b) => a + b, 0);
  if (totalWidth < minTotalWidth && growColumns) {
    const toFill = Math.max(minTotalWidth - totalWidth, 0);
    const growFactors = ifaces.map((i) => i?.grow ?? 0.1);
    const growTotal = growFactors.reduce((a, b) => a + b, 0);
    const growWidths = growFactors.map((g) => (g / growTotal) * toFill);
    for (let i = 0; i < widths.length; i++) {
      widths[i] += growWidths[i];
    }
  } else if (totalWidth > minTotalWidth) {
    // TODO @UX: if the total width is bigger, should indicate that the column at border is ~half occluded
  }

  columnWidths.value = widths;
});

function getRowHeight(id: string): number {
  /* Gets the maximum value of any elements in the row */
  const maxInnerHeight = grid
    .getColumn(id)
    .map((e) => e.previewSize?.height.value ?? 0)
    .reduce((a, b) => Math.max(a, b), 0);
  return Math.min(maxInnerHeight, maxRowHeight);
}

const extendedTypesRefs = useElementRefs<InstanceType<typeof InlineTypeCell>>();
const extendButtonRef: Ref<HTMLButtonElement | null> = ref(null);

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
  if (grid.refs.value.length > 0) {
    grid.focus(0, columnsInOrder.value[0]);
  } else {
    focusFirstRecord();
  }
}

function focusFirstRecord() {
  if (grid.refs.value.length > 0) {
    grid.focus(0, columnsInOrder.value[0]);
  } else {
    (addRecordRef.value ?? addFieldRef.value)?.focus();
  }
}

function focusLastRecord() {
  if (grid.refs.value.length > 0) {
    grid.focus(-1, columnsInOrder.value[0]);
  } else {
    descriptionRef.value?.focus();
  }
}

const ops = useOperations();

function insertField(isUnionWith?: boolean) {
  const nextOrderKey = generateKeyBetween(
    context.typeNodes.value?.[context.typeNodes.value?.length - 1 ?? 0]?.orderKey ?? INTEGER_ZERO,
    null
  );
  if (!isUnionWith) {
    context.createTypeNode(
      makeTypeNode({
        name: "field " + selfFields.value?.length,
        tag: TypeTag.String,
        orderKey: nextOrderKey,
      })
    );
    nextTick(() => grid.focus(0, selfFields.value.find((f) => f.orderKey == nextOrderKey)?.name ?? ""));
  } else {
    context.createTypeNode(
      makeTypeNode({
        name: "",
        tag: TypeTag.TypeReference,
        orderKey: nextOrderKey,
        flags: TypeFlag.IsUnionWith,
      })
    );
    nextTick(() => extendedTypesRefs.focus(extendedTypes.value.slice(-1)[0].id));
  }
}

function duplicateField(fieldId: string) {
  // :DuplicateTypeNode
  const fieldIdx = selfFields.value?.findIndex((m) => m.id === fieldId);
  if (fieldIdx < 0) return;
  const field = selfFields.value?.[fieldIdx];
  const orderKey = generateKeyBetween(field?.orderKey ?? null, selfFields.value?.[fieldIdx + 1]?.orderKey ?? null);
  // "name" => "name 2", "name 2" => "name 3", etc.
  const newName =
    field.name?.replace(/(\d+)?$/, (_, num) => (parseInt(num ?? "1") + 1).toString()) ?? field.name + " 2";
  const newFieldNode = {
    ...field,
    id: newTypeNodeId(),
    name: newName,
    orderKey,
    referenceId: field.reference?.id,
  };
  context.createTypeNode(newFieldNode);
  nextTick(() => grid.focus(fieldIdx + 1, "type"));
}

function updateFieldType(node: SimpleType, changed: SimpleType) {
  context.updateTypeNode(node, changed);
}

function deleteField(node: SimpleType) {
  const fieldIdx = selfFields.value?.findIndex((n) => n.id === node.id);
  context.deleteTypeNode(node);
  grid.focus(fieldIdx - 1, "name");
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
  nextTick(() => grid.focus(-1, columnsInOrder.value[0]));
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
  if (!isTable.value) return; // can't delete the main record
  const recordIdx = recordsInView.value.findIndex((r) => r.id === recordId);
  if (recordIdx < 0) throw new Error("record not found: " + recordId);
  ops.symbol.softDeleteRecord(null, recordId);
  // move focus up
  grid.focus(recordIdx - 1, columnsInOrder.value[0]);
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

const extraStatementActions = computed(() => {
  const inlineActions: StatementAction[] = [];
  if (isTable.value) {
    if ((fetchedRecords?.value?.statement?.records.totalCount ?? -1) == -1) {
      inlineActions.push({
        label: "Reload view",
        icon: ArrowPathIcon,
        active: loading.value,
        action: () => refetch(),
      });
    }
    inlineActions.push({
      label: "Add record",
      icon: PlusIcon,
      action: () => insertRecord(),
    });
  }
  inlineActions.push({
    label: "Extend type",
    icon: CubeTransparentIcon,
    action: () => insertField(true),
  });
  inlineActions.push({
    label: "Add field",
    icon: SquaresPlusIcon,
    action: () => insertField(),
  });
  return inlineActions;
});

const recordActions: RecordAction[] = [
  {
    label: "Duplicate",
    icon: Square2StackIcon,
    action: (record: any) => insertRecord(record.id),
  },
  {
    label: "Delete",
    icon: TrashIcon,
    action: (record: any) => deleteRecord(record.id),
  },
];

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
    addRecordRef.value?.blur();
    addFieldRef.value?.blur();
    grid.blur();
  },
  // prevent outer drag and drop while inside grid
  innerDrag: computed(() => !position.isOutside.value),
});
</script>
<template>
  <!-- Declaration -->
  <div class="flex flex-row justify-between">
    <div class="flex flex-row items-baseline">
      <DeclarationCell
        ref="declarationRef"
        @navigate-down="focusDescriptionFromTop"
        @navigate-right="
          extendedTypes.length > 0 ? extendedTypesRefs.focus(extendedTypes[0].id) : extendButtonRef?.focus()
        "
      />
      <!-- Extended types -->
      <div class="ml-1" v-if="(extendedTypes?.length ?? 0) > 0">
        <span class="mr-1 text-orange-600">has</span>
        <div class="inline-flex flex-row gap-x-1">
          <InlineTypeCell
            v-for="field of extendedTypes"
            :ref="(el: any) => extendedTypesRefs.registerRef(field.id, el)"
            :model-value="field"
            @update:model-value="(val) => context.updateTypeNode(field, val)"
            @delete-self="context.deleteTypeNode(field)"
            @navigate-left="
              field.id == extendedTypes[0].id
                ? declarationRef?.focus()
                : extendedTypesRefs.focus(extendedTypes[extendedTypes.findIndex((n) => n.id == field.id) - 1].id)
            "
            @navigate-right="
              field.id == extendedTypes[extendedTypes.length - 1].id
                ? extendButtonRef?.focus()
                : extendedTypesRefs.focus(extendedTypes[extendedTypes.findIndex((n) => n.id == field.id) + 1].id)
            "
            @navigate-down="focusDescriptionFromTop"
            @navigate-up="context.navigateUp"
            :active="context.focused.value || context.editing.value"
            :key="field.id"
            :readonly="context.readonly.value"
            structref-only
            hide-flags
            hide-icon
            class="w-full rounded-sm border border-transparent border-opacity-[15%] focus-within:border-solid focus-within:border-orange-900 focus-within:bg-orange-100 hover:bg-orange-100"
          />
        </div>
      </div>
      <!-- Inline buttons -->
      <button
        v-if="!context.readonly.value && (extendedTypes?.length ?? 0) <= 1"
        ref="extendButtonRef"
        @keydown.down.exact.prevent="focusDescriptionFromTop"
        @keydown.up.exact.prevent="context.navigateUp"
        @keydown.left.exact.prevent="
          extendedTypes.length > 0 ? extendedTypesRefs.focus(extendedTypes[0].id) : declarationRef?.focus()
        "
        tabindex="-1"
        @click="() => insertField(true)"
        class="ml-2 w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 focus:outline-none group-focus-within/statement:text-gray-400"
      >
        +type
      </button>
      <button
        tabindex="-1"
        v-if="description.length == 0 && !context.readonly.value && !addingDescription"
        @click="
          addingDescription = true;
          descriptionRef?.focus();
        "
        class="ml-2 w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 focus:outline-none group-focus-within/statement:text-gray-400"
      >
        +description
      </button>
    </div>
    <div
      class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
    >
      <!-- Only display total count if we know it (auto-set to -1 once we get sync events) -->
      <span v-if="(fetchedRecords?.statement?.records.totalCount ?? -1) > 0 && isTable" class="text-gray-400">
        {{ humanizeNumber(fetchedRecords?.statement?.records.totalCount ?? 0) }}
      </span>
      <InlineActions :extraActions="extraStatementActions" />
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
  <!-- Table (in table form but manually sized) -->
  <!-- Wrapper to contain any scrolling -->
  <div v-if="isTable" class="w-full overflow-x-auto" :style="appearance.contentWidthAsMaxWidth">
    <div ref="gridRef" class="-mx-1 flex w-full min-w-fit flex-col">
      <!-- Header (with types) -->
      <div class="flex flex-row border-b border-orange-900 border-opacity-[12%]">
        <div v-for="(field, x) in allFields" :key="field?.id" class="">
          <div class="flex flex-row gap-0.5 whitespace-nowrap focus-within:bg-orange-100">
            <InlineTypeTupleCell
              :ref="(el: any) => grid.registerColumnRef('', field.name as string, el)"
              :type="field"
              :readonly="context.readonly.value || extendedFields.find((n) => n.name == field.name) != null"
              :inlined="extendedFields.find((n) => n.name == field.name) != null"
              class="h-full w-full border border-transparent p-1 text-gray-400 focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
              :model-value="field"
              @update:model-value="(node: any) => updateFieldType(field, node)"
              @navigate-left="grid.navigateLeft('', field.name as string)"
              @navigate-right="grid.navigateRight('', field.name as string)"
              @navigate-up="grid.navigateUp('', field.name as string)"
              @navigate-down="grid.navigateDown('', field.name as string)"
              @delete-self="deleteField(field)"
              @duplicate-self="duplicateField(field.id)"
              :style="{
                width: columnWidths[x] + 'px',
              }"
            />
          </div>
        </div>
      </div>
      <!-- Records -->
      <div
        v-for="record in recordsInView"
        :key="record.id"
        class="group/record flex flex-row border-b border-orange-900 border-opacity-[12%] align-top"
      >
        <!-- Record action -->
        <div class="absolute -left-5 mt-1">
          <ActionPopover v-if="!context.readonly.value" v-slot="{ open }" :thing="record" :actions="recordActions">
            <Squares2X2Icon
              class="h-4 w-4 text-gray-400"
              :class="[open ? '' : 'opacity-0 transition-opacity focus:opacity-100 group-hover/record:opacity-100']"
            />
          </ActionPopover>
        </div>
        <!-- Record values -->
        <div
          v-for="(field, x) in allFields"
          :key="record.id + '.' + field?.id"
          class="h-full min-h-[32px] overflow-hidden"
          :class="[verticalBorders && x > 0 ? 'border-l border-orange-900 border-opacity-[12%]' : '']"
          :style="{
            'height': getRowHeight(record.id as string) + 8 + 'px',
            'width': columnWidths[x] + 'px',
          }"
        >
          <InlineValueCell2
            :ref="(el: any) => grid.registerColumnRef(record.id, field.name as string, el)"
            :model-value="record.data?.[field.key as string]"
            @update:model-value="(val) => writeRecordField(record.id, field.key as string, val)"
            :type="runtimeTypeOf(field)"
            :readonly="context.readonly.value"
            :active="context.editing.value || context.focused.value"
            debounced
            :supports-drop="!context.readonly.value"
            @drop-files="(p, v) => onDropFiles(record.id, field.name as string, p, v)"
            @navigate-left="grid.navigateLeft(record.id, field.name as string)"
            @navigate-right="grid.navigateRight(record.id, field.name as string)"
            @navigate-up="grid.navigateUp(record.id, field.name as string)"
            @navigate-down="grid.navigateDown(record.id, field.name as string)"
            @delete-self="deleteRecordField(record.id, field.key)"
            class="h-full w-full overflow-hidden border border-transparent px-1 py-1 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
            :style="{ 'max-height': maxRowHeight + 'px' }"
          />
        </div>
      </div>
    </div>
  </div>
  <!-- Single value (vertical) -->
  <table ref="gridRef" v-else class="-mx-1 w-full table-fixed">
    <!-- TODO @Cleanup: restructure table/value views to reduce duplication -->
    <tr
      v-for="(field, y) in allFields"
      :key="field.id"
      :class="[y < allFields.length - 1 ? 'border-b border-orange-900 border-opacity-[12%]' : '']"
    >
      <td class="w-1/4">
        <InlineTypeTupleCell
          :ref="(el: any) => grid.registerColumnRef(field?.id, 'type', el)"
          :type="field"
          :readonly="context.readonly.value || extendedFields.find((n) => n.name == field.name) != null"
          :inlined="extendedFields.find((n) => n.name == field.name) != null"
          class="min-h-fit w-full self-start border border-transparent px-1 py-0.5 text-gray-400 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
          :model-value="field"
          @update:model-value="(node: any) => updateFieldType(field, node)"
          @navigate-up="grid.navigateUp(field?.id, 'type')"
          @navigate-down="grid.navigateDown(field?.id, 'type')"
          @navigate-right="grid.navigateRight(field?.id, 'type')"
          @navigate-left="grid.navigateLeft(field?.id, 'type')"
          @delete-self="deleteField(field)"
          @duplicate-self="duplicateField(field.id)"
          :style="{
            'height': getRowHeight(field.id as string) + 8 + 'px',
          }"
        />
      </td>
      <!-- main record should always exist but just in case? -->
      <td v-if="mainRecord">
        <InlineValueCell2
          :ref="(el: any) => grid.registerColumnRef(field.id, 'value', el)"
          :model-value="mainRecord.data?.[field.key as string]"
          @update:model-value="(val) => writeRecordField(mainRecord.id, field.key as string, val)"
          :type="runtimeTypeOf(field)"
          :readonly="context.readonly.value"
          :active="context.editing.value || context.focused.value"
          debounced
          :supports-drop="!context.readonly.value"
          @drop-files="(p, v) => onDropFiles(mainRecord.id, field.name as string, p, v)"
          @delete-self="deleteRecordField(mainRecord.id, field.key as string)"
          @navigate-up="grid.navigateUp(field.id, 'value')"
          @navigate-down="grid.navigateDown(field.id, 'value')"
          @navigate-right="grid.navigateRight(field.id, 'value')"
          @navigate-left="grid.navigateLeft(field.id, 'value')"
          class="h-full w-full self-start border border-transparent px-1 py-0.5 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
          :class="[verticalBorders ? 'border-l border-orange-900 border-opacity-[12%]' : '']"
          :style="{ 'max-height': maxRowHeight + 'px' }"
        />
      </td>
    </tr>
  </table>
  <!-- Load more -->
  <div class="my-1 flex flex-row gap-2">
    <button
      v-if="pageInfo?.hasNextPage && isTable"
      @click="loadMore()"
      @keydown.up.exact.prevent="focusLastRecord"
      @keydown.right.exact.prevent="addRecordRef?.focus"
      @keydown.down.exact.prevent="context.navigateDown"
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
      v-if="!context.readonly.value && isTable"
      tabindex="-1"
      ref="addRecordRef"
      class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="insertRecord()"
      @enter="insertRecord()"
      @keydown.up.exact.prevent="focusLastRecord"
      @keydown.right.exact.prevent="addFieldRef?.focus"
      @keydown.left.exact.prevent="loadMoreRef?.focus"
      @keydown.down.exact.prevent="context.navigateDown"
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
      @keydown.up.exact.prevent="focusLastRecord"
      @keydown.left.exact.prevent="addRecordRef?.focus"
      @keydown.down.exact.prevent="context.navigateDown"
    >
      +field
    </button>
  </div>
</template>
