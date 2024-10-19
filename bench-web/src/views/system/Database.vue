<script lang="ts" setup>
import { blockToType } from "@/language/block";
import { getPropertyName, getPropertyTitle, TYPE_BLOCK_TYPES } from "@/language/const";
import {
  createField,
  getPropertyType,
  getStorageKey,
  makeTypeInfo,
  NAME_TYPE,
  resolveType,
  TypeIdentity,
} from "@/language/field";
import { cloneNode, moveNode } from "@/language/node";
import { DebounceLevel, newChangeId } from "@/language/transaction";
import { packValue, unpackValue } from "@/language/value";
import {
  BenchType,
  EditOperationData,
  EditOperationType,
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
  SelectionType,
  Timestamp,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import {
  describeNode,
  isNode,
  propertyInfo,
  toPlainNodeRef,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { SearchConnectionParams, useExistingConnection, useSearchConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { ActionContext, ActionMapImplementation } from "@/ui/action";
import { DraggedContent, MultiAnchor, startDraggingIfAllowed, useMultiDropZone } from "@/ui/drag";
import { getNodeIcon, getTypeIcon, IconInline } from "@/ui/icon";
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
import { makeViewId, ViewComponent, viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { getViewComponent } from "@/views/registry";
import { MaybeElement, useElementSize } from "@vueuse/core";
import { computed, ref, Ref, toRef } from "vue";

const ACTION_HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const ROW_HEIGHT_MIN = 32;
const ROW_HEIGHT_MAX = 200;
const ROW_ACTIONS_WIDTH = 32;
const ROW_PADDING_X = 8; // per side, so ROW_PADDING*2 per side
const POPOVER_HEIGHT_MIN = 32;
const POPOVER_HEIGHT_MAX = 400;
const POPOVER_WIDTH_MIN = 32;
const POPOVER_WIDTH_MAX = 400;
const FULL_PADDING = 12;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "variant" | "isInput" | "selection">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const selfView = spaceGraph.getRef(self);

// NOTE :UX: Database view should be factored out into Table/Feed/etc. query views (?)

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const preparedPkgConnection = useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: true });
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);

//
// Search/filter
//

// nocheckin: wait with search until Database is actually committed? (else optimistic commit makes us query too early and errors)
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
  {
    name: "database.records",
    live: true,
  },
  computed(
    (): SearchConnectionParams<NodeType.RECORD> => ({
      scope: PACKAGE_SCOPE.value,
      nodeType: NodeType.RECORD,
      first: limit.value,
      count: true,
      blockPtr: nodePtr.value,
      isEnabled: nodePtr.value != null,
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
const cellComponentRefs: Ref<Record<string, MaybeElement>> = ref({});

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

function getMinColumnWidth(type: TypeIdentity, viewType: ViewType | undefined) {
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
  isPopover: boolean;
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
      width: getMinColumnWidth(columnIn.type, view?.type),
      debounce: getColumnDebounce(columnIn.type, view?.type),
      paddingTop: padding.paddingTop,
      paddingBottom: padding.paddingBottom,
      isPopover: false, // not sure which views/cells should open as popover
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
    const fieldType = resolveType(field, pkgGraph);
    addColumn({
      kind: "field",
      id: field.id,
      title: field.name,
      icon: getNodeIcon(field),
      type: fieldType,
      field,
      storageKey: getStorageKey(field, fieldType),
      isInput: true,
      isHighlighted: canvas.isHighlighted(field),
      isInspected: canvas.isInspected(field),
      isSelected: isSelectedField(field),
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
  options?: { debounce?: DebounceLevel },
) {
  if (options?.debounce == null) options = { ...(options ?? {}), debounce: column.debounce };
  if (column.kind == "property") {
    recordConnection.tx.update(record, { [column.propertyName]: newValue }, options);
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
    recordConnection.tx.update(record, operations, options);
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
  const selectedFields = new Set(props.selection?.nodesPtr?.map((ptr) => ptr.ck));
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
const hasSelection = computed(() => Object.keys(selectedRecordsById.value).length > 0);
const numSelectedRecords = computed(() => Object.keys(selectedRecordsById.value).length);
const isAllSelected = computed(() => numSelectedRecords.value >= records.value.length);

function isSelectedRecord(record: RecordData) {
  return selectedRecordsById.value[record.id] != null;
}

function isSelectedField(field: FieldData) {
  return selectedFieldsByCk.value[field.ck] != null;
}

function isSelectedCell(record: RecordData, column: ColumnView) {
  if (!isSelectedRecord(record)) return false;
  if ((props.selection?.fieldsPtr?.length ?? 0) == 0) return true;
  return column.kind == "field" && isSelectedField(column.field);
}

function addSelectionRecord(record: RecordData) {
  if (selfView.value == null) throw new Error("no self view");
  spaceConnection.tx.update(
    selfView.value,
    { selection: expandSelection(props.selection, [record]) },
    { debounce: "tick" },
  );
}

function removeSelectionRecord(record: RecordData) {
  if (selfView.value == null || props.selection == null) throw new Error("no self view");
  spaceConnection.tx.update(
    selfView.value,
    { selection: collapseSelection(props.selection, [record]) },
    { debounce: "tick" },
  );
}

function setSelectionRecord(record: RecordData, selected: boolean) {
  if (selected) {
    addSelectionRecord(record);
  } else {
    removeSelectionRecord(record);
  }
}

function selectAll() {
  const selection = {
    metatype: ObjectType.SELECTION,
    type: SelectionType.LIST,
    nodesPtr: records.value.map(toPlainNodeRef),
    fieldsPtr: [],
  };
  spaceConnection.tx.update(selfView.value!, { selection }, { debounce: "tick" });
}

function selectNone() {
  spaceConnection.tx.update(selfView.value!, { selection: undefined }, { debounce: "tick" });
}

// nocheckin: field selection

// working with selection

function duplicateSelection(): RecordData[] {
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
  const tx = recordConnection.tx.with({ change: { key: newChangeId(), title: "Delete records" } });
  for (const record of Object.values(selectedRecordsById.value)) {
    tx.delete(record);
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
  "common.create.record": () => createRecord(),
  "common.edit.delete": (action, ctx) => {
    if (hasSelection.value) {
      deleteSelection();
    } else {
      const { node } = getNodeFromContext(ctx);
      if (node != null) {
        pkgConnection.tx.delete(node);
      }
    }
  },
  "common.edit.duplicate": (action, ctx) => {
    if (hasSelection.value) {
      duplicateSelection();
    } else {
      const { node } = getNodeFromContext(ctx);
      if (node != null) {
        const tx = pkgConnection.tx.with({ change: { key: newChangeId(), title: "Duplicate record" } });
        cloneNode(tx, recordGraph, node);
      }
    }
  },
};

canvas.registerView(self, id);
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
      }"
    >
      <!-- Expressions (filters/sorts) -->
      <!-- nocheckin :Incomplete: filter/sort Database -->
      <template v-if="true">
        <button class="group/button rounded px-1 hover:bg-gray-100">
          <i class="fa fa-plus mr-1.5 text-center text-gray-300 group-hover/button:text-primary-900" />
          <span class="text-gray-400 group-hover/button:text-primary-900">Filter</span>
        </button>
        <button class="group/button rounded px-1 hover:bg-gray-100">
          <i class="fa fa-plus mr-1.5 text-center text-gray-300 group-hover/button:text-primary-900" />
          <span class="text-gray-400 group-hover/button:text-primary-900">Sort</span>
        </button>
      </template>
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
        <div v-if="hasSelection" class="flex flex-row items-center rounded border">
          <span class="h-full px-2 py-0.5 font-medium text-primary-900">{{ numSelectedRecords }} selected</span>
          <button
            v-tooltip="{ title: 'Duplicate', small: true }"
            class="w-8 border-x py-0.5 text-gray-700 hover:bg-gray-100 hover:text-primary-900"
            @click="duplicateSelection"
          >
            <i class="fas fa-clone" />
          </button>
          <button
            v-tooltip="{ title: 'Delete', small: true }"
            class="w-8 py-0.5 text-gray-700 hover:bg-gray-100 hover:text-primary-900"
            @click="deleteSelection"
          >
            <i class="fas fa-trash-can" />
          </button>
        </div>

        <!-- Pagination -->
        <!-- nocheckin: pagination -->
        <div>
          <span v-if="page?.total != null" class="text-gray-400">{{ page.size }} / {{ page?.total }}</span>
        </div>

        <!-- Controls -->
        <!-- Add field -->
        <button
          class="group/button rounded px-1 hover:bg-gray-100 hover:text-primary-900 data-[popover=true]:bg-gray-100 data-[popover=true]:text-primary-900"
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
          <i class="fa fa-plus mr-1.5 text-center text-gray-700 group-hover/button:text-primary-900" />
          <span>Field</span>
        </button>
        <!-- Add record -->
        <button class="group/button rounded px-1 hover:bg-gray-100" @click="() => createRecord()">
          <i class="fa fa-plus mr-1.5 text-center text-gray-700 group-hover/button:text-primary-900" />
          <span class="group-hover/button:text-primary-900">Record</span>
        </button>
      </div>
    </div>

    <!-- Body (scroll horizontally, and vertically if not compact) -->
    <Scroll
      ref="bodyRef"
      :size="bodySize"
      :orientation="variant == Variant.COMPACT ? Orientation.HORIZONTAL : undefined"
      :track-width="ScrollbarWidth.md"
    >
      <div class="flex flex-col border-gray-200">
        <!-- Column headers (sticky) -->
        <div
          ref="headerRef"
          class="z-20 flex flex-row items-center border-gray-200"
          :class="[variant != Variant.COMPACT ? 'sticky top-0' : '']"
          :style="{
            height: `${ROW_HEIGHT_MIN}px`,
          }"
        >
          <!-- Composite actions -->
          <div
            v-if="variant != Variant.COMPACT"
            class="group sticky left-0 z-30 flex flex-shrink-0 flex-row items-center justify-center border-b transition-colors duration-150"
            :class="[hasSelection ? 'border-gray-200 bg-white' : 'border-transparent bg-transparent']"
            :style="{
              width: `${ROW_ACTIONS_WIDTH}px`,
              height: `${ROW_HEIGHT_MIN}px`,
            }"
          >
            <!-- Selection checkbox -->
            <input
              type="checkbox"
              class="h-4 w-4 rounded border-gray-200 text-gray-200 transition-colors duration-150 focus:ring-0"
              :class="[hasSelection ? 'opacity-100' : 'opacity-0 group-hover:opacity-100']"
              :checked="isAllSelected"
              @change="(e) => (!isAllSelected ? selectAll() : selectNone())"
            />
          </div>

          <!-- Column headers -->
          <div
            v-for="(column, i) in columns"
            :key="column.id"
            :ref="
              (ref: any) => (ref != null ? (columnHeaderRefs[column.id] = ref) : delete columnHeaderRefs[column.id])
            "
            v-contextmenu="
              (context: PopoverContext): PopoverInfoIn => {
                context = { ...context, triggerNode: column.kind == 'field' ? column.field : undefined };
                return {
                  kind: 'menu',
                  placement: 'bottom-right',
                  items: menuActionsLike(['common.edit.rename', 'common.edit.duplicate', 'common.edit.delete'], {
                    context,
                  }),
                  context,
                };
              }
            "
            class="relative flex h-full flex-shrink-0 cursor-pointer items-center border-b border-gray-200 border-l-transparent bg-white px-2 data-[dragging=true]:opacity-50"
            :class="[
              i > 0 ? 'border-l' : '',
              column.isInspected ? 'bg-primary-100' : column.isHighlighted ? 'bg-primary-50' : 'hover:bg-gray-100',
            ]"
            :style="{
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
              class="absolute z-10 h-full w-1 rounded-sm bg-primary-900"
              :class="[activeHeaderDropZone?.anchor == 'start' ? (i == 0 ? 'left-0' : '-left-[3px]') : '-right-[3px]']"
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
              class="mr-1.5 rounded p-0.5 text-gray-700 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
              v-bind="column.icon"
            />
            <NativeInput
              v-if="column.kind == 'field'"
              class="truncate font-medium"
              :model-value="column.title"
              :value-type="NAME_TYPE"
              :variant="Variant.STEALTH"
              is-input
              @update:model-value="
                (newValue) => pkgConnection.tx.update(column.field, { name: newValue }, { debounce: 'long' })
              "
            />
            <span v-else class="truncate font-medium">{{ column.title }}</span>
          </div>
        </div>
        <!-- Empty columns -->
        <div
          v-if="columns.length == 0"
          class="flex w-full flex-row items-center justify-center border-b text-gray-400 hover:bg-gray-100"
          :style="{ height: `${ROW_HEIGHT_MIN}px` }"
        >
          No columns.
        </div>

        <!-- Status (if not connected or empty) -->
        <div
          v-if="!isConnected"
          class="flex w-full flex-row items-center text-center justify-center"
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
          class="w-full text-center text-gray-400 hover:bg-gray-100 hover:text-primary-900"
          :style="{ height: `${ROW_HEIGHT_MIN}px` }"
          @click="createRecord"
        >
          <i class="fas fa-empty-set mr-1.5" />
          <span class="">No records. Click to add.</span>
        </button>

        <!-- Row -->
        <div
          v-for="(record, j) in records"
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
          class="group/row flex flex-row border-gray-200"
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
            :class="[hasSelection ? 'border-gray-200 bg-white' : 'border-transparent bg-transparent']"
            :style="{
              width: `${ROW_ACTIONS_WIDTH}px`,
            }"
          >
            <!-- Selection checkbox -->
            <input
              type="checkbox"
              class="h-4 w-4 rounded border-gray-200 text-gray-200 transition-colors duration-150 focus:ring-0"
              :class="[hasSelection ? 'opacity-100' : 'opacity-0 group-hover/row:opacity-100']"
              :checked="selectedRecordsById[record.id] != null"
              @change="(e) => setSelectionRecord(record, (e.target as HTMLInputElement).checked)"
            />
          </div>

          <!-- Cells -->
          <div
            v-for="(column, i) in columns"
            :ref="
              (ref: any) =>
                ref != null
                  ? (cellWrapperRefs[getCellId(record, column)] = ref)
                  : delete cellWrapperRefs[getCellId(record, column)]
            "
            class="flex-shrink-0 cursor-pointer overflow-hidden border-b border-gray-200 px-2 text-gray-900"
            :class="[i > 0 ? 'border-l' : '', isSelectedCell(record, column) ? 'bg-primary-100' : '']"
            :style="{
              width: `${column.width}px`,
              minHeight: `${ROW_HEIGHT_MIN}px`,
              maxHeight: `${ROW_HEIGHT_MAX}px`,
              paddingTop: `${column.paddingTop}px`,
              paddingBottom: `${column.paddingBottom}px`,
            }"
            :data-column-id="column.id /* used to mark this as a column for click handler below */"
            @click="
              (event) => {
                if (column.viewType == ViewType.TOGGLE) {
                  // just toggle it directly
                  const value = readColumnValue(record, column);
                  writeColumnValue(record, column, !value);
                } else if (column.isPopover) {
                  // open popover cell edit
                  const columnEl = (event.target as HTMLElement)?.closest('[data-column-id]')?.firstElementChild;
                  if (columnEl == null) return;
                  const width = Math.max(POPOVER_WIDTH_MIN, Math.min(POPOVER_WIDTH_MAX, column.width));
                  const height = Math.max(
                    POPOVER_HEIGHT_MIN,
                    Math.min(POPOVER_HEIGHT_MAX, columnEl?.getBoundingClientRect().height!),
                  );
                  const columnPos = (columnEl as HTMLElement).getBoundingClientRect();
                  const position = { x: columnPos.left - ROW_PADDING_X, y: columnPos.top - column.paddingTop };
                  // center inside cell if popover width is less than cell width
                  if (width < column.width) {
                    position.x += (column.width - width) / 2;
                  }
                  pushPopover({
                    trigger: columnEl as HTMLElement,
                    reference: position,
                    info: {
                      component: column.viewComponent,
                      placement: 'inside-top-left',
                      referenceMargin: 0,
                      props: {
                        ...column.viewProps,
                        isInput: true,
                        size: { metatype: ObjectType.BOX, width, height },
                        isInline: true,
                        modelValue: readColumnValue(record, column),
                      },
                      dontAnimate: true,
                      onUpdate: (value: any) => writeColumnValue(record, column, value),
                      onApply: (value: any) => writeColumnValue(record, column, value, { debounce: 'tick' }),
                    },
                  });
                } else {
                  // focus cell component
                  const componentEl = cellComponentRefs[getCellId(record, column)];
                  if (componentEl != null) focusInElement(componentEl);
                }
              }
            "
          >
            <!-- Inner column view -->
            <component
              :is="column.viewComponent"
              v-if="column.viewComponent != null"
              :ref="
                (ref: any) =>
                  ref != null
                    ? (cellComponentRefs[getCellId(record, column)] = ref)
                    : delete cellComponentRefs[getCellId(record, column)]
              "
              v-bind="column.viewProps"
              class="select-none"
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
            class="w-full border-b border-gray-200 text-center text-gray-400 hover:bg-gray-100 hover:text-primary-900"
            :style="{ height: `${ROW_HEIGHT_MIN}px` }"
          />
        </div>
      </div>
    </Scroll>
  </div>
</template>
