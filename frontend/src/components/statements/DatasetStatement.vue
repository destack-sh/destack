<script lang="ts" setup>
import ActionPopover from "@/components/basic/ActionPopover.vue";
import DragHandleIcon from "@/components/basic/DragHandleIcon.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { getInterface } from "@/components/inputs";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import StatementActions from "@/components/statements/StatementActions.vue";
import TypedDeclarationCell from "@/components/statements/TypedStatementDeclaration.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { humanizeNumber } from "@/composables/useNow";
import { useActiveScroll } from "@/composables/useScroll";
import { graphql, useFragment } from "@/gql";
import {
  QueryOp,
  SortOrder,
  type SearchDatasetQueryVariables,
  type DatasetSort,
  type DatasetQuery,
  ModuleMutationType,
  TypeHint,
} from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import {
  useEditorContext,
  useElementEditorSettings,
  type RecordAction,
  type StatementAction,
  type StatementHeader,
} from "@/state/bench";
import { useMagicActions } from "@/state/file";
import { useCurrentModule } from "@/state/module";
import { useOperations } from "@/state/operations";
import { newRecordId } from "@/state/operations/statement";
import { useStatementContext, type Field } from "@/state/statement";
import { generateKeyBetween, generateNKeysBetween } from "@/utils/fractional";
import { IS_DEBUG, IS_LOCALHOST } from "@/utils/globals";
import {
  ArrowDownIcon,
  ArrowPathIcon,
  CubeTransparentIcon,
  EllipsisHorizontalIcon,
  MagnifyingGlassIcon,
  PlusIcon,
  Square2StackIcon,
  SquaresPlusIcon,
  TrashIcon,
  ChevronDoubleDownIcon,
  ChevronDoubleUpIcon,
  XMarkIcon,
  Bars3Icon,
  XCircleIcon as XCircleIconOutline,
  TagIcon,
} from "@heroicons/vue/24/outline";
import { useApolloClient, useQuery } from "@vue/apollo-composable";
import { onStartTyping, useDebounceFn, useElementBounding, useMouseInElement, useScroll } from "@vueuse/core";
import { computed, nextTick, ref, watch, type Ref, onMounted } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { TypeStorageFormat, getStorageFormat, SubfieldType, canSort, getMainSubfield } from "@/state/type";
import { toValueRef } from "@/utils/functools";
import { useMutationListener } from "@/state/sync";
import { DateTime } from "luxon";
import { TypeTag } from "@/gql/graphql";
import { FieldType } from "@/state/fragments";
import { XCircleIcon as XCircleIconSolid } from "@heroicons/vue/24/solid";
import StatementTags from "@/components/statements/StatementTags.vue";

const props = defineProps<{ folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void }>();

const context = useStatementContext();
const module = useCurrentModule();
const PAGE_SIZE = context.standalone.value ? 50 : 15;
const editor = useEditorContext();
const addingDescription = ref(false);
const showDescription = computed(() => description.value.length > 0 || addingDescription.value);

const appearance = useAppearance();
const client = useApolloClient();
const declarationRef: Ref<InstanceType<typeof TypedDeclarationCell> | null> = ref(null);
const tagsRef: Ref<InstanceType<typeof StatementTags> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);
const gridRef: Ref<HTMLDivElement | null> = ref(null);
const innerGridRef: Ref<HTMLDivElement | null> = ref(null);
const loadMoreRef: Ref<HTMLButtonElement | null> = ref(null);
const addRecordRef: Ref<HTMLButtonElement | null> = ref(null);
const createFieldRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const searchRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

type DatasetStatementProperties = {
  inlineQuery?: string;
  wrapColumns: boolean;
  // local 'view' (because we don't have proper module dataset view yet, this is the only view)
  sorts?: DatasetSort[];
  query?: DatasetQuery;
};

const properties = useElementEditorSettings<DatasetStatementProperties>(context.statement, {
  inlineQuery: undefined,
  wrapColumns: false,
});

// reset inline query to undefined if it's empty on load
onMounted(() => {
  if ((properties.inlineQuery ?? "").trim().length == 0) {
    properties.inlineQuery = undefined;
  }
});

// find every string-stored field for search
const stringFields = computed(() =>
  context.allFields.value.filter((f) => getStorageFormat(f.tag, f.hint, f.flags) == TypeStorageFormat.STRING)
);
const nameFields = computed(() => stringFields.value.filter((f) => f.hint == TypeHint.Name));
const enumFields = computed(() =>
  context.allFields.value.filter(
    (f) =>
      f.tag == TypeTag.Enum ||
      (f.tag == TypeTag.TypeReference && module.statementOf(f.reference?.id)?.rootTypeTag == TypeTag.Enum)
  )
);
// TODO @UX: apply inline search to local records immediately/optmistically
// update search query on inline query change
const inlineQuery: Ref<DatasetQuery | undefined> = ref(undefined);
function getInlineQuery() {
  if ((properties.inlineQuery ?? "").trim().length == 0) return undefined;
  const subqueries = [
    ...stringFields.value.map(
      (f) =>
        ({
          op: QueryOp.Matches,
          key: "value." + module.getTypedKey(f),
          value: properties.inlineQuery,
        } as DatasetQuery)
    ),
    ...nameFields.value.map(
      (f) =>
        ({
          op: QueryOp.StartsWith,
          key: "value." + module.getTypedKey(f) + "." + SubfieldType.starts_with,
          value: properties.inlineQuery,
        } as DatasetQuery)
    ),
  ];
  // filter for enum fields members that match the query
  for (const enumField of enumFields.value) {
    const matchingMembers = module
      .statementOf(enumField.reference?.id)
      ?.fields.map((m) => useFragment(FieldType, m))
      .filter((m) => m.name?.toLowerCase().startsWith(properties.inlineQuery?.toLowerCase() ?? ""));
    if (matchingMembers == null || matchingMembers.length == 0) continue;
    subqueries.push({
      key: "value." + module.getTypedKey(enumField),
      op: QueryOp.Equals,
      value: matchingMembers.map((m) => m.key),
    } as DatasetQuery);
  }

  if (subqueries.length == 0) return undefined; // TODO @UX: indicate inline search is not possible if no plausible subqueries
  return { op: QueryOp.Or, queries: subqueries } as DatasetQuery;
}
function updateInlineQuery() {
  inlineQuery.value = getInlineQuery();
}
const updateInlineQueryDebounced = useDebounceFn(updateInlineQuery, 100);
watch(
  () => [properties.inlineQuery, stringFields.value, nameFields.value, enumFields.value],
  updateInlineQueryDebounced,
  { immediate: true }
);

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
function clearSort() {
  properties.sorts = undefined;
}
function removeSort(sort: { key: string }) {
  properties.sorts = properties.sorts?.filter((s) => !s.key.includes(sort.key));
}
const sort: Ref<DatasetSort[] | null> = computed(() => {
  if (properties.sorts == null || properties.sorts.length == 0) return null;
  return properties.sorts;
});

const SEARCH_QUERY = graphql(/* GraphQL */ `
  query searchRecord(
    $statementId: GlobalID!
    $query: SearchQuery
    $sort: [SearchSort!]
    $after: String
    $limit: Int
    $count: Boolean
  ) {
    searchDataset(statementId: $statementId, query: $query, sort: $sort, after: $after, limit: $limit, count: $count) {
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
          value
        }
      }
    }
  }
`);
const searchQueryVariables: Ref<SearchDatasetQueryVariables> = computed(
  () =>
    ({
      statementId: context.statement.value.id,
      after: null as string | null,
      query: inlineQuery.value,
      sort: sort.value,
      limit: PAGE_SIZE + 1, // overfetch by one to get order key for next page
      count: true,
    } as SearchDatasetQueryVariables)
);
const {
  loading: recordsLoading,
  result: recordsFetchedResult,
  error: recordsError,
  refetch,
  fetchMore,
} = useQuery(SEARCH_QUERY, toValueRef(searchQueryVariables), {
  fetchPolicy: "network-only",
  enabled: computed(() => !module.loading.value) as any, // the vue composable typing is all fucked up
});
const pageInfo = computed(() => recordsFetchedResult.value?.searchDataset.pageInfo);
const recordsFetched = computed(
  () =>
    recordsFetchedResult.value?.searchDataset.edges
      .slice(0, pageInfo.value?.hasNextPage ? -1 : undefined)
      .map((e) => e.node) ?? []
);
const loading = computed(
  () => (recordsFetchedResult.value == null || recordsLoading.value) && recordsError.value == null
);
const totalCount = computed(() => recordsFetchedResult?.value?.searchDataset.totalCount ?? 0);

const recordsInView = computed(() => recordsFetched.value.filter((n) => n.deletedAt == null));
const lastRecordInView = computed(() => recordsInView.value?.[recordsInView.value.length - 1]);
const overfetchedRecord = computed(() =>
  pageInfo.value?.hasNextPage ? recordsFetchedResult.value?.searchDataset.edges.slice(-1)[0]?.node : null
);

// auto refetch when bumped (1s is the OS indexing delay)
const refetchDebounced = useDebounceFn(refetch, 1000, { maxWait: 10000 });
useMutationListener([ModuleMutationType.BumpStatement], context.statement.value?.id, () => {
  refetchDebounced();
});
// trigger refetch (debounced) once if just created to autoload if the dataset was duplicated
onMounted(() => {
  const delta = DateTime.now().diff(DateTime.fromISO(context.statement.value?.createdAt ?? ""));
  if (delta.as("seconds") < 1) {
    refetchDebounced();
  }
});

function loadMore() {
  if (!pageInfo.value?.hasNextPage) return;
  fetchMore({
    variables: {
      after: recordsFetchedResult.value?.searchDataset.edges.slice(-1)[0]?.cursor,
      limit: PAGE_SIZE + 1, // technically no need to overfetch but limit is a key arg for the relay style pagination merge policy
    },
  });
}

function toggleInlineSearch() {
  if (properties.inlineQuery == null) {
    properties.inlineQuery = "";
    nextTick(() => searchRef.value?.focus());
  } else {
    properties.inlineQuery = undefined;
    nextTick(() => declarationRef.value?.focus());
  }
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
const defaultGrowFactor = 0.1;
const defaultMinWidth = 50;
const growColumns = true;
const showPropertiesColumn = true; // used to be only in write mode, but useful if no columns and for settings shortcut
const propertiesColumnWidth = 40;
const columnWidths: Ref<number[]> = ref([]);
const rowHeights: Ref<number[]> = ref([]);
const gridOffsetX: Ref<number> = computed(() => {
  if (editor.size.value.width > editor.editor.value.contentWidthWithMargin) {
    return (editor.size.value.width - editor.editor.value.contentWidth) / 2;
  } else {
    return editor.editor.value.contentMarginX;
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
    editor.editor.value.contentWidth,
    editor.editor.value.contentMarginX,
    context.xOffset,
    editor.size.value,
    context.allFields.value,
    Object.values(grid.refsByColumn.value).map((r) => [r.previewSize.width.value, r.previewSize.height.value]),
  ],
  () => {
    // update column widths
    const targetMinTotalWidth =
      Math.min(editor.size.value.width - editor.editor.value.contentMarginX * 2, editor.editor.value.contentWidth) -
      context.xOffset.value -
      8; // not sure why -8, probably some mx-1? borders?
    const ifaces: ({ minWidth?: number; grow?: number } | undefined)[] = context.allFields.value.map((f) =>
      getInterface(f)
    );
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
  { immediate: context.allFields.value.length == 0 } // force update on first render if no columns to trigger initial sizing
);

const gridBounding = useElementBounding(gridRef);
const innerGridBounding = useElementBounding(innerGridRef);
const gridScroll = useScroll(gridRef);
const gridScrollOffsetX = computed(() => gridScroll.x.value);
const hasFloatingHeader = computed(() => {
  // sticky the header to the top if the grid is partially visible (top of editor viewport)
  const editorTop = editor.pos.value.top + appearance.editorHeaderHeight;
  return gridBounding.top.value < editorTop && gridBounding.bottom.value - minRowHeight > editorTop;
});
const gridOverhangLeft = computed(() => {
  // how much the grid overhangs the left of the editor
  return Math.max(editor.pos.value.left - innerGridBounding.left.value, 0);
});
const gridOverhangRight = computed(() => {
  // how much the grid overhangs the right of the editor
  return Math.max(innerGridBounding.right.value - (editor.pos.value.left + editor.size.value.width), 0);
});

// navigation
useActiveScroll(gridRef);

// auto-edit value field if starting to type (clear & focus)
onStartTyping((e) => {
  if (context.readonly.value) return;
  const cell = grid.findRef((r) => r.$el.parentNode.contains(e.target));
  if (cell != null && cell.rowId != "") {
    const field = context.allFields.value.find((f) => f.key == cell.column);
    if (field == null) return;
    deleteRecordField(cell.rowId, module.getTypedKey(field) as string);
    nextTick(() => cell.ref.edit?.());
  }
});

function focusDescriptionFromTop() {
  if (props.folded) {
    context.navigateDown();
  } else if (description.value?.length > 0 || addingDescription.value) {
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
    focusDescriptionFromBottom();
  }
}

// state

const ops = useOperations();

function createNewField(template: Pick<Field, "tag" | "hint" | "flags" | "reference" | "metadata">) {
  grid.beginBatchChange();
  const field = context.createNewField(template);
  nextTick(() => {
    grid.flush();
    nextTick(() => grid.focus("", field.key));
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
  // note: this was changed, not sure if it's still needed
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

function moveField(field: Field, position: "before" | "after", other: Field) {
  const otherIndex = context.selfFields.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(context.selfFields.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    context.moveField(field, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, context.selfFields.value[otherIndex + 1]?.orderKey ?? null);
    context.moveField(field, orderKey);
  }
}

function dropField(droppedId: string, position: "above" | "below" | "right" | "left", fieldId: string) {
  const dropped = context.selfFields.value.find((n) => n.id == droppedId);
  const field = context.selfFields.value.find((n) => n.id == fieldId);
  if (dropped == null || field == null || dropped.id == field.id) return; // ignore invalid / cross statement drops
  moveField(dropped, ["above", "left"].includes(position) ? "before" : "after", field);
  nextTick(() => grid.focus("", dropped.key ?? ""));
}

function getNewOrderKey(belowRecordId?: string): string | null {
  const recordIdx = recordsInView.value.findIndex((r) => r.id === belowRecordId);
  const recordBelow = recordsInView.value[recordIdx + 1] ?? overfetchedRecord.value;
  const beforeOk = recordsInView.value[recordIdx]?.orderKey ?? null;
  const afterOk = recordBelow?.orderKey ?? null;
  return generateKeyBetween(beforeOk, afterOk);
}

function insertRecordAtEnd() {
  insertRecord({ belowRecordId: lastRecordInView.value?.id });
}

function insertRecord(options?: { belowRecordId?: string; value?: any }) {
  const orderKey = getNewOrderKey(options?.belowRecordId);
  const recordId = newRecordId();
  ops.symbol.createRecord(null, recordId, context.statement.value.id, orderKey, options?.value ?? ({} as any));
  // add record to search results optimistically (regardless of filter)
  const recordRef = client.client.cache.identify({ __typename: "Record", id: recordId });
  const optimisticRecord = {
    __typename: "Record",
    id: recordId,
    revision: -1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    deletedAt: null,
    orderKey,
    value: options?.value ?? ({} as any),
  };
  client.client.cache.updateQuery(
    {
      query: SEARCH_QUERY,
      variables: searchQueryVariables.value,
    },
    (
      data = {
        searchDataset: {
          __typename: "RecordConnection" as any,
          totalCount: 0,
          edges: [],
          pageInfo: { hasNextPage: false, hasPreviousPage: false },
        },
      }
    ) => ({
      searchDataset: {
        ...data?.searchDataset,
        pageInfo: data?.searchDataset.pageInfo ?? {
          startCursor: null,
          endCursor: null,
          hasNextPage: false,
          hasPreviousPage: false,
        },
        totalCount: (data?.searchDataset.totalCount ?? 0) + 1,
        edges: [
          ...(data?.searchDataset.edges ?? []),
          {
            __typename: "RecordEdge" as any,
            cursor: data?.searchDataset.pageInfo.endCursor ?? "0",
            node: { __ref: recordRef, ...optimisticRecord } as any,
          },
        ],
      },
    })
  );
  // focus new record (for some reason the good ol' nextTick alone doesn't work here)
  grid.onColumnAvailable(recordId, columnsInOrder.value[0], (ref) => nextTick(ref.focus));
}

function writeRecordField(recordId: string, key: string, value: any) {
  const record = recordsInView.value.find((r) => r.id === recordId);
  if (record == null) throw new Error("record not found: " + recordId);
  const oldValue = record?.value;
  const newValue = { ...oldValue, [key]: value };
  ops.symbol.updateRecord(null, context.statement.value.id, recordId, oldValue, newValue);
}

function deleteRecordField(recordId: string, key: string) {
  const record = recordsInView.value.find((r) => r.id === recordId);
  if (record == null) throw new Error("record not found: " + recordId);
  const oldValue = record?.value;
  const newValue = { ...oldValue };
  // TODO @Robustness: figure out better way to clear field in opensearch backend (maybe update by query?)
  newValue[key] = [];
  ops.symbol.updateRecord(null, context.statement.value.id, recordId, oldValue, newValue);
}

function deleteRecord(recordId: string) {
  const recordIdx = recordsInView.value.findIndex((r) => r.id === recordId);
  if (recordIdx < 0) throw new Error("record not found: " + recordId);
  ops.symbol.softDeleteRecord(null, context.statement.value.id, recordId);
  grid.focus(recordIdx, columnsInOrder.value[0]);
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
    orderKeys = generateNKeysBetween(above?.orderKey ?? null, record.orderKey ?? null, files.length);
  } else {
    orderKeys = generateNKeysBetween(record.orderKey ?? null, below?.orderKey ?? null, files.length);
  }
  await magic.insertFilesAsRecords(column, orderKeys, files);
}
const position = useMouseInElement(gridRef);

// actions

function unfoldIfFolded() {
  if (props.folded) emit("toggleFold");
}

const extraActions = computed(() => {
  const actions: StatementAction[] = [];
  actions.push({
    label: "Search",
    icon: MagnifyingGlassIcon,
    action: () => {
      unfoldIfFolded();
      properties.inlineQuery = "";
      nextTick(() => searchRef.value?.focus());
    },
    hideInline: true,
  });
  if (IS_LOCALHOST || IS_DEBUG) {
    actions.push({
      label: "Reload view",
      icon: ArrowPathIcon,
      active: loading.value,
      action: () => refetch(),
    });
  }
  actions.push({
    label: "Add tag",
    icon: TagIcon,
    action: () => {
      unfoldIfFolded();
      tagsRef.value?.open();
    },
  });
  actions.push({
    label: "Add description",
    icon: Bars3Icon,
    disabled: showDescription.value,
    action: () => {
      unfoldIfFolded();
      addingDescription.value = true;
      nextTick(() => descriptionRef.value?.focus());
    },
  });
  actions.push({
    label: "Add record",
    icon: PlusIcon,
    action: () => {
      unfoldIfFolded();
      insertRecordAtEnd();
    },
  });
  actions.push({
    label: "Add field",
    icon: SquaresPlusIcon,
    action: () => {
      unfoldIfFolded();
      createFieldRef.value?.show();
    },
    hideInline: true,
  });
  actions.push({
    label: "Include type",
    icon: CubeTransparentIcon,
    action: () => {
      unfoldIfFolded();
      createUnionField();
    },
    hideInline: true,
  });
  actions.push({
    label: properties.wrapColumns ? "Unwrap columns" : "Wrap columns",
    icon: properties.wrapColumns ? ChevronDoubleUpIcon : ChevronDoubleDownIcon,
    action: () => (properties.wrapColumns = !properties.wrapColumns),
    hideInline: true,
  });
  return actions;
});
context.setCustomActions(extraActions);

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
    if (position == "first" || props.folded) {
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
  loading,
});
</script>
<template>
  <!-- Declaration -->
  <div class="flex max-w-full flex-row justify-between gap-2">
    <div class="flex max-w-full flex-row">
      <TypedDeclarationCell
        ref="declarationRef"
        @navigate-down="focusDescriptionFromTop"
        @navigate-up="context.navigateUp"
        @add-base="createUnionField"
      />
      <!-- Views (soon) -->
      <!-- Count -->
      <span class="ml-1 text-gray-400">{{ humanizeNumber(totalCount) }}</span>
      <StatementTags ref="tagsRef" class="ml-1.5" />
    </div>
    <!-- Inline actions -->
    <!-- always show when focused or inline query is active (not perfect from a UX standpoint...) -->
    <div
      class="flex flex-shrink-0 flex-row gap-1 transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value || properties.inlineQuery != null ? '' : 'opacity-0'"
    >
      <!-- Quick inline search -->
      <button
        tabindex="-1"
        class="mb-0.5 rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
        @click="() => toggleInlineSearch()"
      >
        <MagnifyingGlassIcon class="h-4 w-4" />
      </button>
      <div
        v-if="properties.inlineQuery != null"
        class="relative h-full w-40 transition-transform duration-150"
        @click="searchRef?.focus"
      >
        <EditableSpan
          ref="searchRef"
          :class="context.focused.value ? '' : 'h-0'"
          :model-value="properties.inlineQuery ?? ''"
          :readonly="false"
          @update:model-value="(v) => (properties.inlineQuery = v)"
          @keydown.escape.exact.prevent="toggleInlineSearch"
          class="overflow-hidden whitespace-nowrap"
          placeholder
        />
        <!-- Placeholder -->
        <span class="text-gray-400" v-if="(properties.inlineQuery ?? '').trim() == ''">Type to search...</span>
        <!-- Cancel button -->
        <button
          tabindex="-1"
          v-if="(properties.inlineQuery ?? '').trim() != ''"
          class="absolute right-0 top-0 h-full rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
          @click="() => (properties.inlineQuery = undefined)"
        >
          <XCircleIconOutline class="h-4 w-4" />
        </button>
      </div>
      <!-- Other actions -->
      <!-- not entirely sure why we need the margin hack here and above... -->
      <StatementActions class="-mt-0.5" :extraActions="extraActions" />
      <CreateFieldInterface
        ref="createFieldRef"
        :title="'New field on ' + context.statement.value.name"
        @select="createNewField"
      />
    </div>
  </div>
  <!-- Folded info -->
  <button
    v-if="folded"
    class="mt-0.5 flex max-w-full flex-shrink flex-row gap-1.5 truncate text-gray-400"
    :class="folded ? 'rounded-sm hover:bg-gray-100' : ''"
    @click="$emit('toggleFold')"
  >
    <span>{{ humanizeNumber(totalCount) }} {{ totalCount == 1 ? "record" : "records" }}</span>
    <!-- folded info -->
    <template v-if="folded">
      •
      <span v-for="field in context.allFields.value" :key="field.id">{{ field.name }}</span>
    </template>
  </button>
  <!-- Description -->
  <EditableSpan
    v-if="!folded"
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
    v-if="!folded && description.length == 0 && !context.readonly.value && addingDescription"
    @click="descriptionRef?.focus()"
    class="-mx-0.5 w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
  >
    Add description
  </button>
  <!-- Sorts/filters -->
  <div
    v-if="(!folded && (properties.sorts ?? []).length > 0) || properties.query != null"
    class="-mx-0.5 mb-1 mt-1 flex flex-row flex-wrap gap-1.5"
  >
    <!-- Sort pills -->
    <span
      v-for="sort in properties.sorts ?? []"
      :key="sort.key"
      class="flex w-fit flex-row items-center rounded-xl border border-gray-300 px-1.5 text-gray-900"
    >
      <span class="">{{ context.allFields.value.find((f) => sort.key.includes(f.key))?.name }}</span>
      <span class="ml-0.5 text-gray-700">{{ sort.order == SortOrder.Ascending ? "↑" : "↓" }}</span>
      <!-- Clear button -->
      <button @click="removeSort(sort)">
        <XMarkIcon class="h-3 w-3 text-gray-400" />
      </button>
    </span>
    <!-- Filter pills (if simple) -->
    <!-- (soon) -->
  </div>
  <!-- Table (in table form but manually sized) -->
  <!-- Wrapper to contain any scrolling -->
  <div
    v-if="!folded"
    ref="gridRef"
    class="overflow-x-auto"
    :style="{
      'margin-left': -gridOffsetX + 'px',
      'margin-right': -gridOffsetX + 'px',
      'padding-left': gridOffsetX + 'px',
      'padding-right': gridOffsetX + 'px',
      'max-width': editor.size.value.width + 'px',
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
      ></div>
      <!-- Header (with types) -->
      <!-- To make this 'sticky' without creating a new stacking context we position it absolutely 'above' the placeholder above  -->
      <div
        class="z-[1] flex flex-row self-start border-b border-orange-900 border-opacity-[12%]"
        :class="(context.focused.value && !context.editing.value) || !hasFloatingHeader ? '' : 'bg-white'"
        :style="{
          position: hasFloatingHeader ? 'fixed' : 'absolute',
          left: hasFloatingHeader
            ? -gridScrollOffsetX + 4 + editor.pos.value.left + gridOffsetX + 'px'
            : -gridScrollOffsetX + 4 + 'px',
          top: hasFloatingHeader ? editor.pos.value.top + appearance.editorHeaderHeight + 'px' : undefined,
          /* clip to editor bounds (different stacking context so need to 're-clip' into editor) */
          clipPath: hasFloatingHeader ? `inset(0px ${gridOverhangRight}px 0px ${gridOverhangLeft}px)` : undefined,
        }"
      >
        <div v-for="(field, x) in context.allFields.value" :key="field?.id" class="">
          <div class="whitespace-nowrap focus-within:bg-orange-100">
            <FieldInterface
              :ref="(el: any) => grid.registerColumnRef('', field.key as string, el)"
              :key="field?.id + '.header'"
              :type="field"
              :readonly="context.readonly.value"
              :inlined="context.inheritedFields.value.find((n) => n.key == field.key) != null"
              is-view
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
              @sort="(order) => addSort(field, order)"
              @enter="grid.navigateDown('', field.key as string)"
              :style="{
                width: columnWidths[x] + 'px',
              }"
            />
          </div>
        </div>
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
              v-if="!context.readonly.value"
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
              @click="true /* TODO @UX: do something on database properties column button */"
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
                v-if="!context.readonly.value"
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
                  <DragHandleIcon class="h-4 w-4" />
                </div>
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
            :model-value="record.value?.[module.getTypedKey(field) as string]"
            @update:model-value="(val) => writeRecordField(record.id, module.getTypedKey(field) as string, val)"
            :type="module.effectiveTypeOf(field)"
            :readonly="context.readonly.value"
            :active="context.editing.value || context.focused.value"
            :wrap="
              Boolean(
                properties.wrapColumns
              ) /* TODO @Cleanup: not sure why the Boolean is needed, but wrapColumns is an object otherwise? */
            "
            debounced
            :supports-drop="!context.readonly.value"
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
        {{ recordsError.message }}
      </button>
      <!-- Load more/loading -->
      <button
        v-if="pageInfo?.hasNextPage"
        ref="loadMoreRef"
        class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900 border-opacity-[12%] px-1 py-1 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        :style="{ height: minRowHeight + 'px' }"
        @click.stop="loadMore()"
        @keydown.up.exact.prevent="focusLastRecord"
        @keydown.down.exact.prevent="context.navigateDown"
        :disabled="loading"
      >
        <template v-if="loading">
          <BusySpinnerIcon class="mr-1 h-4 w-4" :class="loading ? 'animate-spin' : ''" />
          Loading
        </template>
        <template v-else>
          <ArrowDownIcon class="h-4 w-4" />
          Load more
        </template>
      </button>
      <!-- Insert button -->
      <button
        v-if="!context.readonly.value"
        ref="addRecordRef"
        class="flex w-full select-none flex-row items-center gap-0.5 rounded-sm border-b border-orange-900 border-opacity-[12%] px-1 py-1 text-gray-300 outline-none transition duration-75 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        :style="{ height: minRowHeight + 'px' }"
        @click.stop="insertRecordAtEnd()"
        @keydown.up.exact.prevent="(loadMoreRef?.focus ?? focusLastRecord)()"
        @keydown.down.exact.prevent="context.navigateDown"
        :disabled="loading"
      >
        <PlusIcon class="h-4 w-4" /> New
      </button>
    </div>
  </div>
</template>
