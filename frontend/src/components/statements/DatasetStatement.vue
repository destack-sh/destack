<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import DragHandleIcon from "@/components/basic/DragHandleIcon.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { getInterface } from "@/components/inputs";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import TypedDeclarationCell from "@/components/statements/TypedDeclarationCell.vue";
import InlineActions from "@/components/statements/InlineActionsCell.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { humanizeNumber } from "@/composables/useNow";
import { useActiveScroll } from "@/composables/useScroll";
import { graphql } from "@/gql";
import { TypeTag } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useEditorContext, type RecordAction, type StatementAction, type StatementHeader } from "@/state/bench";
import { useMagicActions } from "@/state/file";
import { TypeFlag } from "@/state/module";
import { useOperations } from "@/state/operations";
import { newDatasetRecordId, newFieldId, newFieldKey } from "@/state/operations/statement";
import { makeField, useStatementContext, type Field } from "@/state/statement";
import { generateKeyBetween, generateNKeysBetween, INTEGER_ZERO } from "@/utils/fractional";
import {
  ArrowDownIcon,
  ArrowPathIcon,
  CubeTransparentIcon,
  PlusIcon,
  Square2StackIcon,
  SquaresPlusIcon,
  TrashIcon,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { onStartTyping, useElementBounding, useMouseInElement, useScroll } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const context = useStatementContext();
const PAGE_SIZE = context.standalone.value ? 50 : 25;
const editorView = useEditorContext();
const addingDescription = ref(false);

const appearance = useAppearance();
const declarationRef: Ref<InstanceType<typeof TypedDeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);
const gridRef: Ref<HTMLDivElement | null> = ref(null);
const loadMoreRef: Ref<HTMLButtonElement | null> = ref(null);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);

const {
  loading,
  result: fetchedRecords,
  refetch,
  fetchMore,
} = useQuery(
  graphql(/* GraphQL */ `
    query searchRecords($statementId: GlobalID!, $after: String, $first: Int) {
      searchRecords(statementId: $statementId, after: $after, first: $first) {
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
  `),
  {
    statementId: computed(() => context.statement.value.id),
    after: null as string | null,
    first: PAGE_SIZE + 1, // overfetch by one to get order key for next page
  }
);
const pageInfo = computed(() => fetchedRecords.value?.searchRecords.pageInfo);
const recordsInView = computed(
  () =>
    fetchedRecords.value?.searchRecords.edges
      .slice(0, pageInfo.value?.hasNextPage ? -1 : undefined)
      .map((e) => e.node)
      .filter((n) => n.deletedAt == null)
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? []
);
const lastRecordInView = computed(() => recordsInView.value?.[recordsInView.value.length - 1]);
const overfetchedRecord = computed(() =>
  pageInfo.value?.hasNextPage ? fetchedRecords.value?.searchRecords.edges.slice(-1)[0]?.node : null
);

function loadMore() {
  if (!pageInfo.value?.hasNextPage) return;
  fetchMore({
    variables: {
      after: fetchedRecords.value?.searchRecords.edges.slice(-1)[0]?.cursor,
      first: PAGE_SIZE, // no need to overfetch again, already have 1 extra
    },
  });
}

// grid & grid sizing

const columnsInOrder: Ref<string[]> = computed(() => context.allFields.value?.map((n) => n.key ?? "") ?? []);
const rowIdsInOrder: Ref<string[]> = computed(() => recordsInView.value?.map((r) => r.id) ?? []);
const grid = useNavigationGrid<string, InstanceType<typeof FieldInterface> | InstanceType<typeof ValueInterface>>(
  columnsInOrder,
  computed(() => {
    // one row for fields, then values
    return [{ id: "" }, ...recordsInView.value];
  }),
  {
    gridNavigateUp: focusDescriptionFromBottom,
    gridNavigateDown: () => (loadMoreRef.value ?? addRecordRef.value)?.focus(),
  }
);

const verticalBorders = true;
const minRowHeight = 32; // incl. padding * 2
const rowPadding = 4;
const maxRowHeight = 220;
const growColumns = true;
const columnWidths: Ref<number[]> = ref([]);
const rowHeights: Ref<number[]> = ref([]);
const gridOffsetX: Ref<number> = computed(() => {
  if (editorView.size.value.width > editorView.editor.value.contentWidthWithMargin) {
    return (editorView.size.value.width - editorView.editor.value.contentWidth) / 2;
  } else {
    return editorView.editor.value.contentMarginX;
  }
});
// auto size columns and rows
// manual dependency tracking to prevent recursive updates
// TODO @Robustness @UX @Performance: grid resizing sometimes loops and becomes recursive
//  many :ref are re-triggered, calling registerColumnRef, triggering grid.refsByColumn below..
//  Sometimes this is annoying because it causes noticable lags when editing, especially when adding rows or modifying columns.
//  Only triggering ref updates (on preview size and on widths/heights) on value changes & batching column updates seems to fix this.
watch(
  () => [
    editorView.editor.value.contentWidth,
    editorView.editor.value.contentMarginX,
    context.xOffset,
    editorView.size.value,
    context.allFields.value,
    Object.values(grid.refsByColumn.value).map((r) => [r.previewSize.width.value, r.previewSize.height.value]),
  ],
  () => {
    // update column widths
    const targetMinTotalWidth =
      Math.min(
        editorView.size.value.width - editorView.editor.value.contentMarginX * 2,
        editorView.editor.value.contentWidth
      ) -
      context.xOffset.value -
      8; // not sure why -8, probably some mx-1? borders?
    const ifaces = context.allFields.value.map((f) => getInterface(f));
    // init width to minimum widths as min(header, iface_min)
    const widths: number[] = [];
    for (let i = 0; i < context.allFields.value.length; i++) {
      const iface = ifaces[i];
      const headerWidth = (grid.getRef("", context.allFields.value[i].key ?? "")?.previewSize.width.value ?? 50) + 16; // little padding
      const minWidth = Math.max(headerWidth, iface?.minWidth ?? 50);
      widths.push(minWidth);
    }
    // if the total width is too small, scale up to fill by the grow factors
    const minTotalWidth = widths.reduce((a, b) => a + b, 0);
    if (minTotalWidth < targetMinTotalWidth && growColumns) {
      const toFill = Math.max(targetMinTotalWidth - minTotalWidth, 0);
      const growFactors = ifaces.map((i) => i?.grow ?? 0.1);
      const growTotal = growFactors.reduce((a, b) => a + b, 0);
      const growWidths = growFactors.map((g) => (g / growTotal) * toFill);
      for (let i = 0; i < widths.length; i++) {
        widths[i] += growWidths[i];
      }
    }
    // update row heights
    const heights: number[] = rowIdsInOrder.value
      .map((r) =>
        grid
          .getColumn(r)
          .map((e) => e.previewSize.height.value ?? 0)
          .reduce((a, b) => Math.max(a, b), minRowHeight - rowPadding * 2)
      )
      .map((h) => Math.min(h, maxRowHeight));
    // update if changed (only trigger DOM update if necessary)
    if (widths.some((w, i) => w != columnWidths.value[i]) || heights.some((h, i) => h != rowHeights.value[i])) {
      columnWidths.value = widths;
      rowHeights.value = heights;
    }
  }
);

const gridBounding = useElementBounding(gridRef);
const gridScroll = useScroll(gridRef);
const gridScrollOffsetX = computed(() => gridScroll.x.value);
const headerOffsetY = computed(() => {
  // sticky the header to the top if the grid is partially visible (top of editor viewport)
  const editorTop = editorView.pos.value.top + appearance.editorHeaderHeight;
  if (gridBounding.top.value < editorTop && gridBounding.bottom.value > editorTop) {
    return editorTop - gridBounding.top.value;
  } else {
    return 0;
  }
});

// navigation
useActiveScroll(gridRef);

// auto-edit value field if starting to type (clear & focus)
onStartTyping((e) => {
  if (context.readonly.value) return;
  const cell = grid.findRef((r) => r.$el.parentNode.contains(e.target));
  if (cell != null && cell.rowId != "") {
    const field = context.allFields.value.find((f) => f.key == cell.column);
    deleteRecordField(cell.rowId, field?.key as string);
    nextTick(() => cell.ref.edit?.());
  }
});

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
    addRecordRef.value?.focus();
  }
}

function focusLastRecord() {
  if (grid.refs.value.length > 0) {
    grid.focus(-1, columnsInOrder.value[0]);
  } else {
    descriptionRef.value?.focus();
  }
}

// state

const ops = useOperations();

function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "reference" | "metadata">) {
  grid.beginBatchChange();
  const field = context.createNewField(template);
  nextTick(() => {
    grid.focus("", field.key ?? "");
    grid.flush();
  });
}

function createUnionField() {
  grid.beginBatchChange();
  context.createUnionField();
  nextTick(() => {
    declarationRef.value?.focusLastBase();
    grid.flush();
  });
}

function duplicateField(fieldId: string) {
  const field = context.duplicateField(fieldId);
  if (field != null) {
    nextTick(() => grid.focus("", field.key ?? ""));
  }
}

function updateFieldType(key: string, changed: Field) {
  // we use key instead of id here because of the module.runtimeTypeOf hack (has different id, see above)
  const old = context.fields.value.find((n) => n.key == key);
  if (old == null) return;
  context.updateField(old, { ...changed, id: old.id });
}

function deleteField(node: Field) {
  const fieldIdx = context.selfFields.value?.findIndex((n) => n.id === node.id);
  grid.beginBatchChange();
  context.deleteField(node);
  grid.focus(fieldIdx - 1, "name");
  nextTick(() => grid.flush());
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

function dropField(droppedId: string, position: "above" | "below" | "right" | "left", fieldId: string) {
  const dropped = context.selfFields.value.find((n) => n.id == droppedId);
  const field = context.selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  moveField(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
  nextTick(() => grid.focus("", dropped.key ?? ""));
}

function getNewOrderKey(belowRecordId?: string) {
  const recordIdx = recordsInView.value.findIndex((r) => r.id === belowRecordId);
  const recordBelow = recordsInView.value[recordIdx + 1] ?? overfetchedRecord.value;
  return generateKeyBetween(recordsInView.value[recordIdx]?.orderKey ?? null, recordBelow?.orderKey ?? null);
}

function insertRecordAtEnd() {
  insertRecord({ belowRecordId: lastRecordInView.value?.id });
}

function insertRecord(options?: { belowRecordId?: string; data?: Record<string, any> }) {
  const orderKey = getNewOrderKey(options?.belowRecordId);
  const recordId = newDatasetRecordId();
  ops.symbol.createRecord(null, recordId, context.statement.value.id, orderKey, options?.data ?? ({} as any));
  nextTick(() => grid.focus(recordId, columnsInOrder.value[0]));
}

function writeRecordField(recordId: string, key: string, value: any) {
  const record = recordsInView.value.find((r) => r.id === recordId);
  if (record == null) throw new Error("record not found: " + recordId);
  const oldData = record?.data;
  const newData = { ...oldData, [key]: value };
  ops.symbol.updateRecord(null, context.statement.value.id, recordId, oldData, newData);
}

function deleteRecordField(recordId: string, key: string) {
  const record = recordsInView.value.find((r) => r.id === recordId);
  if (record == null) throw new Error("record not found: " + recordId);
  const oldData = record?.data;
  const newData = { ...oldData };
  delete newData[key];
  ops.symbol.updateRecord(null, context.statement.value.id, recordId, oldData, newData);
}

function deleteRecord(recordId: string) {
  const recordIdx = recordsInView.value.findIndex((r) => r.id === recordId);
  if (recordIdx < 0) throw new Error("record not found: " + recordId);
  ops.symbol.softDeleteRecord(null, context.statement.value.id, recordId);
  // move focus up
  grid.focus(recordIdx - 1, columnsInOrder.value[0]);
}

// drag & drop
const magic = useMagicActions(context.statement as Ref<StatementHeader>);
async function onDropFiles(recordId: string, column: string, position: "above" | "below", files: File[]) {
  console.log("drop insert files into dataset", recordId, column, position, files);
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
  await magic.insertFilesAsRecords(column, orderKeys, files);
}
const position = useMouseInElement(gridRef);

// actions

const extraStatementActions = computed(() => {
  const actions: StatementAction[] = [];
  if ((fetchedRecords?.value?.searchRecords.totalCount ?? -1) == -1) {
    actions.push({
      label: "Reload view",
      icon: ArrowPathIcon,
      active: loading.value,
      action: () => refetch(),
    });
  }
  actions.push({
    label: "Add record",
    icon: PlusIcon,
    action: () => insertRecordAtEnd(),
  });
  actions.push({
    label: "Add field",
    icon: SquaresPlusIcon,
    action: () => createFieldRef.value?.show(),
  });
  actions.push({
    label: "Include type",
    icon: CubeTransparentIcon,
    action: () => createUnionField,
  });
  return actions;
});

const recordActions: RecordAction[] = [
  {
    label: "Duplicate",
    icon: Square2StackIcon,
    action: (record: any) => insertRecord({ belowRecordId: record.id, data: JSON.parse(JSON.stringify(record.data)) }),
  },
  {
    label: "Delete",
    icon: TrashIcon,
    action: (record: any) => deleteRecord(record.id),
  },
];

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    if (position == "first") {
      declarationRef.value?.focus();
    } else {
      (loadMoreRef.value ?? addRecordRef.value)?.focus();
    }
  },
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
    addRecordRef.value?.blur();
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
      <TypedDeclarationCell
        ref="declarationRef"
        @navigate-down="focusDescriptionFromTop"
        @navigate-up="context.navigateUp"
        @add-base="createUnionField"
      />
    </div>
    <div
      class="flex flex-row items-center gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
    >
      <!-- Only display total count if we know it (auto-set to -1 once we get sync events) -->
      <span v-if="(fetchedRecords?.searchRecords.totalCount ?? -1) > 0" class="text-gray-400">
        {{ humanizeNumber(fetchedRecords?.searchRecords.totalCount ?? 0) }}
      </span>
      <InlineActions :extraActions="extraStatementActions" />
      <CreateFieldInterface
        ref="createFieldRef"
        :title="'New field on ' + context.statement.value.name"
        @select="createNewField"
      />
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
  <div
    ref="gridRef"
    class="overflow-x-auto"
    :style="{
      'margin-left': -gridOffsetX + 'px',
      'margin-right': -gridOffsetX + 'px',
      'padding-left': gridOffsetX + 'px',
      'padding-right': gridOffsetX + 'px',
      'max-width': editorView.size.value.width + 'px',
    }"
  >
    <div class="-mx-1 flex min-w-fit flex-col">
      <!-- Header placeholder -->
      <div
        :style="{
          width: columnWidths.reduce((a, b) => a + b, 0) + 'px',
          height: minRowHeight - 1 + 'px',
        }"
      ></div>
      <!-- Header (with types) -->
      <!-- To make this 'sticky' without creating a new stacking context we position it absolutely 'above' the placeholder above  -->
      <!-- TODO @Broken: header pokes out of containing editor view (because it's fixed) -->
      <div
        class="z-[1] flex flex-row self-start border-b border-orange-900 border-opacity-[12%]"
        :style="{
          position: headerOffsetY == 0 ? 'absolute' : 'fixed',
          left:
            headerOffsetY == 0
              ? -gridScrollOffsetX + 4 + 'px'
              : -gridScrollOffsetX + editorView.pos.value.left + gridOffsetX + 'px',
          top: headerOffsetY == 0 ? undefined : editorView.pos.value.top + appearance.editorHeaderHeight + 'px',
        }"
      >
        <div v-for="(field, x) in context.allFields.value" :key="field?.id" class="">
          <div class="flex flex-row gap-0.5 whitespace-nowrap focus-within:bg-orange-100">
            <FieldInterface
              :ref="(el: any) => grid.registerColumnRef('', field.key as string, el)"
              :key="field?.id + '.header'"
              :type="field"
              :readonly="
                context.readonly.value || context.inheritedFields.value.find((n) => n.key == field.key) != null
              "
              :inlined="context.inheritedFields.value.find((n) => n.key == field.key) != null"
              orientation="horizontal"
              class="h-full w-full border border-transparent p-1 text-gray-400 focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
              :model-value="field"
              @update:model-value="(node: any) => updateFieldType(field.key, node)"
              @navigate-left="grid.navigateLeft('', field.key as string)"
              @navigate-right="grid.navigateRight('', field.key as string)"
              @navigate-up="grid.navigateUp('', field.key as string)"
              @navigate-down="grid.navigateDown('', field.key as string)"
              @delete-self="deleteField(field)"
              @duplicate-self="duplicateField(field.id)"
              @drop="(p, v) => dropField(v.id, p, field.id)"
              @enter="grid.navigateDown('', field.key as string)"
              :style="{
                width: columnWidths[x] + 'px',
              }"
            />
          </div>
        </div>
      </div>
      <!-- Records -->
      <div
        v-for="(record, y) in recordsInView"
        :key="record.id"
        class="group/record relative flex flex-row self-start border-b border-orange-900 border-opacity-[12%] align-top"
      >
        <!-- Record actions -->
        <div class="absolute -left-1 mt-1">
          <div class="relative">
            <div class="absolute right-0 flex flex-row-reverse items-baseline gap-0.5">
              <!-- Standard actions -->
              <ActionPopover
                v-if="!context.readonly.value"
                anchor="right"
                v-slot="{ open }"
                :thing="record"
                :actions="recordActions"
              >
                <DragHandleIcon
                  class="h-4 w-4 text-gray-400 hover:text-gray-700"
                  :class="[
                    open
                      ? ''
                      : 'opacity-0 transition-opacity focus:opacity-100 group-focus-within/record:opacity-100 group-hover/record:opacity-100',
                  ]"
                />
              </ActionPopover>
              <!-- Insert record -->
              <button
                v-if="!context.readonly.value"
                class="rounded-sm p-0.5 text-gray-400 opacity-0 transition duration-150 hover:bg-orange-100 hover:text-gray-700 group-focus-within/record:opacity-100 group-hover/record:opacity-100"
                @click="() => insertRecord({ belowRecordId: record.id })"
              >
                <PlusIcon class="h-4 w-4" />
              </button>
            </div>
          </div>
        </div>
        <!-- Record values -->
        <div
          v-for="(field, x) in context.allFields.value"
          :key="record.id + '.' + field?.id"
          class="h-full overflow-hidden"
          :class="[verticalBorders && x > 0 ? 'border-l border-orange-900 border-opacity-[12%]' : '']"
          :style="{
            minHeight: minRowHeight + 'px',
            width: columnWidths[x] + 'px',
            height: rowHeights[y] + rowPadding * 2 + 'px',
          }"
        >
          <ValueInterface
            :ref="(el: any) => grid.registerColumnRef(record.id, field.key as string, el)"
            :model-value="record.data?.[field.key as string]"
            @update:model-value="(val) => writeRecordField(record.id, field.key as string, val)"
            :type="field"
            :readonly="context.readonly.value"
            :active="context.editing.value || context.focused.value"
            debounced
            :supports-drop="!context.readonly.value"
            @drop-files="(p, v) => onDropFiles(record.id, field.key as string, p, v)"
            @navigate-left="grid.navigateLeft(record.id, field.key as string)"
            @navigate-right="grid.navigateRight(record.id, field.key as string)"
            @navigate-up="grid.navigateUp(record.id, field.key as string)"
            @navigate-down="grid.navigateDown(record.id, field.key as string)"
            @delete-self="deleteRecordField(record.id, field.key)"
            class="h-full w-full overflow-hidden border border-transparent p-1 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
            :style="{ 'max-height': maxRowHeight + rowPadding * 2 + 'px' }"
          />
        </div>
      </div>
      <!-- Bottom actions -->
      <!-- Load more/loading -->
      <button
        v-if="pageInfo?.hasNextPage"
        class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900 border-opacity-[12%] px-1 py-1 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        :style="{ height: minRowHeight + 'px' }"
        @click.stop="loadMore()"
        @keydown.up.exact.prevent="focusLastRecord"
        @keydown.down.exact.prevent="context.navigateDown"
        :disabled="loading"
        ref="loadMoreRef"
      >
        <template v-if="loading">
          <ArrowPathIcon class="h-4 w-4" :class="loading ? 'animate-spin' : ''" />
          Loading
        </template>
        <template v-else>
          <ArrowDownIcon class="h-4 w-4" />
          Load {{ PAGE_SIZE }} more
        </template>
      </button>
      <!-- Insert button -->
      <button
        v-if="!context.readonly.value"
        class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900 border-opacity-[12%] px-1 py-1 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        :style="{ height: minRowHeight + 'px' }"
        @click.stop="insertRecordAtEnd()"
        @keydown.up.exact.prevent="(loadMoreRef?.focus ?? focusLastRecord)()"
        @keydown.down.exact.prevent="context.navigateDown"
        :disabled="loading"
        ref="loadMoreRef"
      >
        <PlusIcon class="h-4 w-4" /> New
      </button>
    </div>
  </div>
</template>
