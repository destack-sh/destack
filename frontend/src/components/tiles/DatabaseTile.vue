<script lang="ts" setup>
import { getInputInterface } from "@/components/inputs";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { useElementSize } from "@/composables/useSize";
import type { SearchRecordsQueryVariables, SortOp } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { RECORD_SEARCH_QUERY } from "@/state/database";
import { useCurrentModule, type Statement, newNodeIdentity, type Field, type Record } from "@/state/module";
import { useOperations } from "@/state/operations";
import { useFields } from "@/state/statement";
import { emptyConnection, getUpdatedConnectionQuery } from "@/utils/connection";
import { toValueRef } from "@/utils/functools";
import { EllipsisHorizontalIcon, PlusIcon } from "@heroicons/vue/24/solid";
import { useApolloClient, useQuery } from "@vue/apollo-composable";
import { computed, nextTick, toRef, type Ref, ref, watch } from "vue";

const PAGE_SIZE = 32;
type GRecord<K extends keyof any, V> = globalThis.Record<K, V>;

const props = defineProps<{ statement: Statement; targetMinWidth: number; readonly?: boolean }>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "openActions"): void;
  (e: "createField"): void;
  (e: "addSort", sort: { field: Field; order?: SortOp }): void;
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

// :QueryFieldPolicies
const searchQueryVariables: Ref<SearchRecordsQueryVariables> = computed(
  () =>
    ({
      statementId: props.statement.id,
      after: null as string | null,
      query: null,
      sort: null,
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

function focusLastRecord() {
  if (grid.refs.value.length > 0) {
    grid.focus(-1, columnsInOrder.value[0]);
  } else {
    emit("navigateUp");
  }
}

// display

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
    gridNavigateDown: () => emit("navigateDown"),
  }
);

const rowPadding = 4;
const minRowHeight = 32; // incl. padding x2
const maxRowHeight = 220;
const defaultGrowFactor = 0.1;
const defaultMinWidth = 50;
const growColumns = true;
const propertiesColumnWidth = 52;
const rowHeights: Ref<number[]> = ref([]);
const columnWidths: Ref<number[]> = ref([]);

// update column widths
watch(
  () => [props.targetMinWidth, allFields.value],
  () => {
    // update column widths
    const ifaces: ({ minWidth?: number; grow?: number } | undefined)[] = allFields.value.map((f) =>
      getInputInterface(f)
    );
    ifaces.push({ minWidth: propertiesColumnWidth, grow: 0.05 }); // 'fake' properties column

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
    if (minTotalWidth < props.targetMinWidth && growColumns) {
      const toFill = Math.max(props.targetMinWidth - minTotalWidth, 0);
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
</script>
<template>
  <div class="flex flex-col">
    <!-- Fields -->
    <div class="flex flex-row text-sm">
      <FieldInterface
        :ref="(el: any) => grid.registerColumnRef('', field.key as string, el)"
        v-for="(field, x) in allFields"
        :key="field?.ck"
        class="h-full w-full border-b border-amber-900/[12%] px-1 py-1 text-gray-700 focus-within:border-amber-900 focus-within:bg-amber-100 hover:bg-amber-100"
        :class="[x > 0 ? 'border-l' : '', x == allFields.length - 1 ? 'border-r' : '']"
        :style="{
          width: columnWidths[x] + 'px',
        }"
        is-view
        hide-outline
        orientation="horizontal"
        :model-value="field"
        @update:model-value="updateField(field.key, $event as Field)"
        @navigate-left="grid.navigateLeft('', field.key as string)"
        @navigate-right="grid.navigateRight('', field.key as string)"
        @navigate-up="grid.navigateUp('', field.key as string)"
        @navigate-down="grid.navigateDown('', field.key as string)"
        @delete-self="deleteField(field)"
        @duplicate-self="duplicateField(field.id)"
        @drop="(p, v) => dropField(v.id, p, field.id)"
        @sort="(order) => emit('addSort', { field, order })"
        @enter="grid.navigateDown('', field.key as string)"
      />
      <!-- Properties column (add + settings) -->
      <div
        class="flex flex-row items-center overflow-x-hidden whitespace-nowrap border-b border-r border-amber-900/[12%]"
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
      :class="[appearance.textSmall ? 'text-sm' : '']"
    >
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
        @navigate-left="() => grid.navigateLeft(record.id, field.key as string)"
        @navigate-right="() => grid.navigateRight(record.id, field.key as string)"
        @navigate-up="() => grid.navigateUp(record.id, field.key as string)"
        @navigate-down="() => grid.navigateDown(record.id, field.key as string)"
        @delete-self="() => deleteRecordField(record.id, fieldsTypedKeyByCk[field.ck])"
        class="scroll-hidden h-full overflow-hidden border border-transparent p-1 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
        :class="[
          x > 0 ? 'border-l-orange-900/[12%]' : '',
          x == allFields.length - 1 ? 'border-r-orange-900/[12%]' : '',
        ]"
        :style="{
          minHeight: minRowHeight + 'px',
          maxHeight: maxRowHeight + rowPadding * 2 + 'px',
          width: columnWidths[x] + 'px',
        }"
      />
      <div
        class="overflow-hidden"
        :class="[columnWidths.length > 1 ? 'border-r border-orange-900/[12%]' : '']"
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
      :style="{ height: minRowHeight + 'px' }"
      @click.stop="readonly || insertRecordAtEnd()"
      @keydown.enter.prevent="readonly || insertRecordAtEnd()"
      @keydown.up.exact.prevent="focusLastRecord()"
      @keydown.down.exact.prevent="emit('navigateDown')"
      :disabled="loading"
    >
      <template v-if="readonly"> Nothing here </template>
      <template v-else> <PlusIcon class="h-4 w-4" /> Record </template>
    </button>
  </div>
</template>
