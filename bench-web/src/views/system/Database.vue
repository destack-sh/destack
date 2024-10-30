<script lang="ts" setup>
import { blockToType } from "@/language/block";
import { getPropertyName, getPropertyTitle, TYPE_BLOCK_TYPES } from "@/language/const";
import { makeAndConditional, makeExpression } from "@/language/expression";
import { createField, getPropertyType, getStorageKey, makeTypeInfo, NAME_TYPE, TypeIdentity } from "@/language/field";
import { cloneNode, moveNode } from "@/language/node";
import { DebounceLevel, newChangeId, Transaction } from "@/language/transaction";
import { packValue, unpackValue } from "@/language/value";
import {
  BenchType,
  ChangeCategory,
  EditOperationData,
  EditOperationType,
  ExpressionData,
  ExpressionType,
  FieldData,
  FieldType,
  IconData,
  NodeType,
  ObjectType,
  Orientation,
  PrimitiveType,
  PropertyInfo,
  RecordData,
  RecordProperty,
  SelectionData,
  Timestamp,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import {
  describeNode,
  isNode,
  propertyInfo,
  propertyReference,
  toPlainNodeRef,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { SearchConnectionParams, useExistingConnection, useSearchConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { ActionContext, ActionMapImplementation } from "@/ui/action";
import { DraggedContent, MultiAnchor, startDraggingIfAllowed, useMultiDropZone } from "@/ui/drag";
import { getNodeIcon, getTypeIcon, ICON_BY_EXPRESSION_OP as ICON_BY_EXPRESSION_TYPE, IconInline } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { menuActionsLike, PopoverContext, PopoverInfoIn, pushPopover } from "@/ui/popover";
import {
  collapseSelection,
  expandSelection,
  focusInElement,
  getViewForValueType,
  VIEW_DEFAULT_HEADER_HEIGHT,
} from "@/ui/view";
import { assertNever } from "@/utils/functools";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { getViewComponent } from "@/views/registry";
import { MaybeElement, useElementSize, useEventListener, useKeyModifier } from "@vueuse/core";
import { computed, ref, Ref, shallowRef, toRef } from "vue";

const ACTION_HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const ROW_HEIGHT_MIN = 32;
const ROW_HEIGHT_MAX = 200;
const ROW_ACTIONS_WIDTH = 32;
const FULL_PADDING = 12;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; paddingX?: number; paddingY?: number } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "variant" | "isInput" | "selection">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const spaceTx = () => spaceConnection.tx.with({ category: ChangeCategory.SPACE });
const state = canvas.registerView(self, id);

// NOTE :UX: Database view should be factored out into Table/Feed/etc. query views (?)

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const preparedPkgConnection = useExistingConnection(nodePtr);
const { graph: pkgGraph, graphRaw: pkgGraphRaw, connection: pkgConnection } = preparedPkgConnection;
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: true });
const blockRaw = pkgGraphRaw.getRef(nodePtr);
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);
const fieldsRaw = pkgGraphRaw.getChildrenRef(blockRaw, NodeType.FIELD);

//
// Search/filter
//

const DEFAULT_SORT = makeExpression({
  type: ExpressionType.DESCENDING,
  propertyPtr: propertyReference(NodeType.RECORD, RecordProperty.createdAt),
});
const filters: Ref<ExpressionData[]> = shallowRef([]);
const sorts: Ref<ExpressionData[]> = shallowRef([]);

/** Finds the column associated with the given expression in this Table view (if any) */
function findColumn(expression: ExpressionData): ColumnView | null {
  if (expression.propertyPtr != null) {
    return columns.value.find((c) => c.kind == "property" && c.property.id == expression.propertyPtr?.id) ?? null;
  } else if (expression.fieldPtr != null) {
    return columns.value.find((c) => c.kind == "field" && c.field.ck == expression.fieldPtr?.ck) ?? null;
  } else {
    return null;
  }
}

/** Add/replace sort for the given column */
function addSort(column: ColumnView, type: ExpressionType) {
  if (column.kind == "property") {
    const existing = sorts.value.find((sort) => sort.propertyPtr?.id == column.property.id);
    const sort = makeExpression({ type, propertyPtr: propertyReference(NodeType.RECORD, column.property.id) });
    if (existing != null) {
      sorts.value = [...sorts.value.filter((sort) => sort.propertyPtr?.id != column.property.id), sort];
    } else {
      sorts.value = [...sorts.value, sort];
    }
  } else if (column.kind == "field") {
    const existing = sorts.value.find((sort) => sort.fieldPtr?.ck == column.field.ck);
    const sort = makeExpression({ type, blockPtr: nodePtr.value, fieldPtr: toPlainNodeRef(column.field) });
    if (existing != null) {
      sorts.value = [...sorts.value.filter((sort) => sort.fieldPtr?.ck != column.field.ck), sort];
    } else {
      sorts.value = [...sorts.value, sort];
    }
  } else {
    assertNever(column);
  }
}

// NOTE :Cleanup: unfortunately we need to wait until the block we're querying actually exists remotely :SearchWithMissingBlock
const limit = computed(() => (props.variant == Variant.COMPACT ? 10 : 25));
const {
  graph: recordGraph,
  roots: records,
  connection: recordConnection,
  isStale,
  isConnected,
  isConnecting,
  page,
} = useSearchConnection(
  { name: "database.records", live: true },
  computed(
    (): SearchConnectionParams<NodeType.RECORD> => ({
      scope: PACKAGE_SCOPE.value,
      nodeType: NodeType.RECORD,
      first: limit.value,
      count: true,
      blockPtr: nodePtr.value,
      isEnabled: nodePtr.value != null && blockRaw.value != null,
      sort: sorts.value.length > 0 ? sorts.value : [DEFAULT_SORT],
      filter: makeAndConditional(filters.value),
      select: {
        metatype: ObjectType.SELECT_OPTIONS,
        selectFieldsPtr: fieldsRaw.value.map(toPlainNodeRef),
      },
    }),
  ),
);

//
// State
//

const containerRef = ref<HTMLDivElement | null>(null);
const headerRef: Ref<HTMLDivElement | null> = ref(null);
const bodyRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const columnHeaderRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const cellWrapperRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const cellComponentRefs: Ref<Record<string, ViewExposed>> = ref({});
const shiftKey = useKeyModifier("Shift");

const containerSize = useElementSize(containerRef);
const rowBodyWidth = computed(() => {
  if (props.variant == Variant.COMPACT) {
    return containerSize.width.value;
  } else {
    return containerSize.width.value - ROW_ACTIONS_WIDTH;
  }
});
const bodySize = computed(() => {
  return { width: rowBodyWidth.value, height: containerSize.height.value - ACTION_HEADER_HEIGHT };
});

function getCellId(record: RecordData, column: ColumnView) {
  return `${record.id}.${column.id}`;
}

// NOTE :UX: add records optimistically in Database?
//  (right now, they only show up once committed in the backend and the search connection is updated from there)
function createRecord() {
  if (nodePtr.value == null || block.value == null) throw new Error("no block to add record to");
  const record = recordConnection.tx.create({
    metatype: NodeType.RECORD,
    blockPtr: nodePtr.value,
    packagePtr: block.value.packagePtr,
    parentPtr: nodePtr.value,
    valuePacked: {},
  });
  canvas.inspect({ node: record, view: containerRef.value });
}

function getColumnMinWidth(type: TypeIdentity, viewType: ViewType | undefined) {
  if (type.primitiveType == PrimitiveType.BOOLEAN) {
    return 100;
  } else if (
    type.primitiveType == PrimitiveType.DATE ||
    type.primitiveType == PrimitiveType.DATETIME ||
    type.primitiveType == PrimitiveType.TIME ||
    type.primitiveType == PrimitiveType.INTERVAL
  ) {
    return 150;
  } else {
    return 200;
  }
}

function getColumnDebounce(type: TypeIdentity, viewType: ViewType | undefined): DebounceLevel {
  if (type.primitiveType == PrimitiveType.BOOLEAN) {
    return "tick";
  } else if (type.benchType == BenchType.TEXT || type.benchType == BenchType.CODE) {
    return "long";
  } else {
    return "short";
  }
}

function getColumnPadding(
  type: TypeIdentity,
  viewType: ViewType | undefined,
): { paddingTop: number; paddingBottom: number } {
  // calibrated against ROW_HEIGHT to ensure all types look center-aligned at the default height
  if (viewType == ViewType.TOGGLE) {
    return { paddingTop: 7, paddingBottom: 0 };
  } else if (viewType == ViewType.TEXT) {
    return { paddingTop: 5, paddingBottom: 2 };
  } else {
    return { paddingTop: 5, paddingBottom: 5 };
  }
}

// TODO :Incomplete!: store column views somewhere (in TableView/DatabaseView?) :RichColumns
type ColumnContent =
  | {
      kind: "property";
      property: PropertyInfo;
      propertyName: string;
    }
  | {
      kind: "field";
      field: FieldData;
      storageKey: string;
    };
type ColumnView = {
  idx: number;
  id: string;
  icon: IconData | undefined;
  title: string;
  type: TypeIdentity;
  viewType: ViewType | undefined;
  viewComponent: any | undefined;
  viewProps: any | undefined;
  width: number;
  debounce: DebounceLevel;
  isInput: boolean;
  isInspected: boolean;
  isHighlighted: boolean;
  isSelected: boolean;
  paddingTop: number;
  paddingBottom: number;
} & ColumnContent;
const columns: Ref<ColumnView[]> = computed(() => {
  // NOTE :UX: support Table property columns properly :RichColumns
  const properties: RecordProperty[] = [];
  const columns: ColumnView[] = [];

  function addColumn(
    columnIn: Pick<
      ColumnView,
      "id" | "icon" | "title" | "type" | "isInput" | "isHighlighted" | "isInspected" | "isSelected"
    > &
      ColumnContent,
  ) {
    const view = getViewForValueType(columnIn.type);
    const padding = getColumnPadding(columnIn.type, view?.type);
    const column: ColumnView = {
      ...columnIn,
      idx: columns.length,
      viewType: view?.type,
      viewComponent: view?.type != null ? getViewComponent(view?.type) : null,
      viewProps: { ...view, isInput: columnIn.isInput },
      width: getColumnMinWidth(columnIn.type, view?.type),
      debounce: getColumnDebounce(columnIn.type, view?.type),
      paddingTop: padding.paddingTop,
      paddingBottom: padding.paddingBottom,
    };
    columns.push(column);
  }

  for (const propertyId of properties) {
    // :RichColumns
    const property = propertyInfo(NodeType.RECORD, propertyId);
    const propertyType = getPropertyType(property);
    addColumn({
      id: propertyId.toString(),
      icon: getTypeIcon(propertyType),
      title: getPropertyTitle(property),
      type: propertyType,
      kind: "property",
      property,
      propertyName: getPropertyName(property),
      isInput: false,
      isInspected: false,
      isHighlighted: false,
      isSelected: false,
    });
  }
  for (const field of fields.value) {
    addColumn({
      kind: "field",
      id: field.id,
      title: field.name,
      icon: getNodeIcon(field),
      type: field,
      field,
      storageKey: getStorageKey(field),
      isInput: true,
      isHighlighted: canvas.isHighlighted(field),
      isInspected: canvas.isInspected(field),
      isSelected: isSelectedColumn(field),
    });
  }

  // grow columns to fit container (if possible)
  const totalWidth = columns.reduce((acc, column) => acc + column.width, 0);
  const availableWidth = rowBodyWidth.value;
  if (totalWidth < availableWidth && columns.length > 0) {
    const scale = availableWidth / totalWidth;
    for (const column of columns) {
      column.width = Math.round(column.width * scale);
    }
  }

  return columns;
});

function readColumnValue(record: RecordData, column: ColumnView) {
  if (column.kind === "property") {
    return (record as any)[column.propertyName];
  } else if (column.kind === "field") {
    const valuePacked = (record.valuePacked as any)?.[column.storageKey];
    return unpackValue(valuePacked, column.type, { wrapScalar: false });
  } else {
    assertNever(column);
  }
}

function writeColumnValue(
  record: RecordData,
  column: ColumnView,
  newValue: any | undefined,
  options?: { tx?: Transaction; debounce?: DebounceLevel },
) {
  const tx = options?.tx ?? recordConnection.tx;
  if (options?.debounce == null) options = { ...(options ?? {}), debounce: column.debounce };
  if (column.kind == "property") {
    tx.update(record, { [column.propertyName]: newValue }, options);
  } else if (column.kind == "field") {
    const oldValue = readColumnValue(record, column);
    const newValuePacked = packValue(newValue, column.type, { wrapScalar: false });
    const oldValuePacked = packValue(oldValue, column.type, { wrapScalar: false });
    const operations: EditOperationData[] = [
      {
        metatype: ObjectType.EDIT_OPERATION,
        type: newValue == null ? EditOperationType.CLEAR : EditOperationType.SET,
        path: [RecordProperty.valuePacked.toString(), column.storageKey],
        newValuePacked,
        oldValuePacked,
      },
    ];
    tx.update(record, operations, options);
  } else {
    assertNever(column);
  }
}

//
// Selection
//

const selectedRecordsById: Ref<Record<string, RecordData>> = computed(() => {
  const selectedRecords = new Set(props.selection?.nodesPtr?.map((ptr) => ptr.id));
  return records.value
    .filter((record) => selectedRecords.has(record.id))
    .reduce(
      (acc, record) => {
        acc[record.id] = record;
        return acc;
      },
      {} as Record<string, RecordData>,
    );
});
const selectedFieldsByCk: Ref<Record<string, FieldData>> = computed(() => {
  const selectedFields = new Set(props.selection?.fieldsPtr?.map((ptr) => ptr.ck));
  return fields.value
    .filter((field) => selectedFields.has(field.ck))
    .reduce(
      (acc, field) => {
        acc[field.ck] = field;
        return acc;
      },
      {} as Record<string, FieldData>,
    );
});
const selectedColumns = computed(() => columns.value.filter((column) => column.isSelected));
const hasSelectionRows = computed(
  () => Object.keys(selectedRecordsById.value).length > 0 && numSelectedColumns.value == 0,
);
const numSelectedColumns = computed(() => Object.keys(selectedFieldsByCk.value).length);
const hasSelectionRegion = computed(() => numSelectedRows.value > 0 && numSelectedColumns.value > 0);
const numSelectedRows = computed(() => Object.keys(selectedRecordsById.value).length);
const isAllSelectedRows = computed(() => numSelectedRows.value >= records.value.length);
const isAllSelectedColumns = computed(() => numSelectedColumns.value == 0);
const lastSelectedRow: Ref<RecordData | null> = ref(null);

function isSelectedRow(record: RecordData) {
  return selectedRecordsById.value[record.id] != null;
}

function isSelectedColumn(field: FieldData) {
  return selectedFieldsByCk.value[field.ck] != null;
}

function isSelectedCell(record: RecordData, column: ColumnView) {
  if (!isSelectedRow(record)) return false;
  if ((props.selection?.fieldsPtr?.length ?? 0) == 0) return true;
  return column.kind == "field" && isSelectedColumn(column.field);
}

function addSelectionRow(record: RecordData) {
  state.update({ selection: expandSelection(props.selection, [record]) }, { debounce: "tick" });
}

function removeSelectionRow(record: RecordData) {
  if (props.selection == null) return;
  state.update({ selection: collapseSelection(props.selection, [record]) }, { debounce: "tick" });
}

function setSelectionRow(record: RecordData, selected: boolean, expandFromLast: boolean) {
  if (hasSelectionRegion.value) {
    // clear selection first
    selectNone();
  }
  if (selected) {
    const lastSelectedY = records.value.findIndex((r) => r.id == lastSelectedRow.value?.id);
    const currentY = records.value.findIndex((r) => r.id == record.id);
    if (expandFromLast && lastSelectedY >= 0 && currentY >= 0) {
      const from = Math.min(lastSelectedY, currentY);
      const to = Math.max(lastSelectedY, currentY);
      const selection = records.value.slice(from, to + 1);
      state.update({ selection: expandSelection(props.selection, selection) }, { debounce: "tick" });
    } else {
      addSelectionRow(record);
    }
    lastSelectedRow.value = record;
  } else {
    removeSelectionRow(record);
  }
}

function selectAll() {
  state.update(
    { selection: { metatype: ObjectType.SELECTION, nodesPtr: records.value.map(toPlainNodeRef), fieldsPtr: [] } },
    { debounce: "tick" },
  );
}

function selectNone() {
  lastSelectedRow.value = null;
  if (props.selection != undefined) {
    state.update({ selection: undefined }, { debounce: "tick" });
  }
}

// local selection region
const selectedRegionOrigin = ref<{ x: number; y: number } | null>(null);
const selectedRegionEnd = ref<{ x: number; y: number } | null>(null);

function beginSelectRegion(e: MouseEvent, y: number, row: RecordData, x: number, column: ColumnView) {
  selectedRegionOrigin.value = { x, y };
  updateSelectRegion(e, y, row, x, column);
}

function updateSelectRegion(e: MouseEvent, y: number, row: RecordData, x: number, column: ColumnView) {
  if (selectedRegionOrigin.value == null) return; // not selecting
  if (selectedRegionEnd.value != null && y == selectedRegionEnd.value.y && x == selectedRegionEnd.value.x) return; // no change
  selectedRegionEnd.value = { x, y };
  const region = {
    x1: Math.min(selectedRegionOrigin.value.x, selectedRegionEnd.value.x),
    x2: Math.max(selectedRegionOrigin.value.x, selectedRegionEnd.value.x),
    y1: Math.min(selectedRegionOrigin.value.y, selectedRegionEnd.value.y),
    y2: Math.max(selectedRegionOrigin.value.y, selectedRegionEnd.value.y),
  };

  const selection: SelectionData = {
    metatype: ObjectType.SELECTION,
    nodesPtr: records.value.slice(region.y1, region.y2 + 1).map(toPlainNodeRef),
    fieldsPtr: columns.value
      .slice(region.x1, region.x2 + 1)
      .filter((column) => column.kind == "field")
      .map((column) => toPlainNodeRef(column.field)),
  };
  state.update({ selection }, { debounce: "long" });
}

function endSelectRegion() {
  selectedRegionOrigin.value = null;
  selectedRegionEnd.value = null;
}

useEventListener(window, "mouseup", endSelectRegion);
useEventListener(containerRef, "mousedown", (e) => {
  // clear selection if we're not inside a row or the header
  if (props.selection == null) return;
  const node = canvas.getNodeAt(e.target as HTMLElement);
  if (node?.nodeType != NodeType.RECORD && !headerRef.value?.contains(e.target as HTMLElement)) {
    selectNone();
  }
});

// working with selection

function duplicateSelection(): RecordData[] {
  if (!hasSelectionRows.value) throw new Error("no row selection to duplicate");
  const tx = recordConnection.tx.with({ change: { key: newChangeId(), title: "Duplicate records" } });
  const now = Timestamp.now();
  const clonedRecords: RecordData[] = [];
  for (const record of Object.values(selectedRecordsById.value)) {
    const clonedRecord = cloneNode(tx, recordGraph, record, { now });
    clonedRecords.push(clonedRecord);
  }
  return clonedRecords;
}

function deleteSelection() {
  if (hasSelectionRows.value) {
    // delete all selected records
    const tx = recordConnection.tx.with({ change: { key: newChangeId(), title: "Delete records" } });
    for (const record of Object.values(selectedRecordsById.value)) {
      tx.delete(record);
    }
  } else if (hasSelectionRegion.value) {
    // clear selected fields
    const tx = recordConnection.tx.with({ change: { key: newChangeId(), title: "Clear cells" } });
    for (const record of Object.values(selectedRecordsById.value)) {
      for (const column of selectedColumns.value) {
        if (column.kind == "field") {
          writeColumnValue(record, column, undefined, { tx });
        }
      }
    }
  } else {
    throw new Error("no selection to delete");
  }
}

//
// Drag & drop :TypeDragAndDrop
//

function allowDrop(dragged: DraggedContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent) {
  if (dragged.kind != "node") return false;
  const node = pkgGraph.getOrError(dragged.node);
  return isNode(node, NodeType.FIELD) || (isNode(node, NodeType.BLOCK) && TYPE_BLOCK_TYPES.includes(node.type));
}
function onDrop(dragged: DraggedContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
  if (dragged.kind != "node") return;
  const node = pkgGraph.getOrError(dragged.node);
  const target = targetId != null ? pkgGraph.get({ id: targetId }) : null;
  if (isNode(node, NodeType.FIELD)) {
    // move field
    if (target != null) {
      if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor, target });
    } else {
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor: "center", target: block.value! });
    }
  } else if (isNode(node, NodeType.BLOCK)) {
    // add field with block type
    const type = blockToType(node);
    const fieldIn = { ...type, zone: FieldType.MEMBER };
    if (target != null) {
      if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
      createField(pkgConnection.tx, pkgGraph, { field: fieldIn, anchor, target });
    } else {
      createField(pkgConnection.tx, pkgGraph, { field: fieldIn, anchor: "inside", target: block.value! });
    }
  }
}
const { activeDropZone: activeHeaderDropZone } = useMultiDropZone({
  name: "database.header",
  container: headerRef,
  targets: columnHeaderRefs,
  orientation: Orientation.HORIZONTAL,
  kinds: ["node"],
  metatypes: [NodeType.BLOCK, NodeType.FIELD],
  fallbackToClosest: true,
  allowDrop,
  onDrop,
});

//
// Actions
//

const getNodeFromContext = (ctx: ActionContext | undefined): { node: FieldData | RecordData | null } => {
  const node = ctx?.triggerNode;
  if (isNode(node, NodeType.FIELD) || isNode(node, NodeType.RECORD)) {
    return { node };
  }
  return { node: null };
};
const actions: Partial<ActionMapImplementation<"common" | "database">> = {
  // common
  "common.create.record": () => createRecord(),
  "common.edit.delete": (action, ctx) => {
    if (hasSelectionRows.value || hasSelectionRegion.value) {
      deleteSelection();
    } else {
      const { node } = getNodeFromContext(ctx);
      if (node != null) {
        pkgConnection.tx.delete(node);
      }
    }
  },
  "common.edit.duplicate": (action, ctx) => {
    if (hasSelectionRegion.value) {
      return false; // can't duplicate region selection
    } else if (hasSelectionRows.value) {
      duplicateSelection();
    } else {
      const { node } = getNodeFromContext(ctx);
      if (node != null) {
        const tx = pkgConnection.tx.with({ change: { key: newChangeId(), title: "Duplicate record" } });
        cloneNode(tx, recordGraph, node);
      }
    }
  },
  // database
  "database.column.sortAscending": (action, ctx) => {
    const { node } = getNodeFromContext(ctx);
    if (!isNode(node, NodeType.FIELD)) return false;
    const column = columns.value.find((column) => column.kind == "field" && column.field == node);
    if (column == null) return false;
    addSort(column, ExpressionType.ASCENDING);
  },
  "database.column.sortDescending": (action, ctx) => {
    const { node } = getNodeFromContext(ctx);
    if (!isNode(node, NodeType.FIELD)) return false;
    const column = columns.value.find((column) => column.kind == "field" && column.field == node);
    if (column == null) return false;
    addSort(column, ExpressionType.DESCENDING);
  },
};

defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    ref="containerRef"
    class="h-full select-none"
    :style="{
      marginLeft: variant == Variant.COMPACT ? '0' : `${FULL_PADDING}px`,
      marginRight: variant == Variant.COMPACT ? '0' : `${FULL_PADDING}px`,
      marginBottom: variant == Variant.COMPACT ? '0' : `${FULL_PADDING}px`,
    }"
  >
    <!-- Action header -->
    <div
      class="flex w-full flex-row items-center gap-x-1"
      :style="{
        height: `${ACTION_HEADER_HEIGHT}px`,
        paddingLeft: `${props.paddingX ?? 0}px`,
        paddingRight: `${props.paddingX ?? 0}px`,
      }"
    >
      <!-- Expressions (filters/sorts) -->
      <!-- TODO :UX: Incomplete: filter/sort Database/Table view properly -->
      <div class="flex flex-row items-center gap-x-1">
        <!-- (this should of course be Expression views) -->
        <div
          v-for="(sort, i) in sorts.length > 0 ? sorts : [DEFAULT_SORT]"
          :key="i"
          class="group rounded-sm border border-sky-500 bg-sky-400 px-2 py-0.5"
        >
          <IconInline v-bind="ICON_BY_EXPRESSION_TYPE[sort.type]" class="mr-1.5 text-gray-700" />
          <span class="text-gray-900">{{
            findColumn(sort)?.title ?? getPropertyTitle(DEFAULT_SORT.propertyPtr!)
          }}</span>
          <button class="ml-1.5 text-gray-400 opacity-0 group-hover:opacity-100" @click="() => sorts.splice(i, 1)">
            <i class="fas fa-xmark" />
          </button>
        </div>
      </div>
      <!-- Meta (pagination, status, controls) -->
      <div class="ml-auto flex flex-row items-center gap-x-2">
        <!-- Staleness/Loading -->
        <Transition
          enter-active-class="transition-opacity ease-in duration-150"
          enter-from-class="opacity-0"
          enter-to-class="opacity-100"
          leave-active-class="transition-all ease-out duration-150"
          leave-from-class="opacity-100"
          leave-to-class="opacity-0"
        >
          <span v-if="isStale || isConnecting" class="ml-1">
            <i class="fas fa-circle-small animate-pulse text-gray-400" />
          </span>
        </Transition>

        <!-- Selection -->
        <div v-if="hasSelectionRows" class="flex flex-row items-center rounded-sm border">
          <button class="h-full px-2 py-0.5 font-medium text-primary-700 hover:bg-gray-100" @click="selectNone">
            {{ numSelectedRows }} selected
          </button>
          <button
            v-tooltip="{ title: 'Duplicate', small: true }"
            class="w-8 border-x py-0.5 text-gray-700 hover:bg-gray-100 hover:text-primary-700"
            @click="duplicateSelection"
          >
            <i class="fas fa-clone" />
          </button>
          <button
            v-tooltip="{ title: 'Delete', small: true }"
            class="w-8 py-0.5 text-gray-700 hover:bg-gray-100 hover:text-primary-700"
            @click="deleteSelection"
          >
            <i class="fas fa-trash-can" />
          </button>
        </div>

        <!-- Pagination -->
        <!-- TODO :Incomplete: Database pagination -->
        <div>
          <span v-if="page?.total != null" class="text-gray-400">{{ page.size }} / {{ page?.total }}</span>
        </div>

        <!-- Controls -->
        <!-- Add field -->
        <button
          class="group/button rounded-sm border border-emerald-500 bg-emerald-400 px-2 py-0.5 hover:bg-emerald-300"
          @click="
            (e) => {
              const button = (e.target as HTMLElement).closest('button')!;
              pushPopover({
                trigger: button,
                reference: button,
                info: {
                  component: ViewType.PICKER,
                  placement: 'bottom-left',
                  offset: 'referenceWidth',
                  props: { valueType: makeTypeInfo({ benchType: BenchType.TYPE_INFO }) },
                  onApply: (typeInfo: TypeIdentity) => {
                    createField(pkgConnection.tx, pkgGraph, { anchor: 'inside', target: block!, field: typeInfo });
                  },
                },
              });
            }
          "
        >
          <i class="fa fa-plus mr-1.5 text-center group-hover/button:text-emerald-900" />
          <span>Field</span>
        </button>
        <!-- Add record -->
        <button
          class="group/button rounded-sm border border-sky-500 bg-sky-400 px-2 py-0.5 hover:bg-sky-300"
          @click="() => createRecord()"
        >
          <i class="fa fa-plus mr-1.5 text-center group-hover/button:text-sky-900" />
          <span class="">Record</span>
        </button>
      </div>
    </div>

    <!-- Body (scroll horizontally, and vertically if not compact) -->
    <Scroll
      id="body"
      ref="bodyRef"
      :size="bodySize"
      :orientation="variant == Variant.COMPACT ? Orientation.HORIZONTAL : undefined"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
    >
      <div class="flex flex-col border-gray-300">
        <!-- Column headers (sticky) -->
        <div
          ref="headerRef"
          class="z-20 flex flex-row items-center border-gray-300"
          :class="[variant != Variant.COMPACT ? 'sticky top-0' : '']"
          :style="{
            height: `${ROW_HEIGHT_MIN}px`,
          }"
        >
          <!-- Composite actions -->
          <div
            v-if="variant != Variant.COMPACT"
            class="group sticky left-0 z-30 flex flex-shrink-0 flex-row items-center justify-center border-b transition-colors duration-150"
            :class="[hasSelectionRows ? 'border-gray-300 bg-white' : 'border-transparent bg-transparent']"
            :style="{
              width: `${ROW_ACTIONS_WIDTH}px`,
              height: `${ROW_HEIGHT_MIN}px`,
            }"
          >
            <!-- Selection checkbox -->
            <input
              type="checkbox"
              class="h-4 w-4 rounded-sm border-gray-300 transition-colors duration-150 focus:ring-0"
              :class="[hasSelectionRows ? 'opacity-100' : 'opacity-0 group-hover:opacity-100']"
              :checked="isAllSelectedRows"
              @change="(e) => ((e.target as HTMLInputElement).checked ? selectAll() : selectNone())"
            />
          </div>

          <!-- Column headers -->
          <div
            v-for="(column, x) in columns"
            :key="column.id"
            :ref="
              (ref: any) => (ref != null ? (columnHeaderRefs[column.id] = ref) : delete columnHeaderRefs[column.id])
            "
            v-contextmenu="
              (context: PopoverContext): PopoverInfoIn => {
                context = { ...context, triggerNode: column.kind == 'field' ? column.field : undefined };
                const items = [
                  ...menuActionsLike(
                    [
                      'common.edit.rename',
                      'common.edit.duplicate',
                      'common.edit.delete',
                      'database.column.sortAscending',
                      'database.column.sortDescending',
                      'database.column.filter',
                    ],
                    {
                      context,
                    },
                  ),
                ];
                return {
                  kind: 'menu',
                  placement: 'bottom-right',
                  items,
                  context,
                };
              }
            "
            class="relative flex h-full flex-shrink-0 cursor-pointer items-center border-b border-gray-300 border-l-transparent bg-white px-2 data-[dragging=true]:opacity-50"
            :class="[
              x > 0 ? 'border-l' : '',
              column.isInspected ? 'bg-sky-100' : column.isHighlighted ? 'bg-sky-50' : 'hover:bg-gray-100',
            ]"
            :style="{
              paddingLeft: x == 0 ? `${paddingX ?? 0}px` : undefined,
              paddingRight: x == columns.length - 1 ? `${paddingX ?? 0}px` : undefined,
              width: `${column.width}px`,
            }"
            :data-node-id="column.kind == 'field' ? column.field.ck : undefined"
            :data-node-ck="column.kind == 'field' ? column.field.ck : undefined"
            :data-node-type="column.kind == 'field' ? column.field.metatype : undefined"
            :draggable="column.kind == 'field'"
            @dragstart.stop="
              (e: DragEvent) => column.kind == 'field' && startDraggingIfAllowed(e, pkgGraph, column.field)
            "
            @click="
              (e) => {
                if (column.kind == 'field') {
                  canvas.inspect({ node: column.field, view: e.target as HTMLElement });
                }
              }
            "
          >
            <!-- Drop indicator -->
            <div
              v-if="column.kind == 'field' && activeHeaderDropZone?.targetId == column.field.id"
              class="absolute z-10 h-full w-1 rounded-sm bg-primary-700"
              :class="[activeHeaderDropZone?.anchor == 'start' ? (x == 0 ? 'left-0' : '-left-[3px]') : '-right-[3px]']"
            />
            <!-- Icon/Name -->
            <IconInline
              v-menu="
                (): PopoverInfoIn => ({
                  component: Icon,
                  isEnabled: column.kind == 'field',
                  placement: 'bottom-right',
                  offset: '-referenceWidth',
                  props: { modelValue: column.kind == 'field' ? column.field.icon : undefined },
                  onApply: (newIcon) => {
                    if (column.kind != 'field') return;
                    pkgConnection.tx.update(column.field, { icon: newIcon });
                  },
                })
              "
              class="mr-1.5 rounded-sm p-0.5 text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
              v-bind="column.icon"
            />
            <NativeInput
              v-if="column.kind == 'field'"
              :id="column.id + '.name'"
              class="truncate font-medium"
              :model-value="column.title"
              :value-type="NAME_TYPE"
              :variant="Variant.STEALTH"
              is-input
              @update:model-value="
                (newValue) => pkgConnection.tx.update(column.field, { name: newValue as string }, { debounce: 'long' })
              "
            />
            <span v-else class="truncate font-medium">{{ column.title }}</span>
          </div>
        </div>
        <!-- Empty columns -->
        <div
          v-if="columns.length == 0"
          class="flex w-full flex-row items-center justify-center border-b text-gray-400 hover:bg-gray-100"
          :style="{
            height: `${ROW_HEIGHT_MIN}px`,
            marginLeft: variant == Variant.COMPACT ? undefined : `${ROW_ACTIONS_WIDTH}px`,
            width: variant == Variant.COMPACT ? undefined : `calc(100% - ${ROW_ACTIONS_WIDTH}px)`,
          }"
        >
          No columns.
        </div>

        <!-- Status (if not connected or empty) -->
        <div
          v-if="!isConnected"
          class="flex w-full flex-row items-center justify-center text-center"
          :style="{ height: `${ROW_HEIGHT_MIN}px` }"
        >
          <!-- Loading -->
          <Transition
            enter-from-class="opacity-0"
            enter-active-class="transition-opacity duration-200"
            enter-to-class="opacity-100"
            appear
            mode="out-in"
          >
            <i class="fas fa-spinner-third animate-spin text-gray-400" />
          </Transition>
        </div>
        <!-- No rows -->
        <button
          v-else-if="records.length == 0"
          class="w-full text-center text-gray-400 hover:bg-gray-100 hover:text-primary-700"
          :style="{ height: `${ROW_HEIGHT_MIN}px` }"
          @click="createRecord"
        >
          <i class="fas fa-empty-set mr-1.5" />
          <span class="">No records. Click to add.</span>
        </button>

        <!-- Row -->
        <div
          v-for="(record, y) in records"
          :key="record.id"
          v-contextmenu="
            (context: PopoverContext): PopoverInfoIn => {
              context = { ...context, triggerNode: record };
              return {
                kind: 'menu',
                placement: 'bottom-right',
                items: menuActionsLike(['common.edit.duplicate', 'common.edit.delete'], { context }),
                context,
              };
            }
          "
          class="group/row flex flex-row border-gray-300"
          :class="[]"
          :data-node-id="record.id"
          :data-node-ck="record.id"
          :data-node-type="record.metatype"
          @click="() => canvas.inspect({ node: record, view: containerRef })"
        >
          <!-- Row actions -->
          <div
            v-if="variant != Variant.COMPACT"
            class="sticky left-0 z-10 flex flex-shrink-0 flex-row items-start justify-center border-b pt-[7px] transition-colors duration-150"
            :class="[hasSelectionRows ? 'border-gray-300 bg-white' : 'border-transparent bg-transparent']"
            :style="{
              width: `${ROW_ACTIONS_WIDTH}px`,
            }"
          >
            <!-- Selection checkbox -->
            <input
              type="checkbox"
              class="h-4 w-4 rounded-sm border-gray-300 transition-colors duration-150 focus:ring-0"
              :class="[hasSelectionRows ? 'opacity-100' : 'opacity-0 group-hover/row:opacity-100']"
              :checked="hasSelectionRows && isSelectedRow(record)"
              @change="(e) => setSelectionRow(record, (e.target as HTMLInputElement).checked, shiftKey ?? false)"
            />
          </div>

          <!-- Cells -->
          <div
            v-for="(column, x) in columns"
            :ref="
              (ref: any) =>
                ref != null
                  ? (cellWrapperRefs[getCellId(record, column)] = ref)
                  : delete cellWrapperRefs[getCellId(record, column)]
            "
            class="flex-shrink-0 cursor-pointer overflow-hidden border-b border-gray-300 px-2 text-gray-900"
            :class="[x > 0 ? 'border-l' : '', isSelectedCell(record, column) ? 'bg-sky-100' : '']"
            :style="{
              width: `${column.width}px`,
              minHeight: `${ROW_HEIGHT_MIN}px`,
              maxHeight: `${ROW_HEIGHT_MAX}px`,
              paddingTop: `${column.paddingTop}px`,
              paddingBottom: `${column.paddingBottom + (y == records.length - 1 ? (props.paddingY ?? 0) : 0)}px`,
              paddingLeft: x == 0 ? `${paddingX ?? 8}px` : undefined,
              paddingRight: x == columns.length - 1 ? `${paddingX ?? 8}px` : undefined,
            }"
            :data-column-id="column.id /* used to mark this as a column for click handler below */"
            @click="
              (event) => {
                // interact with / focus cell component
                const componentEl = cellComponentRefs[getCellId(record, column)];
                if (componentEl != null) {
                  if (componentEl?.interact != null) componentEl.interact();
                  else focusInElement(componentEl as unknown as MaybeElement);
                }
              }
            "
            @mousedown="(e) => beginSelectRegion(e, y, record, x, column)"
            @mousemove="(e) => updateSelectRegion(e, y, record, x, column)"
          >
            <!-- Inner column view -->
            <component
              :is="column.viewComponent"
              v-if="column.viewComponent != null"
              :id="getCellId(record, column)"
              :ref="
                (ref: any) =>
                  ref != null
                    ? (cellComponentRefs[getCellId(record, column)] = ref)
                    : delete cellComponentRefs[getCellId(record, column)]
              "
              v-bind="column.viewProps"
              class="cursor-pointer select-none"
              :model-value="readColumnValue(record, column)"
              :variant="Variant.STEALTH"
              :size="{ width: column.width, height: ROW_HEIGHT_MAX }"
              @update:model-value="(value: any) => writeColumnValue(record, column, value)"
            />
            <!-- No view available (internal bug / missing feature) -->
            <span v-else class="text-danger-600">
              {{ column.viewType != null ? ViewType[column.viewType] : "???" }}
            </span>
          </div>
          <!-- No columns -->
          <div
            v-if="columns.length == 0"
            class="w-full border-b border-gray-300 text-center text-gray-400 hover:bg-gray-100 hover:text-primary-700"
            :style="{ height: `${ROW_HEIGHT_MIN}px` }"
          />
        </div>
      </div>
    </Scroll>
  </div>
</template>
