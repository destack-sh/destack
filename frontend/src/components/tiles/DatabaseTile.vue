<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import { getInputInterface } from "@/components/inputs";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { humanizeNumber } from "@/composables/useNow";
import { useElementSize } from "@/composables/useSize";
import {
  EditType,
  QueryEngine,
  type Conditional,
  type SearchRecordsQueryVariables,
  type SortOp,
  type Sort,
} from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import type { RecordAction } from "@/state/bench";
import { RECORD_SEARCH_QUERY } from "@/state/database";
import { useCurrentModule, type Statement, newNodeIdentity, type Field, type Record } from "@/state/module";
import { useOperations } from "@/state/operations";
import { useFields } from "@/state/statement";
import { useEditListener } from "@/state/sync";
import { emptyConnection, getUpdatedConnectionQuery } from "@/utils/connection";
import { toValueRef } from "@/utils/functools";
import { IS_DEBUG } from "@/utils/globals";
import { Square2StackIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { EllipsisHorizontalIcon, EllipsisVerticalIcon, PlusIcon, XCircleIcon } from "@heroicons/vue/24/solid";
import { useApolloClient, useQuery } from "@vue/apollo-composable";
import { onStartTyping, useDebounceFn, useKeyModifier } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, nextTick, toRef, type Ref, ref, watch, onMounted } from "vue";

type GRecord<K extends keyof any, V> = globalThis.Record<K, V>;

const props = defineProps<{
  statement: Statement;
  query?: Conditional;
  queryEngine?: QueryEngine;
  sort?: Sort[];
  after?: string | null;
  targetMinWidth: number;
  paddingLeft?: number;
  showRecordActionPopover?: boolean;
  stickyHeader?: boolean;
  selectable?: boolean;
  pageSize: number;
  readonly?: boolean;
  selectedRecordIds?: string[];
  wrap?: boolean;
}>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "openActions"): void;
  (e: "createField"): void;
  (e: "addSort", sort: { field: Field; order?: SortOp }): void;
  (e: "addFilter", filter: { field: Field }): void;
  (e: "updateSelectedRecordIds", ids: string[]): void;
}>();
const module = useCurrentModule();
const appearance = useAppearance();
const ops = useOperations();
const client = useApolloClient();

// data

const fields = useFields(toRef(props, "statement"));
const { allFields, selfFields, inheritedFields, moveFieldTo } = fields;
const fieldsTypedKeyByCk: Ref<GRecord<string, string>> = toValueRef(
  computed(() => Object.fromEntries(allFields.value.map((f) => [f.ck, module.getTypedKey(f) as string])))
);

const searchQueryVariables: Ref<SearchRecordsQueryVariables> = computed(
  () =>
    ({
      statementId: props.statement.id,
      after: props.after as string | null,
      query: props.query,
      sort: props.sort,
      limit: props.pageSize,
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
  enabled: computed(() => !module.loading.value) as any, // the vue composable typing is all fucked up
  fetchPolicy: "network-only", // don't cache
});
const pageInfo = computed(() => recordsFetchedResult.value?.searchRecords.pageInfo);
const totalCount = computed(() => recordsFetchedResult.value?.searchRecords.totalCount);
const recordsFetched = computed(() => recordsFetchedResult.value?.searchRecords.edges.map((e) => e.node) ?? []);
const loading = computed(
  () => (recordsFetchedResult.value == null || recordsLoading.value) && recordsError.value == null
);
const recordsInView = computed(() => recordsFetched.value.filter((n) => n.deletedAt == null));

// auto refetch when bumped and using OS query engine (1s is the OS indexing delay)
const refetchDebounced = useDebounceFn(refetch, 1000, { maxWait: 5000 });
useEditListener([EditType.BumpStatement], props.statement.id, () => {
  if (props.queryEngine == QueryEngine.Opensearch) {
    refetchDebounced();
  } else {
    refetch();
  }
});
// trigger refetch (debounced) once if just created to autoload if the database was duplicated
onMounted(() => {
  const delta = DateTime.now().diff(DateTime.fromISO(props.statement.createdAt ?? ""));
  if (delta.as("seconds") < 1) {
    refetchDebounced();
  }
});

function loadMore(pageSize?: number) {
  if (!pageInfo.value?.hasNextPage) return;
  fetchMore({
    variables: {
      after: recordsFetchedResult.value?.searchRecords.edges.slice(-1)[0]?.cursor,
      limit: pageSize ?? props.pageSize,
    },
  });
}

function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "referenceCk" | "value"> & Partial<Field>) {
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

function dropField(droppedId: string, position: "above" | "below" | "right" | "left", fieldId: string) {
  const dropped = selfFields.value.find((n) => n.id == droppedId);
  const field = selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  moveFieldTo(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
  nextTick(() => grid.focus("", dropped.key ?? ""));
}

function insertRecordAtEnd() {
  insertRecord({ belowRecordId: recordsInView.value?.[recordsInView.value.length - 1]?.id });
}

function insertRecord(options?: { belowRecordId?: string; value?: any }) {
  const identity = newNodeIdentity(module.id.value, "Record");
  ops.symbol.createRecord(
    null,
    identity.id,
    identity.ck,
    props.statement.id,
    props.statement.ck,
    props.statement.key as string,
    options?.value ?? ({} as any)
  );
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
  newValue[key] = null;
  ops.symbol.updateRecord(null, props.statement.id, recordId, oldValue, newValue);
}

function deleteRecord(recordId: string) {
  const recordIdx = recordsInView.value.findIndex((r) => r.id === recordId);
  if (recordIdx < 0) throw new Error("record not found: " + recordId);
  ops.symbol.softDeleteRecord(null, props.statement.id, recordId);
  grid.focus(recordIdx, columnsInOrder.value[0]);
}

// navigation

const selectedRecordIds: Ref<string[]> = ref(props.selectedRecordIds ?? []);
const hasAnySelectedRecords = computed(() => selectedRecordIds.value.length > 0);
const shiftKeyPressed = useKeyModifier("Shift");

function isRecordSelected(record: { id: string }) {
  return selectedRecordIds.value.length > 0 && selectedRecordIds.value.includes(record.id);
}

function setRecordSelected(record: { id: string }, selected: boolean, shift: boolean) {
  if (selected) {
    if (shift) {
      // add (select) all between last selected and this one
      const lastSelectedIdx = recordsInView.value.findIndex((r) => r.id == selectedRecordIds.value.slice(-1)[0]);
      const thisIdx = recordsInView.value.findIndex((r) => r.id == record.id);
      if (lastSelectedIdx >= 0 && thisIdx >= 0) {
        const [start, end] = lastSelectedIdx < thisIdx ? [lastSelectedIdx, thisIdx] : [thisIdx, lastSelectedIdx];
        for (let i = start; i <= end; i++) {
          if (!selectedRecordIds.value.includes(recordsInView.value[i].id)) {
            selectedRecordIds.value.push(recordsInView.value[i].id);
          }
        }
      }
    }
    selectedRecordIds.value.push(record.id);
  } else {
    const idx = selectedRecordIds.value.indexOf(record.id);
    if (idx >= 0) selectedRecordIds.value.splice(idx, 1);
  }

  emit("updateSelectedRecordIds", selectedRecordIds.value);
}
// sync selected record ids from props
watch(
  () => props.selectedRecordIds,
  (ids) => {
    selectedRecordIds.value = ids ?? [];
  },
  { immediate: true }
);

const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);

function focusLastRecord() {
  if (grid.refs.value.length > 0) {
    grid.focus(-1, columnsInOrder.value[0]);
  } else {
    emit("navigateUp");
  }
}

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

// display

const columnsInOrder: Ref<string[]> = computed(() => allFields.value?.map((n) => n.key ?? "") ?? []);
const grid = useNavigationGrid<string, InstanceType<typeof FieldInterface> | InstanceType<typeof ValueInterface>>(
  columnsInOrder,
  computed(() => {
    // one row for fields, then values
    return [{ id: "" }, ...recordsInView.value];
  }),
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => (props.readonly ? emit("navigateDown") : addRecordRef.value?.focus()),
  }
);

const rowPadding = 4;
const minRowHeight = 32; // incl. padding x2
const maxRowHeight = 220;
const defaultGrowFactor = 0.1;
const defaultMinWidth = 50;
const selectColumnWidth = 32;
const propertiesColumnWidth = 52;
const rowHeights: Ref<number[]> = ref([]);
const columnWidths: Ref<number[]> = ref([]);

// update column widths
watch(
  () => [props.targetMinWidth, allFields.value],
  () => {
    // update column widths
    const ifaces: ({ minWidth?: number; grow?: number } | undefined)[] = allFields.value.map((f) =>
      getInputInterface(module.effectiveTypeOf(f))
    );
    ifaces.push({ minWidth: propertiesColumnWidth, grow: 0.01 }); // 'fake' properties column

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
    let actualTotalWidth = widths.reduce((a, b) => a + b, 0);
    if (props.selectable) actualTotalWidth += selectColumnWidth;
    if (actualTotalWidth < props.targetMinWidth) {
      const toFill = Math.max(props.targetMinWidth - actualTotalWidth, 0);
      const growFactors = ifaces.map((i) => i?.grow ?? defaultGrowFactor);
      const growTotal = growFactors.reduce((a, b) => a + b, 0);
      const growWidths = growFactors.map((g) => (g / growTotal) * toFill);
      for (let i = 0; i < widths.length; i++) {
        widths[i] += growWidths[i];
      }
    }

    // update (if changed)
    if (widths.some((w, i) => w != columnWidths.value[i])) {
      columnWidths.value = widths;
    }
  },
  { immediate: true }
);

// auto-edit value field if starting to type (clear & focus)
onStartTyping((e) => {
  if (props.readonly) return;
  const cell = grid.findRef((r) => r.$el.parentNode.contains(e.target));
  if (cell != null && cell.rowId != "") {
    const field = allFields.value.find((f) => f.key == cell.column);
    if (field == null) return;
    deleteRecordField(cell.rowId, fieldsTypedKeyByCk.value[field.ck]);
    nextTick(() => (cell.ref as unknown as { edit?: () => void }).edit?.());
  }
});

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    if (position == "first") {
      grid.focus(0, columnsInOrder.value[0]);
    } else {
      focusLastRecord();
    }
  },
  blur: () => grid.blur(),
  loadMore,
  refetch,
  loading,
  recordsInView,
  pageInfo,
  totalCount,
  selectedRecordIds,
  createNewField,
  insertRecordAtEnd,
  insertRecord,
});
</script>
<template>
  <div class="relative flex flex-col">
    <!-- Header -->
    <div
      class="flex flex-row self-start align-top text-sm"
      :class="[stickyHeader ? 'sticky top-0 z-10  bg-white' : '']"
    >
      <!-- Select column (placeholder, maybe put something here later) -->
      <!-- Acts as select all / deselect -->
      <button
        v-if="selectable"
        class="select-none border-b border-r border-t border-amber-900/[12%] text-center hover:bg-amber-100"
        :class="[]"
        :style="{
          width: selectColumnWidth + 'px',
        }"
        @click="
          selectedRecordIds.length > 0
            ? (selectedRecordIds = [])
            : (selectedRecordIds = recordsInView.map((r) => r.id));
          emit('updateSelectedRecordIds', selectedRecordIds);
        "
      >
        <span v-if="selectedRecordIds.length > 0" class="py-1 text-amber-600">
          {{ humanizeNumber(selectedRecordIds.length ?? 0) }}
        </span>
      </button>
      <!-- Fields -->
      <FieldInterface
        :ref="(el: any) => grid.registerColumnRef('', field.key as string, el)"
        v-for="(field, x) in allFields"
        :key="field?.ck"
        class="h-full w-full truncate border-b border-t border-amber-900/[12%] px-1.5 py-1 text-gray-700 focus-within:border-amber-900/[15%] focus-within:bg-amber-100 hover:bg-amber-100"
        :class="[x > 0 ? 'border-l' : '', x == allFields.length - 1 ? 'border-r' : '']"
        :style="{
          width: columnWidths[x] + 'px',
          paddingLeft: paddingLeft != null && x == 0 ? paddingLeft + 'px' : undefined,
        }"
        is-view
        orientation="horizontal"
        can-filter
        :readonly="readonly"
        :model-value="field"
        @update:model-value="updateField(field.key, $event as Field)"
        @navigate-left="grid.navigateLeft('', field.key as string)"
        @navigate-right="grid.navigateRight('', field.key as string)"
        @navigate-up="grid.navigateUp('', field.key as string)"
        @navigate-down="grid.navigateDown('', field.key as string)"
        @delete-self="deleteField(field)"
        @duplicate-self="duplicateField(field.id)"
        @drop="(p, v) => dropField(v.id, p, field.id)"
        @enter="grid.navigateDown('', field.key as string)"
        @sort="(order) => emit('addSort', { field, order })"
        @filter="() => emit('addFilter', { field })"
      />
      <!-- Properties column (add + settings) -->
      <div
        class="flex flex-row items-center overflow-x-hidden whitespace-nowrap border-b border-t border-amber-900/[12%]"
        :style="{
          width: columnWidths[columnWidths.length - 1] + 'px',
        }"
      >
        <!-- Add column -->
        <button
          v-if="!readonly"
          tabindex="-1"
          class="h-full rounded-sm p-1.5 text-gray-400 transition duration-150 hover:bg-amber-100 hover:text-gray-700"
          @click="emit('createField')"
        >
          <PlusIcon class="h-4 w-4" />
        </button>
        <!-- Properties -->
        <button
          tabindex="-1"
          class="h-full flex-1 rounded-sm p-1.5 text-gray-400 transition duration-150 hover:bg-amber-100 hover:text-gray-700"
          @click="emit('openActions')"
        >
          <EllipsisHorizontalIcon class="h-4 w-4" />
        </button>
      </div>
    </div>
    <!-- Records -->
    <div
      v-for="(record, y) in recordsInView"
      :key="record.id"
      class="group/record relative flex flex-row self-start border-b border-orange-900/[12%] align-top"
      :class="[appearance.textSmall ? 'text-sm' : '', isRecordSelected(record) ? 'bg-orange-100' : '']"
    >
      <!-- Select row -->
      <div
        v-if="selectable"
        class="select-none border-r border-orange-900/[12%] text-center align-middle hover:cursor-pointer"
        :style="{ width: selectColumnWidth + 'px' }"
        @click="setRecordSelected(record, !isRecordSelected(record), shiftKeyPressed ?? false)"
      >
        <input
          type="checkbox"
          ref="checkboxRef"
          class="mt-1.5 h-4 w-4 cursor-pointer rounded border border-gray-300 text-orange-600 ring-0 transition-opacity duration-75 focus:ring-0"
          :class="hasAnySelectedRecords ? 'opacity-100' : 'opacity-0 group-hover/record:opacity-100'"
          :checked="isRecordSelected(record)"
          :disabled="props.readonly"
        />
      </div>
      <!-- Action popover -->
      <div v-if="showRecordActionPopover" class="absolute -left-6 top-1">
        <ActionPopover v-if="!readonly" anchor="right" v-slot="{ open }" :thing="record" :actions="recordActions">
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
      </div>
      <!-- Values -->
      <ValueInterface
        :ref="(el: any) => grid.registerColumnRef(record.id, field.key as string, el)"
        v-for="(field, x) in allFields"
        :key="field?.ck"
        :model-value="record.value?.[fieldsTypedKeyByCk[field.ck]]"
        @update:model-value="writeRecordField(record.id, fieldsTypedKeyByCk[field.ck], $event)"
        :type="module.effectiveTypeOf(field)"
        :readonly="readonly"
        :inlined="inheritedFields.find((n) => n.key == field.key) != null"
        debounced
        :supports-drop="false"
        :wrap="wrap"
        @navigate-left="() => grid.navigateLeft(record.id, field.key as string)"
        @navigate-right="() => grid.navigateRight(record.id, field.key as string)"
        @navigate-up="() => grid.navigateUp(record.id, field.key as string)"
        @navigate-down="() => grid.navigateDown(record.id, field.key as string)"
        @delete-self="() => deleteRecordField(record.id, fieldsTypedKeyByCk[field.ck])"
        class="scroll-hidden h-full overflow-hidden border border-transparent p-1 focus-within:border-solid focus-within:border-orange-900/[15%] focus-within:bg-orange-100 hover:bg-orange-100"
        :class="[
          x > 0 ? 'border-l-orange-900/[12%]' : '',
          x == allFields.length - 1 ? 'border-r-orange-900/[12%]' : '',
        ]"
        :style="{
          minHeight: minRowHeight + 'px',
          maxHeight: maxRowHeight + rowPadding * 2 + 'px',
          width: columnWidths[x] + 'px',
          paddingLeft: paddingLeft != null && x == 0 ? paddingLeft + 'px' : undefined,
        }"
      />
      <!-- Extra empty 'value' for properties column (also useful as placeholder if database has no fields) -->
      <div
        :style="{
          minHeight: minRowHeight + 'px',
          width: columnWidths[columnWidths.length - 1] + 'px',
          height: rowHeights[y] + rowPadding * 2 + 'px',
        }"
      />
    </div>
    <!-- Insert button (or 'nothing here') -->
    <button
      v-if="!loading && (!readonly || recordsInView.length == 0)"
      ref="addRecordRef"
      class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900/[12%] px-1 py-1 text-sm text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      :style="{ height: minRowHeight + 'px', paddingLeft: paddingLeft != null ? paddingLeft + 'px' : undefined }"
      @click.stop="readonly || insertRecordAtEnd()"
      @keydown.enter.prevent="readonly || insertRecordAtEnd()"
      @keydown.up.exact.prevent="focusLastRecord()"
      @keydown.down.exact.prevent="emit('navigateDown')"
      :disabled="loading"
    >
      <template v-if="readonly"> Nothing here </template>
      <template v-else> <PlusIcon class="h-4 w-4" /> Record </template>
    </button>
    <!-- Failed to load -->
    <div
      v-if="!loading && recordsError != null"
      class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900/[12%] px-1 py-1 text-red-600 outline-none transition duration-75 hover:bg-orange-100 hover:text-red-600 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      :style="{ minHeight: minRowHeight + 'px' }"
      @click.stop="refetch()"
    >
      <XCircleIcon class="h-4 w-4" /> <span class="whitespace-nowrap font-bold">Failed to load:</span>
      <span class="max-w-full truncate">{{ IS_DEBUG ? recordsError.message : "Internal error" }}</span>
    </div>
  </div>
</template>
