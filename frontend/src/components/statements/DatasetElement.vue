<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import DragHandleIcon from "@/components/basic/DragHandleIcon.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { getInterface } from "@/components/inputs";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { useActiveScroll } from "@/composables/useScroll";
import { graphql } from "@/gql";
import {
  QueryOp,
  SortOrder,
  ModuleMutationType,
  TypeHint,
  type SearchSort,
  type SearchQuery,
  type SearchRecordsQueryVariables,
} from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { usePanelContext, useElementPanelSettings, type RecordAction, type StatementAction } from "@/state/bench";
import { useMagicActions } from "@/state/file";
import { useCurrentModule, type Field, newNodeIdentity, type Statement, type Record } from "@/state/module";
import { useOperations } from "@/state/operations";
import { useFields, type DatasetStatementProperties } from "@/state/statement";
import { generateKeyBetween, generateNKeysBetween } from "@/utils/fractional";
import { IS_DEBUG, IS_LOCALHOST } from "@/utils/globals";
import {
  ArrowDownIcon,
  ArrowPathIcon,
  EllipsisHorizontalIcon,
  MagnifyingGlassIcon,
  PlusIcon,
  Square2StackIcon,
  TrashIcon,
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
  XMarkIcon,
  SquaresPlusIcon,
  EllipsisVerticalIcon,
} from "@heroicons/vue/24/outline";
import { useApolloClient, useQuery } from "@vue/apollo-composable";
import { onStartTyping, useDebounceFn, useElementBounding, useMouseInElement, useScroll } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref, onMounted, toRef } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { canSort, getMainSubfield } from "@/state/type";
import { toValueRef } from "@/utils/functools";
import { useMutationListener } from "@/state/sync";
import { DateTime } from "luxon";
import { TypeTag } from "@/gql/graphql";
import { XCircleIcon as XCircleIconSolid } from "@heroicons/vue/24/solid";
import type { StatementEmit, StatementProps } from "@/components/statements";
import { RECORD_SEARCH_QUERY, useDatasetInlineSearch } from "@/state/dataset";
import { humanizeNumber } from "@/composables/useNow";
import { emptyConnection, getUpdatedConnectionQuery } from "@/utils/connection";

const PAGE_SIZE = 10;

const props =
  defineProps<Pick<StatementProps, "statement" | "focused" | "readonly" | "editing" | "xoffset" | "visible">>();
const emit = defineEmits<StatementEmit>();

const module = useCurrentModule();
const panel = usePanelContext();
const appearance = useAppearance();
const client = useApolloClient();

const gridRef: Ref<HTMLDivElement | null> = ref(null);
const innerGridRef: Ref<HTMLDivElement | null> = ref(null);
const loadMoreRef: Ref<HTMLButtonElement | null> = ref(null);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);

const fields = useFields(toRef(props, "statement"));
const { allFields, selfFields, inheritedFields } = fields;

const properties = useElementPanelSettings<DatasetStatementProperties>(toRef(props, "statement"), {
  inlineQuery: undefined,
  wrapColumns: false,
});

// reset inline query to undefined if it's empty on load
onMounted(() => {
  if ((properties.inlineQuery ?? "").trim().length == 0) {
    properties.inlineQuery = undefined;
  }
});

const { inlineQuery } = useDatasetInlineSearch(fields, toRef(properties, "inlineQuery"));

function addSort(field: Field, order: SortOrder) {
  const subkey = canSort(field, { excludeSubfields: true }) ? "" : "." + getMainSubfield(field);
  const key = "value." + module.getTypedKey(field) + subkey;
  if (properties.sorts == null) properties.sorts = [];
  // replace or append sort
  const oldIndex = properties.sorts.findIndex((s) => s.key == key);
  if (oldIndex >= 0) {
    properties.sorts.splice(oldIndex, 1, { key, order });
  } else {
    properties.sorts.push({ key, order });
  }
}
function removeSort(sort: { key: string }) {
  properties.sorts = properties.sorts?.filter((s) => !s.key.includes(sort.key));
}
const sort: Ref<SearchSort[] | null> = computed(() => {
  if (properties.sorts == null || properties.sorts.length == 0) return null;
  return properties.sorts;
});

// :QueryFieldPolicies
const searchQueryVariables: Ref<SearchRecordsQueryVariables> = computed(
  () =>
    ({
      statementId: props.statement.id,
      after: null as string | null,
      query: inlineQuery.value,
      sort: sort.value,
      limit: PAGE_SIZE,
      count: true,
    } as SearchRecordsQueryVariables)
);
const {
  loading: recordsLoading,
  result: recordsFetchedResult,
  error: recordsError,
  refetch,
  fetchMore,
} = useQuery(RECORD_SEARCH_QUERY, toValueRef(searchQueryVariables), {
  fetchPolicy: "network-only",
  enabled: computed(() => !module.loading.value && props.visible) as any, // the vue composable typing is all fucked up
});
const pageInfo = computed(() => recordsFetchedResult.value?.searchRecords.pageInfo);
const totalCount = computed(() => recordsFetchedResult.value?.searchRecords.totalCount);
const recordsFetched = computed(() => recordsFetchedResult.value?.searchRecords.edges.map((e) => e.node) ?? []);
const loading = computed(
  () => (recordsFetchedResult.value == null || recordsLoading.value) && recordsError.value == null
);

const recordsInView = computed(() => recordsFetched.value.filter((n) => n.deletedAt == null));

// auto refetch when bumped (1s is the OS indexing delay)
const refetchDebounced = useDebounceFn(refetch, 1000, { maxWait: 10000 });
useMutationListener([ModuleMutationType.BumpStatement], props.statement.id, () => {
  refetchDebounced();
});
// trigger refetch (debounced) once if just created to autoload if the dataset was duplicated
onMounted(() => {
  const delta = DateTime.now().diff(DateTime.fromISO(props.statement.createdAt ?? ""));
  if (delta.as("seconds") < 1) {
    refetchDebounced();
  }
});

function loadMore() {
  if (!pageInfo.value?.hasNextPage) return;
  fetchMore({
    variables: {
      after: recordsFetchedResult.value?.searchRecords.edges.slice(-1)[0]?.cursor,
      limit: PAGE_SIZE,
    },
  });
}

// grid & grid sizing

const columnsInOrder: Ref<string[]> = computed(() => allFields.value?.map((n) => n.key ?? "") ?? []);
const rowIdsInOrder: Ref<string[]> = computed(() => recordsInView.value?.map((r) => r.id) ?? []);
const grid = useNavigationGrid<string, InstanceType<typeof FieldInterface> | InstanceType<typeof ValueInterface>>(
  columnsInOrder,
  computed(() => {
    // one row for fields, then values
    return [{ id: "" }, ...recordsInView.value];
  }),
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => (loadMoreRef.value ?? addRecordRef.value)?.focus(),
  }
);

const verticalBorders = true;
const minRowHeight = 32; // incl. padding * 2
const rowPadding = 4;
const maxRowHeight = 220;
const defaultGrowFactor = 0.1;
const defaultMinWidth = 50;
const growColumns = true;
const showPropertiesColumn = true; // used to be only in write mode, but useful if no columns and for settings shortcut
const propertiesColumnWidth = 40;
const columnWidths: Ref<number[]> = ref([]);
const rowHeights: Ref<number[]> = ref([]);
const gridOffsetX: Ref<number> = computed(() => {
  if (panel.size.value.width > panel.panel.value.contentWidthWithMargin) {
    return (panel.size.value.width - panel.panel.value.contentWidth) / 2;
  } else {
    return panel.panel.value.contentMarginX;
  }
});
// auto size columns and rows
// manual dependency tracking to prevent recursive updates
// TODO @Robustness @UX @Performance: grid resizing sometimes loops and becomes recursive
//  many :ref are re-triggered, calling registerColumnRef, triggering grid.refsByColumn below..
//  Sometimes this is annoying because it causes noticable lags when editing, especially when adding rows or modifying columns.
//  Only triggering ref updates (on preview size and on widths/heights) on value changes & batching column updates seems to fix this (mostly).
watch(
  () => [
    properties.wrapColumns,
    panel.panel.value.contentWidth,
    panel.panel.value.contentMarginX,
    props.xoffset,
    panel.size.value,
    allFields.value,
    Object.values(grid.refsByColumn.value).map((r) => [r.previewSize.width.value, r.previewSize.height.value]),
  ],
  () => {
    // update column widths
    const targetMinTotalWidth =
      Math.min(panel.size.value.width - panel.panel.value.contentMarginX * 2, panel.panel.value.contentWidth) -
      props.xoffset -
      8; // from Statement interface
    const ifaces: ({ minWidth?: number; grow?: number } | undefined)[] = allFields.value.map((f) => getInterface(f));
    if (showPropertiesColumn) {
      ifaces.push({ minWidth: propertiesColumnWidth, grow: 0.05 }); // 'fake' properties column
    }

    // init width to minimum widths as min(header, iface_min)
    const widths: number[] = [];
    for (let i = 0; i < ifaces.length; i++) {
      const iface = ifaces[i];
      const headerWidth =
        i < columnsInOrder.value.length
          ? (grid.getRef("", columnsInOrder.value[i])?.previewSize.width.value ?? defaultMinWidth) + 16 // little padding
          : 0;
      const minWidth = Math.max(headerWidth, iface?.minWidth ?? defaultMinWidth);
      widths.push(minWidth);
    }
    // if the total width is too small, scale up to fill by the grow factors
    const minTotalWidth = widths.reduce((a, b) => a + b, 0);
    if (minTotalWidth < targetMinTotalWidth && growColumns) {
      const toFill = Math.max(targetMinTotalWidth - minTotalWidth, 0);
      const growFactors = ifaces.map((i) => i?.grow ?? defaultGrowFactor);
      const growTotal = growFactors.reduce((a, b) => a + b, 0);
      const growWidths = growFactors.map((g) => (g / growTotal) * toFill);
      for (let i = 0; i < widths.length; i++) {
        widths[i] += growWidths[i];
      }
    }

    let heights: number[];
    if (properties.wrapColumns) {
      // compute wrapped row heights
      heights = rowIdsInOrder.value
        .map((r) =>
          grid
            .getColumn(r)
            .map((e) => e.previewSize.height.value ?? 0)
            .reduce((a, b) => Math.max(a, b), minRowHeight - rowPadding * 2)
        )
        .map((h) => Math.min(h, maxRowHeight));
    } else {
      heights = rowIdsInOrder.value.map((r) => minRowHeight - rowPadding * 2);
    }

    // update if changed (only trigger DOM update if necessary)
    if (widths.some((w, i) => w != columnWidths.value[i]) || heights.some((h, i) => h != rowHeights.value[i])) {
      columnWidths.value = widths;
      rowHeights.value = heights;
    }
  },
  { immediate: allFields.value.length == 0 } // force update on first render if no columns to trigger initial sizing
);

const gridBounding = useElementBounding(gridRef);
const innerGridBounding = useElementBounding(innerGridRef);
const gridScroll = useScroll(gridRef);
const gridScrollOffsetX = computed(() => gridScroll.x.value);
const isHeaderRowFloating = computed(() => {
  // sticky the header to the top if the grid is partially visible (top of editor viewport)
  const editorTop = panel.pos.value.top + appearance.panelHeaderHeight;
  return gridBounding.top.value < editorTop && gridBounding.bottom.value - minRowHeight > editorTop;
});
const gridOverhangLeft = computed(() => {
  // how much the grid overhangs the left of the editor
  return Math.max(panel.pos.value.left - innerGridBounding.left.value, 0);
});
const gridOverhangRight = computed(() => {
  // how much the grid overhangs the right of the editor
  return Math.max(innerGridBounding.right.value - (panel.pos.value.left + panel.size.value.width), 0);
});

// navigation
useActiveScroll(gridRef);

// auto-edit value field if starting to type (clear & focus)
onStartTyping((e) => {
  if (props.readonly) return;
  const cell = grid.findRef((r) => r.$el.parentNode.contains(e.target));
  if (cell != null && cell.rowId != "") {
    const field = allFields.value.find((f) => f.key == cell.column);
    if (field == null) return;
    deleteRecordField(cell.rowId, module.getTypedKey(field) as string);
    nextTick(() => (cell.ref as unknown as { edit?: () => void }).edit?.());
  }
});

function focusLastRecord() {
  if (grid.refs.value.length > 0) {
    grid.focus(-1, columnsInOrder.value[0]);
  } else {
    emit("navigateUp");
  }
}

// state

const ops = useOperations();

function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "metadata"> & Partial<Field>) {
  grid.beginBatchChange();
  const field = fields.createNewField(template);
  nextTick(() => {
    grid.flush();
    nextTick(() => (grid.getRef("", field.key) as InstanceType<typeof FieldInterface>).open("all"));
  });
}

function duplicateField(fieldId: string) {
  const field = fields.duplicateField(fieldId);
  if (field != null) {
    nextTick(() => grid.focus("", field.key ?? ""));
  }
}

function updateField(key: string, changed: Field) {
  // we use key instead of id here because of the module.runtimeTypeOf hack (has different id, see above)
  // note: this was changed, not sure if it's still needed
  const old = allFields.value.find((n) => n.key == key);
  if (old == null) return;
  fields.updateField(old, { ...changed, id: old.id });
}

function deleteField(node: Field) {
  const fieldIdx = selfFields.value?.findIndex((n) => n.id === node.id);
  grid.beginBatchChange();
  fields.deleteField(node);
  grid.focus(fieldIdx - 1, "name");
  nextTick(() => grid.flush());
}

function moveField(field: Field, position: "before" | "after", other: Field) {
  const otherIndex = selfFields.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(selfFields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    fields.moveField(field, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, selfFields.value[otherIndex + 1]?.orderKey ?? null);
    fields.moveField(field, orderKey);
  }
}

function dropField(droppedId: string, position: "above" | "below" | "right" | "left", fieldId: string) {
  const dropped = selfFields.value.find((n) => n.id == droppedId);
  const field = selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  moveField(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
  nextTick(() => grid.focus("", dropped.key ?? ""));
}

function insertRecordAtEnd() {
  insertRecord({ belowRecordId: recordsInView.value?.[recordsInView.value.length - 1]?.id });
}

function insertRecord(options?: { belowRecordId?: string; value?: any }) {
  const identity = newNodeIdentity(module.id.value, "Record");
  ops.symbol.createRecord(null, identity.id, identity.ck, props.statement.id, null, options?.value ?? ({} as any));
  // add record to search results optimistically (regardless of filter)
  const recordRef = client.client.cache.identify({ __typename: "Record", id: identity.id });
  const optimisticRecord: Record = {
    __typename: "Record",
    id: identity.id,
    ck: identity.ck,
    revision: -1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    deletedAt: null,
    orderKey: null,
    value: options?.value ?? ({} as any),
  };
  client.client.cache.updateQuery(
    {
      query: RECORD_SEARCH_QUERY,
      variables: searchQueryVariables.value,
    },
    (data) => ({
      searchRecords: getUpdatedConnectionQuery<Record, "RecordConnection">(
        { __ref: recordRef, ...optimisticRecord } as any,
        data?.searchRecords ?? (emptyConnection<Record, "RecordConnection">("RecordConnection") as any),
        undefined,
        "end"
      ) as any,
    })
  );
  // focus new record (for some reason the good ol' nextTick alone doesn't work here)
  grid.onColumnAvailable(identity.id, columnsInOrder.value[0], (ref) => nextTick(ref.focus));
}

function writeRecordField(recordId: string, key: string, value: any) {
  const record = recordsInView.value.find((r) => r.id === recordId);
  if (record == null) throw new Error("record not found: " + recordId);
  const oldValue = record?.value;
  const newValue = { ...oldValue, [key]: value };
  ops.symbol.updateRecord(null, props.statement.id, recordId, oldValue, newValue);
}

function deleteRecordField(recordId: string, key: string) {
  const record = recordsInView.value.find((r) => r.id === recordId);
  if (record == null) throw new Error("record not found: " + recordId);
  const oldValue = record?.value;
  const newValue = { ...oldValue };
  // TODO @Robustness: figure out better way to clear field in opensearch backend (maybe update by query?)
  newValue[key] = [];
  ops.symbol.updateRecord(null, props.statement.id, recordId, oldValue, newValue);
}

function deleteRecord(recordId: string) {
  const recordIdx = recordsInView.value.findIndex((r) => r.id === recordId);
  if (recordIdx < 0) throw new Error("record not found: " + recordId);
  ops.symbol.softDeleteRecord(null, props.statement.id, recordId);
  grid.focus(recordIdx, columnsInOrder.value[0]);
}

// drag & drop
const magic = useMagicActions(toRef(props, "statement"));
async function onDropFiles(recordId: string, column: string, position: "above" | "below", files: File[]) {
  console.log("drop insert files into dataset", recordId, column, position, files);
  const recordIdx = recordsInView.value.findIndex((r) => r.id === recordId);
  const record = recordsInView.value[recordIdx];
  const above = recordsInView.value[recordIdx - 1];
  const below = recordsInView.value[recordIdx + 1];
  let orderKeys;
  if (position == "above") {
    orderKeys = generateNKeysBetween(above?.orderKey ?? null, record.orderKey ?? null, files.length);
  } else {
    orderKeys = generateNKeysBetween(record.orderKey ?? null, below?.orderKey ?? null, files.length);
  }
  await magic.insertFilesAsRecords(column, orderKeys, files);
}
const position = useMouseInElement(gridRef);

// actions

const actions = computed(() => {
  const actions: StatementAction[] = [];
  if (IS_LOCALHOST || IS_DEBUG) {
    actions.push({
      label: "Reload view",
      icon: ArrowPathIcon,
      active: loading.value,
      action: () => refetch(),
    });
  }
  actions.push({
    label: "Add field",
    groupId: "edit",
    icon: SquaresPlusIcon,
    disabled: props.readonly,
    action: () => {
      createFieldRef.value?.show();
    },
  });
  actions.push({
    label: "Add record",
    groupId: "edit",
    disabled: props.readonly,
    icon: PlusIcon,
    action: () => {
      insertRecordAtEnd();
    },
  });
  actions.push({
    label: properties.wrapColumns ? "Unwrap columns" : "Wrap columns",
    groupId: "nav",
    icon: properties.wrapColumns ? ChevronDoubleUpIcon : ChevronDoubleDownIcon,
    action: () => (properties.wrapColumns = !properties.wrapColumns),
    hideInline: true,
  });
  return actions;
});

const recordActions: RecordAction[] = [
  {
    label: "Duplicate",
    icon: Square2StackIcon,
    action: (record: any) =>
      insertRecord({ belowRecordId: record.id, value: JSON.parse(JSON.stringify(record.value)) }),
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
      if (grid.refs.value.length > 0) {
        grid.focus(0, columnsInOrder.value[0]);
      } else {
        addRecordRef.value?.focus();
      }
    } else {
      focusLastRecord();
    }
  },
  blur: () => {
    addRecordRef.value?.blur();
    grid.blur();
  },
  // prevent outer drag and drop while inside grid
  capturingDrag: computed(() => !position.isOutside.value),
  loading,
  actions,
});
</script>
<template>
  <div>
    <!-- Sorts/filters -->
    <div
      v-if="(properties.sorts ?? []).length > 0 || properties.query != null"
      class="-mx-0.5 mb-1 mt-1 flex flex-row flex-wrap gap-1.5"
    >
      <!-- Sort pills -->
      <span
        v-for="sort in properties.sorts ?? []"
        :key="sort.key"
        class="flex w-fit flex-row items-center rounded-xl border border-gray-300 px-1.5 text-gray-900"
      >
        <span class="">{{ allFields.find((f) => sort.key.includes(f.key))?.name }}</span>
        <span class="ml-0.5 text-gray-700">{{ sort.order == SortOrder.Ascending ? "↑" : "↓" }}</span>
        <!-- Clear button -->
        <button @click="removeSort(sort)">
          <XMarkIcon class="h-3 w-3 text-gray-400" />
        </button>
      </span>
    </div>
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
        'max-width': panel.size.value.width + 'px',
      }"
    >
      <!-- Inner grid -->
      <div ref="innerGridRef" class="-mx-1 flex min-w-fit flex-col">
        <!-- Header placeholder -->
        <div
          :style="{
            width: columnWidths.reduce((a, b) => a + b, 0) + 'px',
            height: minRowHeight - 1 + 'px',
          }"
        />
        <!-- Header (with types) -->
        <!-- To make this 'sticky' without creating a new stacking context we position it absolutely 'above' the placeholder above  -->
        <div
          class="z-[1] flex flex-row self-start border-b border-orange-900 border-opacity-[12%]"
          :class="(focused && !editing) || !isHeaderRowFloating ? '' : 'bg-white'"
          :style="{
            position: isHeaderRowFloating ? 'fixed' : 'absolute',
            left: isHeaderRowFloating
              ? -gridScrollOffsetX + 4 + panel.pos.value.left + gridOffsetX + 'px'
              : -gridScrollOffsetX + 4 + 'px',
            top: isHeaderRowFloating ? panel.pos.value.top + appearance.panelHeaderHeight - 2 + 'px' : undefined,
            /* clip to editor bounds (different stacking context so need to 're-clip' into editor) */
            clipPath: isHeaderRowFloating ? `inset(0px ${gridOverhangRight}px 0px ${gridOverhangLeft}px)` : undefined,
          }"
        >
          <div v-for="(field, x) in allFields" :key="field?.id" class="">
            <div class="whitespace-nowrap focus-within:bg-orange-100">
              <FieldInterface
                :ref="(el: any) => grid.registerColumnRef('', field.key as string, el)"
                :key="field?.id + '.header'"
                :type="field"
                :readonly="readonly"
                :ref-types="[TypeTag.Struct, TypeTag.Enum]"
                :inlined="inheritedFields.find((n) => n.key == field.key) != null"
                is-view
                hide-outline
                orientation="horizontal"
                class="h-full w-full border border-transparent p-1 text-gray-400 focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
                :model-value="field"
                @update:model-value="(node: any) => updateField(field.key, node)"
                @navigate-left="grid.navigateLeft('', field.key as string)"
                @navigate-right="grid.navigateRight('', field.key as string)"
                @navigate-up="grid.navigateUp('', field.key as string)"
                @navigate-down="grid.navigateDown('', field.key as string)"
                @delete-self="deleteField(field)"
                @duplicate-self="duplicateField(field.id)"
                @drop="(p, v) => dropField(v.id, p, field.id)"
                @sort="(order) => addSort(field, order)"
                @enter="grid.navigateDown('', field.key as string)"
                :style="{
                  width: columnWidths[x] + 'px',
                }"
              />
            </div>
          </div>
          <CreateFieldInterface
            ref="createFieldRef"
            :title="'New field'"
            :ref-types="[TypeTag.Enum, TypeTag.Struct]"
            @select="createNewField"
          />
          <!-- Properties column (add + settings) -->
          <div
            v-if="showPropertiesColumn"
            :style="{
              width: columnWidths[columnWidths.length - 1] + 'px',
            }"
          >
            <div class="flex h-full w-full flex-row items-center whitespace-nowrap">
              <!-- Add column -->
              <button
                v-if="!readonly"
                tabindex="-1"
                class="h-full rounded-sm p-1.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
                @click="createFieldRef?.show()"
              >
                <PlusIcon class="h-4 w-4" />
              </button>
              <!-- Properties -->
              <button
                tabindex="-1"
                class="h-full flex-1 rounded-sm p-1.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
                @click="emit('openActions')"
              >
                <EllipsisHorizontalIcon class="h-4 w-4" />
              </button>
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
          <div class="absolute -left-0.5 mt-1">
            <div class="relative">
              <div class="absolute right-0.5 flex flex-row-reverse items-center gap-0.5">
                <!-- Standard actions -->
                <ActionPopover
                  v-if="!readonly"
                  anchor="right"
                  v-slot="{ open }"
                  :thing="record"
                  :actions="recordActions"
                >
                  <div
                    class="p-0.5 text-gray-400 hover:text-gray-700"
                    :class="[
                      open
                        ? ''
                        : 'opacity-0 transition-opacity focus:opacity-100 group-focus-within/record:opacity-100 group-hover/record:opacity-100',
                    ]"
                  >
                    <EllipsisVerticalIcon class="h-4 w-4" />
                  </div>
                </ActionPopover>
                <!-- Insert record -->
                <button
                  v-if="!readonly"
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
            v-for="(field, x) in allFields"
            :key="record.id + '.' + field?.id"
            class="h-full overflow-hidden"
            :class="[verticalBorders && x > 0 ? 'border-l border-orange-900 border-opacity-[12%]' : '']"
            :style="{
              minHeight: minRowHeight + 'px',
              width: columnWidths[x] + 'px',
              height: rowHeights[y] + rowPadding * 2 + 'px',
            }"
          >
            <!-- TODO @Cleanup: not sure why the Boolean(properties.wrapColumns) is needed, but wrapColumns is an object otherwise?  -->
            <ValueInterface
              :ref="(el: any) => grid.registerColumnRef(record.id, field.key as string, el)"
              :model-value="record.value?.[module.getTypedKey(field) as string]"
              @update:model-value="(val) => writeRecordField(record.id, module.getTypedKey(field) as string, val)"
              :type="module.effectiveTypeOf(field)"
              :readonly="readonly"
              :active="editing || focused"
              :wrap="Boolean(properties.wrapColumns)"
              debounced
              :supports-drop="!readonly"
              @drop-files="(p, v) => onDropFiles(record.id, field.key as string, p, v)"
              @navigate-left="grid.navigateLeft(record.id, field.key as string)"
              @navigate-right="grid.navigateRight(record.id, field.key as string)"
              @navigate-up="grid.navigateUp(record.id, field.key as string)"
              @navigate-down="grid.navigateDown(record.id, field.key as string)"
              @delete-self="deleteRecordField(record.id, module.getTypedKey(field) as string)"
              class="scroll-hidden h-full w-full overflow-auto border border-transparent p-1 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
              :style="{ 'max-height': maxRowHeight + rowPadding * 2 + 'px' }"
            />
          </div>
          <!-- Extra properties column (empty) -->
          <div v-if="showPropertiesColumn">
            <div
              class="h-full overflow-hidden"
              :class="[
                verticalBorders && columnWidths.length > 1 ? 'border-l border-orange-900 border-opacity-[12%]' : '',
              ]"
              :style="{
                minHeight: minRowHeight + 'px',
                width: columnWidths[columnWidths.length - 1] + 'px',
                height: rowHeights[y] + rowPadding * 2 + 'px',
              }"
            >
              <div class="h-full w-full overflow-hidden border border-transparent p-1"></div>
            </div>
          </div>
        </div>
        <!-- Bottom actions -->
        <!-- Failed to load -->
        <button
          v-if="!loading && recordsError != null"
          class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900 border-opacity-[12%] px-1 py-1 text-red-600 outline-none transition duration-75 hover:bg-orange-100 hover:text-red-600 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
          :style="{ minHeight: minRowHeight + 'px' }"
          @click.stop="refetch()"
        >
          <XCircleIconSolid class="h-4 w-4" /> <span class="whitespace-nowrap font-bold">Failed to load:</span>
          <span class="max-w-full truncate">{{ recordsError.message }}</span>
        </button>
        <!-- Load more/loading -->
        <button
          v-if="pageInfo?.hasNextPage"
          ref="loadMoreRef"
          class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900 border-opacity-[12%] px-1 py-1 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
          :style="{ height: minRowHeight + 'px' }"
          @click.stop="loadMore()"
          @keydown.enter.prevent="loadMore(), $nextTick(() => focusLastRecord())"
          @keydown.up.exact.prevent="focusLastRecord"
          @keydown.down.exact.prevent="emit('navigateDown')"
          :disabled="loading"
        >
          <template v-if="loading">
            <BusySpinnerIcon class="mr-1 h-4 w-4" :class="loading ? 'animate-spin' : ''" />
            Loading
          </template>
          <template v-else>
            <ArrowDownIcon class="h-4 w-4" />
            Load {{ PAGE_SIZE }} more (of {{ humanizeNumber(totalCount ?? 0) }})
          </template>
        </button>
        <!-- Insert button (or 'nothing here') -->
        <button
          v-if="!readonly || recordsInView.length == 0"
          ref="addRecordRef"
          class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900 border-opacity-[12%] px-1 py-1 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
          :style="{ height: minRowHeight + 'px' }"
          @click.stop="readonly || insertRecordAtEnd()"
          @keydown.enter.prevent="readonly || insertRecordAtEnd()"
          @keydown.up.exact.prevent="(loadMoreRef?.focus ?? focusLastRecord)()"
          @keydown.down.exact.prevent="emit('navigateDown')"
          :disabled="loading"
        >
          <template v-if="readonly"> Nothing here </template>
          <template v-else> <PlusIcon class="h-4 w-4" /> New </template>
        </button>
      </div>
    </div>
  </div>
</template>
