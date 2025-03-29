<script lang="ts" setup>
import { getPropertyName, getPropertyTitle } from "@/language/core/const";
import { makeAndConditional, makeExpression } from "@/language/core/expression";
import { moveNode, packSubnode } from "@/language/core/node";
import { getPropertyType, getStorageKey, makeType, NAME_TYPE, nodeToType, TypeIdentity } from "@/language/core/type";
import { packValue, unpackValue } from "@/language/core/value";
import {
  DebounceLevel,
  getTransactionOptionsForType,
  newChangeId,
  Transaction,
  TransactionOptions,
} from "@/language/runtime/transaction";
import { createField } from "@/language/source/field";
import {
  BenchType,
  EditOperationData,
  EditOperationType,
  ExpressionData,
  ExpressionType,
  FieldData,
  FieldType,
  IconData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  PickerVariant,
  PrimitiveType,
  PropertyInfo,
  RecordData,
  RecordProperty,
  TypeKind,
  ViewData,
  ViewType,
} from "@/proto/wire";
import {
  describeNode,
  isNode,
  propertyInfo,
  propertyReference,
  toNodeRef,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { SearchConnectionParams, useAutoConnection, useSearchConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { CommandMapKit, fireCommandById } from "@/ui/command";
import {
  DragContent,
  MultiAnchor,
  startDraggingIfAllowed,
  startSelectingIfAllowed,
  useMultiDropZone,
  useSelectionZone,
} from "@/ui/drag";
import { getNodeIcon, getTypeIcon, IconInline } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { PopoverInfoIn, pushDefaultMenu, pushPopover } from "@/ui/popover";
import { useNodeTableCommands } from "@/ui/table";
import { TooltipInfo } from "@/ui/tooltip";
import {
  collapseSelection,
  expandSelection,
  focusInElement,
  getViewForType,
  VIEW_DEFAULT_HEADER_HEIGHT,
  VIEW_DEFAULT_ROOT_HEADER_HEIGHT,
} from "@/ui/view";
import { assertNever } from "@/utils/functools";
import HistoryNavigator from "@/views/builtins/HistoryNavigator.vue";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import InlineHeader from "@/views/builtins/InlineHeader.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { FocusAnchor, NavigationDirection, type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { getViewComponent } from "@/views/registry";
import { MaybeElement, useElementSize, useKeyModifier } from "@vueuse/core";
import { computed, ref, Ref, shallowRef, toRef } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const COMMAND_HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const ROW_HEIGHT_MIN = 32;
const ROW_HEIGHT_MAX = 200;
const ROW_COMMANDS_WIDTH = 50;
const GUTTER_WIDTH = 60;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    paddingX?: number;
    paddingY?: number;
    containerGutterWidth?: number;
    isRoot?: boolean;
  } & Partial<Pick<ViewData, "focus" | "icon" | "nodePtr" | "isInput" | "isInline" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);
const isSelected = computed(() => state.isSelected(databasePtr.value));

// NOTE :UX :Architecture: Database view should be factored out into general Table/Feed/List/etc. query/collection views (?)

const databasePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.DATABASE>);
const preparedConnection = useAutoConnection(databasePtr);
const { graph: graph, graphRaw: graphRaw, connection: connection } = preparedConnection;
const database = graph.getRef(databasePtr, { ignoreAncestors: true });
const databaseRaw = graphRaw.getRef(databasePtr);
const fields = graph.getChildrenRef(database, NodeType.FIELD);
const fieldsRaw = graphRaw.getChildrenRef(databaseRaw, NodeType.FIELD);

//
// Search/filter
//

const DEFAULT_SORT = makeExpression({
  type: ExpressionType.ASCENDING,
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
    const sort = makeExpression({ type, blockPtr: databasePtr.value, fieldPtr: toNodeRef(column.field) });
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
const limit = computed(() => (props.isMinimal ? 10 : 30));
const {
  graph: recordGraph,
  roots: records,
  connection: recordConnection,
  isStale,
  isConnected,
  isConnecting,
  page,
} = useSearchConnection(
  { name: "table.records", live: true },
  computed(
    (): SearchConnectionParams<NodeType.RECORD> => ({
      scope: PACKAGE_SCOPE.value,
      nodeType: NodeType.RECORD,
      first: limit.value,
      count: true,
      baseTypePtr: databasePtr.value,
      isEnabled: databasePtr.value != null && databaseRaw.value != null,
      sort: sorts.value.length > 0 ? sorts.value : [DEFAULT_SORT],
      filter: makeAndConditional(filters.value),
      select: { metatype: ObjectType.SELECT_OPTIONS, selectFieldsPtr: fieldsRaw.value.map(toNodeRef) },
    }),
  ),
);

//
// State
//

const historyRef: Ref<InstanceType<typeof HistoryNavigator> | null> = ref(null);
const headerRef: Ref<InstanceType<typeof InlineHeader> | null> = ref(null);
const containerRef = ref<HTMLDivElement | null>(null);
const columnHeaderRef: Ref<HTMLDivElement | null> = ref(null);
const bodyRef: Ref<HTMLDivElement | null> = ref(null);
const columnHeaderRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const cellWrapperRefs: Ref<Record<string, HTMLElement | null>> = ref({});
const cellComponentRefs: Ref<Record<string, ViewExpose>> = ref({});
const shiftKey = useKeyModifier("Shift");

const headerSize = useElementSize(headerRef as Ref<MaybeElement>);
const containerSize = useElementSize(containerRef as Ref<MaybeElement>);
const rowWidth = computed(() => {
  if (props.isMinimal) {
    return containerSize.width.value; // row commands are floating to the left
  } else {
    return containerSize.width.value - GUTTER_WIDTH * 2;
  }
});
const bodySize = computed(() => {
  if (props.isMinimal) {
    return {
      width: containerSize.width.value + (props.containerGutterWidth ?? 0) * 2,
      height: containerSize.height.value - COMMAND_HEADER_HEIGHT,
    };
  } else {
    return {
      width: rowWidth.value,
      height: containerSize.height.value - COMMAND_HEADER_HEIGHT,
    };
  }
});

function getCellId(record: RecordData, column: ColumnView) {
  return `${record.id}.${column.id}`;
}

// NOTE :UX :Architecture: add records optimistically in Database?
//  (right now, they only show up once committed in the backend and the search connection is updated from there)
function createRecord() {
  if (database.value == null) throw new Error("no block to add record to");
  const databasePtr = toNodeRef(database.value);
  const record = recordConnection.tx.create({
    metatype: NodeType.RECORD,
    parentPtr: databasePtr,
    packagePtr: database.value.packagePtr,
    databasePtr,
    valuePacked: {},
  });
  canvas.inspect({ node: record, view: containerRef.value });
  return record;
}

function getColumnMinWidth(type: TypeIdentity, viewType: ViewType | undefined) {
  if (type.primitiveType == PrimitiveType.BOOLEAN) {
    return 100;
  } else if (
    type.primitiveType == PrimitiveType.DATE ||
    type.primitiveType == PrimitiveType.DATETIME ||
    type.primitiveType == PrimitiveType.TIME ||
    type.primitiveType == PrimitiveType.DURATION
  ) {
    return 150;
  } else {
    return 200;
  }
}

function getColumnPadding(
  type: TypeIdentity,
  viewType: ViewType | undefined,
): { paddingTop: number; paddingBottom: number } {
  // calibrated against ROW_HEIGHT to ensure all types look center-aligned at the default height
  if (viewType == ViewType.TOGGLE) {
    return { paddingTop: 5, paddingBottom: 0 };
  } else if (viewType == ViewType.TEXT || viewType == ViewType.FILE) {
    return { paddingTop: 3, paddingBottom: 2 };
  } else if (viewType == ViewType.NUMBER || viewType == ViewType.STRING) {
    return { paddingTop: 6, paddingBottom: 2 };
  } else {
    return { paddingTop: 6, paddingBottom: 2 };
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
  options: TransactionOptions;
  isInput: boolean;
  isInspected: boolean;
  isHighlighted: boolean;
  isSelected: boolean;
  isName: boolean;
  paddingTop: number;
  paddingBottom: number;
} & ColumnContent;
const columns: Ref<ColumnView[]> = computed(() => {
  // NOTE :UX: support Table property columns properly :RichColumns
  const columns: ColumnView[] = [];

  function column(
    columnIn: Pick<
      ColumnView,
      "id" | "icon" | "title" | "type" | "isInput" | "isHighlighted" | "isInspected" | "isName"
    > &
      ColumnContent,
  ) {
    const view = getViewForType(columnIn.type, { forcePickerDropdown: true });
    const padding = getColumnPadding(columnIn.type, view?.type);
    const column: ColumnView = {
      ...columnIn,
      idx: columns.length,
      viewType: view?.type,
      viewComponent: view?.type != null ? getViewComponent(view?.type) : null,
      viewProps: { ...view, isInput: columnIn.isInput },
      width: getColumnMinWidth(columnIn.type, view?.type),
      options: getTransactionOptionsForType(columnIn.type),
      paddingTop: padding.paddingTop,
      paddingBottom: padding.paddingBottom,
      isSelected: false,
    };
    columns.push(column);
  }

  for (const propertyId of [RecordProperty.name] as RecordProperty[]) {
    // :RichColumns
    const property = propertyInfo(NodeType.RECORD, propertyId);
    const propertyType = getPropertyType(property);
    column({
      id: propertyId.toString(),
      icon: getTypeIcon(propertyType),
      title: getPropertyTitle(property),
      type: propertyType,
      kind: "property",
      property,
      propertyName: getPropertyName(property),
      isInput: true,
      isInspected: false,
      isHighlighted: false,
      isName: propertyId == RecordProperty.name,
    });
  }
  for (const field of fields.value) {
    column({
      kind: "field",
      id: field.id,
      title: field.name ?? "Field",
      icon: getNodeIcon(field),
      type: field,
      field,
      storageKey: getStorageKey(field),
      isInput: true,
      isHighlighted: canvas.isHighlighted(field),
      isInspected: canvas.isInspected(field),
      isName: false,
    });
  }

  // grow columns to fit container (if possible)
  const totalWidth = columns.reduce((acc, column) => acc + column.width, 0);
  const availableWidth = rowWidth.value;
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
    return unpackValue(valuePacked, column.type);
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
  if (options?.debounce == null) options = { ...(options ?? {}), ...column.options };
  if (column.kind == "property") {
    tx.update(record, { [column.propertyName]: newValue }, options);
  } else if (column.kind == "field") {
    const oldValue = readColumnValue(record, column);
    const newValuePacked = packValue(newValue, column.type);
    const oldValuePacked = packValue(oldValue, column.type);
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

const selectionOverlayContainerRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionOverlayBodyRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZoneContainer = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayContainerRef });
const selectionZoneBody = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayBodyRef });
const selectedRecordsById: Ref<Record<string, RecordData>> = computed(() => {
  const selectedRecords = new Set(canvas.selection?.nodesPtr?.map((ptr) => ptr.id));
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
const selectedRecords = computed(() => Object.values(selectedRecordsById.value));
const numSelectedRows = computed(() => selectedRecords.value.length);
const isAllSelectedRows = computed(() => numSelectedRows.value >= records.value.length);
const lastSelectedRow: Ref<RecordData | null> = ref(null);

function isSelectedRow(record: RecordData) {
  return selectedRecordsById.value[record.id] != null;
}

function isSelectedCell(record: RecordData, column: ColumnView) {
  return isSelectedRow(record);
}

function addSelectionRow(record: RecordData) {
  canvas.select(expandSelection(canvas.selection, [record]));
}

function removeSelectionRow(record: RecordData) {
  if (canvas.selection == null) return;
  canvas.select(collapseSelection(canvas.selection, [record]));
}

function setSelectionRow(record: RecordData, selected: boolean, expandFromLast: boolean) {
  if (selected) {
    const lastSelectedY = records.value.findIndex((r) => r.id == lastSelectedRow.value?.id);
    const currentY = records.value.findIndex((r) => r.id == record.id);
    if (expandFromLast && lastSelectedY >= 0 && currentY >= 0) {
      const from = Math.min(lastSelectedY, currentY);
      const to = Math.max(lastSelectedY, currentY);
      const newSelection = records.value.slice(from, to + 1);
      state.select(expandSelection(canvas.selection, newSelection));
    } else {
      addSelectionRow(record);
    }
    lastSelectedRow.value = record;
  } else {
    removeSelectionRow(record);
  }
}

//
// Drag & drop :TypeDragAndDrop
//

function allowDrop(dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event?: DragEvent) {
  if (dragged.kind != "node" && dragged.kind != "selection") return false;
  return dragged.nodes.every((node) => {
    node = graph.getOrError(node);
    return isNode(node, NodeType.FIELD);
  });
}
function onDrop(dragged: DragContent, anchor: MultiAnchor, targetId: string | null, event: DragEvent) {
  if (dragged.kind != "node" && dragged.kind != "selection") return;
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
  for (let node of dragged.nodes) {
    node = graph.getOrError(node);
    const target = targetId != null ? graph.get({ id: targetId }) : null;
    if (isNode(node, NodeType.FIELD)) {
      // move field
      if (target != null) {
        if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
        moveNode(tx, graph, node, { anchor, target });
      } else {
        moveNode(tx, graph, node, { anchor: "center", target: database.value! });
      }
    } else {
      // add field with block type
      const type = nodeToType(node, "instance");
      const fieldIn = { ...type, zone: FieldType.MEMBER };
      if (target != null) {
        if (!isNode(target, NodeType.FIELD)) throw new Error(`unexpected target node: ${describeNode(target)}`);
        createField(tx, graph, { field: fieldIn, anchor, target });
      } else {
        createField(tx, graph, { field: fieldIn, anchor: "inside", target: database.value! });
      }
    }
  }
}
const { activeDropZone: activeHeaderDropZone } = useMultiDropZone({
  name: "table.header",
  container: columnHeaderRef,
  targetsInOrder: computed(() => columns.value.map((column) => column.id)),
  targetsById: columnHeaderRefs,
  orientation: Orientation.HORIZONTAL,
  kinds: ["node", "selection"],
  metatypes: [NodeType.BLOCK, NodeType.FIELD],
  fallbackToClosest: true,
  allowDrop,
  onDrop,
});

//
// Navigation
//

function navigateFromHeader(direction: NavigationDirection) {
  // NOTE :UX: navigate to records for Database
  emit("navigate", direction);
}

//
// Commands
//

const commands: Partial<CommandMapKit<"space" | "table" | "list">> = {
  // edit
  "space.edit.rename": () => {
    headerRef.value?.focusIdentifier("left");
  },
  ...useNodeTableCommands({
    nodeType: NodeType.RECORD,
    self: state.baseViewRef,
    graph: graph,
    list: records,
    create: () => createRecord(),
    txFactory: () => connection.tx,
  }),
  // table
  "table.column.sortAscending": (command, ctx) => {
    const node = ctx.nodes?.[0];
    if (!isNode(node, NodeType.FIELD)) return false;
    const column = columns.value.find((column) => column.kind == "field" && column.field == node);
    if (column == null) return false;
    addSort(column, ExpressionType.ASCENDING);
  },
  "table.column.sortDescending": (command, ctx) => {
    const node = ctx.nodes?.[0];
    if (!isNode(node, NodeType.FIELD)) return false;
    const column = columns.value.find((column) => column.kind == "field" && column.field == node);
    if (column == null) return false;
    addSort(column, ExpressionType.DESCENDING);
  },
};

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  headerRef.value?.focus?.(anchor ?? "left");
}

defineExpose<ViewExpose>({ self, id, commands: commands, focus });
</script>
<template>
  <div
    ref="containerRef"
    class="h-full select-none"
    :style="{
      marginBottom: isMinimal ? '0' : `${GUTTER_WIDTH}px`,
    }"
    @mousedown="(e: MouseEvent) => startSelectingIfAllowed(selectionZoneContainer, e)"
  >
    <!-- Header -->
    <InlineHeader
      ref="headerRef"
      :self="self"
      :node="database"
      :node-ptr="databasePtr"
      :prepared-connection="preparedConnection"
      :is-root="isRoot"
      :is-inline="isInline"
      :is-minimal="isMinimal"
      :focus="props.focus"
      :width="rowWidth"
      @navigate="(direction: NavigationDirection) => navigateFromHeader(direction)"
    >
      <template #left="{ style }">
        <!-- Expressions (filters/sorts) -->
        <!-- TODO :UX: Incomplete: filter/sort Database/Table view properly -->
        <!-- (this should of course be Expression views) -->
        <template v-if="style == 'page'">
          <div
            v-for="(sort, i) in sorts.length > 0 ? sorts : [DEFAULT_SORT]"
            :key="i"
            class="group rounded-full px-2 py-0.5 hover:bg-gray-100"
          >
            <span class="text-gray-900">
              {{ findColumn(sort)?.title ?? getPropertyTitle(DEFAULT_SORT.propertyPtr!) }}
            </span>
            <button
              class="ml-1.5 text-gray-400 opacity-0 transition-colors duration-150 group-hover:opacity-100"
              @click="() => sorts.splice(i, 1)"
            >
              <i class="fas fa-xmark" />
            </button>
          </div>
        </template>
      </template>

      <!-- Meta -->
      <template #right="{ style }">
        <!-- Selection -->
        <div
          class="flex flex-row items-center rounded border transition-colors duration-150"
          :class="numSelectedRows > 0 ? 'opacity-100' : 'pointer-events-none opacity-0'"
          data-suppress-drag="both"
        >
          <button class="h-full px-2 py-0.5 font-medium hover:bg-gray-100" @click="state.deselect()">
            {{ numSelectedRows }} selected
          </button>
          <button
            v-tooltip="{ title: 'Duplicate', small: true }"
            class="w-8 border-x py-0.5 text-gray-700 hover:bg-gray-100"
            :disabled="numSelectedRows == 0"
            @click.stop="fireCommandById('space.edit.duplicate', { nodes: selectedRecords })"
          >
            <i class="fas fa-clone" />
          </button>
          <button
            v-tooltip="{ title: 'Delete', small: true }"
            class="w-8 py-0.5 text-gray-700 hover:bg-gray-100"
            :disabled="numSelectedRows == 0"
            @click.stop="fireCommandById('space.edit.delete', { nodes: selectedRecords })"
          >
            <i class="fas fa-trash" />
          </button>
        </div>

        <!-- Status -->
        <div v-if="recordConnection.isConnecting.value" class="">
          <!-- Loading -->
          <i class="fas fa-spinner-third animate-spin text-gray-400" />
        </div>
        <!-- Add field -->
        <button
          class="group/button rounded px-1 py-0.5 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
          :class="style == 'block' ? 'opacity-0 group-hover/block:opacity-100 group-hover/header:opacity-100' : ''"
          @click="
            (e: MouseEvent) => {
              const button = (e.target as HTMLElement).closest('button')!;
              pushPopover({
                kind: 'view',
                trigger: button,
                reference: button,
                component: ViewType.PICKER,
                title: 'Add Field',
                placement: 'bottom-left',
                offset: 'referenceWidth',
                props: {
                  valueType: makeType({ benchType: BenchType.TYPE }),
                  subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, {
                    variant: PickerVariant.DROPDOWN_LARGE,
                  }),
                },
                onApply: (typeInfo: TypeIdentity) => {
                  createField(connection.tx, graph, { anchor: 'inside', target: database!, field: typeInfo });
                },
              });
            }
          "
        >
          <i class="fas fa-plus mr-1.5 text-center" />
          <span>Field</span>
        </button>
        <!-- Add record -->
        <button
          v-if="style == 'page'"
          class="group/button rounded px-1 py-0.5 text-gray-400 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-700"
          :class="isMinimal ? 'text-gray-400 hover:text-gray-700' : ''"
          @click="() => createRecord()"
        >
          <i class="fas fa-plus mr-1.5 text-center" />
          <span class="">Record</span>
        </button>
      </template>
    </InlineHeader>

    <!-- Body outer wrapper (scroll horizontally, and vertically if not compact) -->
    <Scroll
      v-if="database"
      id="scroll"
      :size="bodySize"
      :orientation="isMinimal ? Orientation.HORIZONTAL : undefined"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
      :style="{
        marginLeft: containerGutterWidth != null ? `${-containerGutterWidth}px` : undefined,
      }"
      @mousedown="(e: MouseEvent) => startSelectingIfAllowed(selectionZoneBody, e)"
    >
      <!-- Body inner wrapper -->
      <div
        ref="bodyRef"
        :style="{
          paddingLeft: !isMinimal
            ? `${GUTTER_WIDTH + (containerGutterWidth ?? 0) - ROW_COMMANDS_WIDTH}px`
            : `${(containerGutterWidth ?? 0) - ROW_COMMANDS_WIDTH}px`,
          paddingRight: !isMinimal
            ? `${GUTTER_WIDTH + (containerGutterWidth ?? 0)}px`
            : `${containerGutterWidth ?? 0}px`,
          minHeight: !isMinimal
            ? `${bodySize.height - headerSize.height.value - (isRoot ? VIEW_DEFAULT_ROOT_HEADER_HEIGHT : 0) - COMMAND_HEADER_HEIGHT - 30}px`
            : undefined,
        }"
      >
        <!-- Column headers (sticky) -->
        <div
          ref="columnHeaderRef"
          class="group/header z-20 flex flex-row items-center border-gray-200"
          :class="[!isMinimal ? 'sticky top-0' : '']"
          :style="{
            height: `${ROW_HEIGHT_MIN}px`,
          }"
        >
          <!-- Composite commands -->
          <div
            class="sticky left-0 z-30 flex flex-shrink-0 flex-row items-center justify-end px-1.5 transition-colors duration-150"
            :class="[selectedRecords.length > 0 ? 'bg-white' : 'bg-transparent']"
            :style="{
              width: `${ROW_COMMANDS_WIDTH}px`,
              height: `${ROW_HEIGHT_MIN}px`,
            }"
            data-suppress-drag="both"
          >
            <!-- Selection checkbox -->
            <button
              class="flex h-4 w-4 items-center rounded border border-gray-200 bg-white px-[1px] transition-colors duration-150"
              :class="selectedRecords.length > 0 ? 'opacity-100' : 'opacity-0 group-hover/header:opacity-100'"
              @click="() => (isAllSelectedRows ? state.deselect() : state.select(records))"
            >
              <span
                class="inline-block h-3 w-3 rounded transition-colors duration-75"
                :class="isAllSelectedRows ? 'bg-gray-700' : ''"
              />
            </button>
          </div>

          <!-- Column header -->
          <div
            v-for="(column, x) in columns"
            :key="column.id"
            :ref="
              (ref: any) => (ref != null ? (columnHeaderRefs[column.id] = ref) : delete columnHeaderRefs[column.id])
            "
            class="relative flex h-full flex-shrink-0 cursor-pointer items-center border-b border-gray-200 border-l-transparent px-2 transition-colors duration-150 data-[dragging=true]:opacity-50"
            :class="[
              x > 0 ? 'border-l' : '',
              isSelected
                ? 'bg-transparent'
                : column.isInspected || column.isHighlighted
                  ? 'bg-gray-100'
                  : 'bg-white hover:bg-gray-100',
            ]"
            :style="{
              paddingLeft: x == 0 && paddingX != null ? `${paddingX}px` : undefined,
              paddingRight: x == columns.length - 1 && paddingX != null ? `${paddingX}px` : undefined,
              width: `${column.width}px`,
            }"
            data-contextmenu-items="table.column.*"
            :data-node-type="column.kind == 'field' ? column.field.metatype : undefined"
            :data-node-id="column.kind == 'field' ? column.field.id : undefined"
            :data-node-ck="column.kind == 'field' ? column.field.ck : undefined"
            data-suppress-drag="select"
            :draggable="column.kind == 'field'"
            @dragstart.stop="(e: DragEvent) => column.kind == 'field' && startDraggingIfAllowed(e, column.field)"
            @mousedown="
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
              class="absolute z-10 h-full w-1 rounded bg-gray-700"
              :class="[activeHeaderDropZone?.anchor == 'start' ? (x == 0 ? 'left-0' : '-left-[3px]') : '-right-[3px]']"
            />
            <!-- Icon/Name -->
            <IconInline
              v-menu="
                (): PopoverInfoIn => ({
                  kind: 'view',
                  component: Icon,
                  isEnabled: column.kind == 'field',
                  placement: 'bottom-right',
                  offset: '-referenceWidth',
                  props: { modelValue: column.kind == 'field' ? column.field.icon : undefined },
                  onApply: (newIcon) => {
                    if (column.kind != 'field') return;
                    connection.tx.update(column.field, { icon: newIcon });
                  },
                })
              "
              class="mr-1.5 w-5 rounded p-0.5 text-center text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
              v-bind="column.icon"
            />
            <NativeInput
              v-if="column.kind == 'field'"
              :id="column.id + '.name'"
              class="truncate font-medium"
              placeholder="Name..."
              :model-value="column.title"
              :value-type="NAME_TYPE"
              is-input
              is-minimal
              @update:model-value="
                (newValue) => connection.tx.update(column.field, { name: newValue as string }, { debounce: 'long' })
              "
            />
            <span v-else class="truncate font-medium">{{ column.title }}</span>
          </div>
          <!-- Empty columns -->
          <button
            v-if="columns.length == 0"
            class="flex w-full flex-row items-center justify-center border-b text-gray-400 hover:bg-gray-100"
            :style="{
              height: `${ROW_HEIGHT_MIN}px`,
              width: isMinimal ? undefined : `calc(100% - ${ROW_COMMANDS_WIDTH}px)`,
            }"
            @click="
              createField(connection.tx, graph, {
                anchor: 'inside',
                target: database!,
                field: { type: FieldType.MEMBER, kind: TypeKind.STRUCT, benchType: BenchType.TEXT, name: 'Text' },
              })
            "
          >
            No columns. Click to add.
          </button>
        </div>

        <!-- Row -->
        <div
          v-for="(record, y) in records"
          :key="record.id"
          class="group/row flex flex-row border-gray-200"
          :data-node-id="record.id"
          :data-node-ck="record.id"
          :data-node-type="record.metatype"
        >
          <!-- Row commands -->
          <div
            class="sticky left-0 z-10 flex flex-shrink-0 flex-row items-center justify-end gap-x-1 px-1.5 transition-colors duration-150"
            :class="[selectedRecords.length > 0 ? 'bg-white' : 'bg-transparent']"
            :style="{
              width: `${ROW_COMMANDS_WIDTH}px`,
              height: `${ROW_HEIGHT_MIN}px`,
            }"
            data-suppress-drag="both"
          >
            <!-- Controls -->
            <button
              class="rounded text-gray-400 opacity-0 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700 group-hover/row:opacity-100"
              @click="(e) => (setSelectionRow(record, true, false), pushDefaultMenu('main', record, e))"
            >
              <i class="fas fa-ellipsis-vertical w-5 text-center" />
            </button>
            <!-- Selection checkbox -->
            <button
              class="flex h-4 w-4 items-center rounded border border-gray-200 bg-white px-[1px] transition-colors duration-150"
              :class="selectedRecords.length > 0 ? 'opacity-100' : 'opacity-0 group-hover/row:opacity-100'"
              @click="() => setSelectionRow(record, !isSelectedRow(record), shiftKey ?? false)"
            >
              <span
                class="inline-block h-3 w-3 rounded transition-colors duration-75"
                :class="selectedRecords.length > 0 && isSelectedRow(record) ? 'bg-gray-700' : ''"
              />
            </button>
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
            class="flex-shrink-0 cursor-pointer overflow-hidden border-b border-gray-200 text-gray-900 transition-colors duration-150"
            :class="[
              x > 0 ? 'border-l' : '',
              isSelectedCell(record, column) ? 'bg-orange-400/20' : canvas.isInspected(record) ? 'bg-gray-100' : '',
              column.isName && record.icon != null ? 'flex flex-row items-center gap-x-1.5 px-2' : 'px-2',
            ]"
            :style="{
              width: `${column.width}px`,
              minHeight: `${ROW_HEIGHT_MIN}px`,
              maxHeight: `${ROW_HEIGHT_MAX}px`,
              paddingTop: `${column.paddingTop}px`,
              paddingBottom: `${column.paddingBottom + (y == records.length - 1 ? (props.paddingY ?? 0) : 0)}px`,
              paddingLeft: x == 0 && paddingX != null ? `${paddingX}px` : undefined,
              paddingRight: x == columns.length - 1 && paddingX != null ? `${paddingX}px` : undefined,
            }"
            :data-column-id="column.id /* used to mark this as a column for click handler below */"
            @mousedown="
              (event) => {
                // interact with / focus cell component
                const componentEl = cellComponentRefs[getCellId(record, column)];
                if (componentEl != null) {
                  if (componentEl?.interact != null) componentEl.interact();
                  else focusInElement(componentEl as unknown as MaybeElement);
                }
              }
            "
          >
            <!-- Inline title icon -->
            <IconInline
              v-if="column.isName && record.icon != null"
              ref="iconRef"
              v-tooltip="{ small: true, text: `Change icon` } as TooltipInfo"
              v-menu="
                (): PopoverInfoIn => ({
                  kind: 'view',
                  component: Icon,
                  placement: 'bottom-right',
                  offset: '-referenceWidth',
                  props: { modelValue: (record as any)!.icon, isInput: true },
                  isEnabled: isInput,
                  onApply: (newIcon) => recordConnection.tx.update(record, { icon: newIcon }),
                })
              "
              v-bind="getNodeIcon(record)"
              class="w-5 rounded py-0.5 text-center transition-colors duration-75 hover:bg-gray-100"
              :class="record.icon != null ? 'text-gray-700 hover:text-gray-900' : 'text-gray-400 hover:text-gray-500'"
            />
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
              class="flex-1 cursor-pointer select-none"
              :model-value="readColumnValue(record, column)"
              is-minimal
              is-small
              :size="{ width: column.width, height: ROW_HEIGHT_MAX }"
              :parent="database"
              @update:model-value="(value: any) => writeColumnValue(record, column, value)"
              @mousedown.stop
            />
            <!-- No view available (internal bug / missing feature) -->
            <span v-else class="text-danger-600">
              {{ column.viewType != null ? ViewType[column.viewType] : "No View" }}
            </span>
          </div>
          <!-- No columns -->
          <div
            v-if="columns.length == 0"
            class="w-full border-b border-gray-200 text-center text-gray-400 hover:bg-gray-100"
            :style="{ height: `${ROW_HEIGHT_MIN}px` }"
          />
        </div>

        <!-- Footer -->
        <div
          class="flex flex-row items-center justify-center text-center"
          :style="{ paddingLeft: `${ROW_COMMANDS_WIDTH}px`, height: `${ROW_HEIGHT_MIN}px` }"
        >
          <button class="h-full w-full px-3 text-left text-gray-400 hover:bg-gray-100" @click="createRecord()">
            <i class="fas fa-plus mr-1.5" />
            <span class="">Record</span>
          </button>
        </div>

        <!-- Selection -->
        <SelectionOverlay ref="selectionOverlayBodyRef" :zone="selectionZoneBody" />
      </div>
    </Scroll>

    <!-- Selection -->
    <SelectionOverlay ref="selectionOverlayContainerRef" :zone="selectionZoneContainer" />
    <Inaccessible v-if="!database" :connection="connection" :node="databasePtr" class="h-full w-full" />
  </div>
</template>
