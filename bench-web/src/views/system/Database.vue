<script lang="ts" setup>
import { getPropertyName, getPropertyTitle } from "@/language/const";
import {
  createField,
  getPropertyType,
  getStorageKey,
  makeTypeInfo,
  NAME_TYPE,
  resolveType,
  TypeIdentity,
} from "@/language/field";
import { DebounceLevel } from "@/language/transaction";
import { packValue, unpackValue } from "@/language/value";
import {
  BenchType,
  EditOperationData,
  EditOperationType,
  FieldData,
  IconData,
  NodeType,
  ObjectType,
  Orientation,
  PrimitiveType,
  PropertyInfo,
  RecordData,
  RecordProperty,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { propertyInfo, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { SearchConnectionParams, useExistingConnection, useSearchConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { getNodeIcon, getTypeIcon, IconInline } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { PopoverInfoIn, pushPopover } from "@/ui/popover";
import { getViewForValueType, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { assertNever } from "@/utils/functools";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { getViewComponent } from "@/views/registry";
import { useElementSize } from "@vueuse/core";
import { computed, ref, Ref, toRef } from "vue";

const ACTION_HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const ROW_HEIGHT = 32;
const MIN_COLUMN_WIDTH = 50;
const ROW_ACTIONS_WIDTH = 32;
const ROW_PADDING_X = 8; // per side, so ROW_PADDING*2 per side
const ROW_PADDING_Y = 4; // per side, so ROW_PADDING*2 per side
const FULL_PADDING = 12;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "variant" | "isInput" | "selection">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

// NOTE :UX: Database view should be factored out into Table/Feed/etc. query views (?)

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const preparedPkgConnection = useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: true });
const fields = pkgGraph.getChildrenRef(block, NodeType.FIELD);

//
// Search/filter
//

const limit = computed(() => (props.variant == Variant.COMPACT ? 10 : 40));
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
// Interaction
//

// nocheckin: add records optimistically (?)
function addRecord() {
  if (nodePtr.value == null || block.value == null) throw new Error("no block to add record to");
  recordConnection.tx.create({
    metatype: NodeType.RECORD,
    blockPtr: nodePtr.value,
    packagePtr: block.value.packagePtr,
    parentPtr: nodePtr.value,
    valuePacked: {},
  });
}

//
// Presentation
//

const containerRef = ref<HTMLDivElement | null>(null);
const containerSize = useElementSize(containerRef);
const rowBodyWidth = computed(() => {
  return containerSize.width.value - ROW_ACTIONS_WIDTH;
});
const bodySize = computed(() => {
  return { width: containerSize.width.value, height: containerSize.height.value - ACTION_HEADER_HEIGHT };
});

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
  return "short"; // nocheckin: calibrate debounce levels properly
}

// nocheckin: store column views somewhere
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
} & (
  | {
      kind: "property";
      property: PropertyInfo;
      propertyName: string;
    }
  | {
      kind: "field";
      field: FieldData;
      storageKey: string;
    }
);
const columns: Ref<ColumnView[]> = computed(() => {
  const properties = [RecordProperty.createdEpoch, RecordProperty.updatedEpoch];
  const columns: ColumnView[] = [];
  for (const propertyId of properties) {
    const property = propertyInfo(NodeType.RECORD, propertyId);
    const propertyType = getPropertyType(property);
    const view = getViewForValueType(propertyType);
    const column: ColumnView = {
      kind: "property",
      idx: columns.length,
      id: propertyId.toString(),
      icon: getTypeIcon(propertyType),
      title: getPropertyTitle(property),
      type: propertyType,
      property,
      propertyName: getPropertyName(property),
      viewType: view?.type,
      viewComponent: view?.type != null ? getViewComponent(view?.type) : null,
      viewProps: view,
      width: 0,
      debounce: getColumnDebounce(propertyType, view?.type),
    };
    columns.push(column);
  }
  for (const field of fields.value) {
    const fieldType = resolveType(field, pkgGraph);
    const view = getViewForValueType(field);
    const column: ColumnView = {
      kind: "field",
      idx: columns.length,
      id: field.ck,
      title: field.name,
      icon: getNodeIcon(field),
      type: field,
      field,
      storageKey: getStorageKey(field, fieldType),
      viewType: view?.type,
      viewComponent: view?.type != null ? getViewComponent(view?.type) : null,
      viewProps: view,
      width: 0,
      debounce: getColumnDebounce(fieldType, view?.type),
    };
    columns.push(column);
  }

  // assign widths according to type
  for (const column of columns) {
    column.width = getMinColumnWidth(column.type, column.viewType);
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

function duplicateSelection() {
  throw new Error("nocheckin: duplicateSelection");
}

function deleteSelection() {
  throw new Error("nocheckin: deleteSelection");
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div
    ref="containerRef"
    class="h-full"
    :style="{
      marginLeft: variant == Variant.COMPACT ? '0' : `${FULL_PADDING}px`,
      marginRight: variant == Variant.COMPACT ? '0' : `${FULL_PADDING}px`,
      marginBottom: variant == Variant.COMPACT ? '0' : `${FULL_PADDING}px`,
    }"
  >
    <!-- Action header -->
    <div
      class="flex w-full flex-row items-center"
      :style="{
        height: `${ACTION_HEADER_HEIGHT}px`,
      }"
    >
      <!-- Expressions (filters/sorts) -->
      <!-- TODO :Incomplete: filter/sort Database -->
      <template v-if="false">
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
        <div v-if="true" class="flex flex-row items-center rounded border">
          <span class="h-full px-2 py-0.5 font-medium text-primary-900">2 selected</span>
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
        <span v-if="page?.total != null">{{ page.size }} / {{ page?.total }}</span>
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
      </div>
      <!-- Add record -->
      <button class="group/button rounded px-1 hover:bg-gray-100" @click="() => addRecord()">
        <i class="fa fa-plus mr-1.5 text-center text-gray-700 group-hover/button:text-primary-900" />
        <span class="group-hover/button:text-primary-900">Record</span>
      </button>
    </div>

    <!-- Body (scroll horizontally, and vertically if not compact) -->
    <Scroll
      :size="bodySize"
      :orientation="variant == Variant.COMPACT ? Orientation.HORIZONTAL : undefined"
      :track-width="ScrollbarWidth.sm"
      track-is-overlay
    >
      <div class="flex flex-col border-gray-200">
        <!-- Column headers (sticky) -->
        <div
          class="flex flex-row items-center border-gray-200"
          :style="{
            height: `${ROW_HEIGHT}px`,
          }"
        >
          <!-- Row actions -->
          <div
            v-if="variant != Variant.COMPACT"
            class="flex flex-shrink-0 flex-row items-center"
            :style="{
              width: `${ROW_ACTIONS_WIDTH}px`,
            }"
          ></div>
          <!-- Column header -->
          <div
            v-for="(column, i) in columns"
            :key="column.id"
            class="flex h-full flex-shrink-0 cursor-pointer items-center border-b border-gray-200 border-l-transparent px-2 py-1 hover:bg-gray-100"
            :class="[i > 0 ? 'border-l' : '']"
            :style="{
              width: `${column.width}px`,
            }"
            @click="
              () => {
                if (column.kind == 'field') {
                  canvas.inspect({ node: column.field, view: self });
                }
              }
            "
          >
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
              class="mr-1.5 rounded p-0.5 hover:cursor-pointer hover:bg-gray-100 data-[popover=true]:bg-gray-100"
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

        <!-- nocheckin: show status for rows if loading/error -->

        <!-- Row -->
        <div
          v-for="(record, j) in records"
          :key="record.id"
          class="flex flex-row border-gray-200"
          :class="[]"
          :data-node-id="record.id"
          :data-node-ck="record.id"
          :data-node-type="record.metatype"
          @click="() => canvas.inspect({ node: record, view: containerRef })"
        >
          <!-- Row actions -->
          <div
            v-if="variant != Variant.COMPACT"
            class="flex flex-shrink-0 flex-row items-start py-1"
            :style="{
              width: `${ROW_ACTIONS_WIDTH}px`,
            }"
          >
            <!-- Selection checkbox -->
            <input type="checkbox" class="h-4 w-4 rounded border-gray-200 text-gray-200 focus:ring-0" />
          </div>

          <!-- Columns -->
          <div
            v-for="(column, i) in columns"
            class="flex-shrink-0 cursor-pointer border-b border-gray-200 px-2 py-1 text-gray-900"
            :class="[i > 0 ? 'border-l' : '']"
            :style="{
              width: `${column.width}px`,
              minHeight: `${ROW_HEIGHT}px`,
            }"
            :data-column-id="column.id /* used to mark this as a column for click handler below */"
            @click="
              (event) => {
                if (column.viewType == ViewType.TOGGLE) {
                  const value = readColumnValue(record, column);
                  writeColumnValue(record, column, !value);
                } else {
                  const columnEl = (event.target as HTMLElement)?.closest('[data-column-id]');
                  if (columnEl == null) return;
                  pushPopover({
                    trigger: columnEl as HTMLElement,
                    reference: columnEl as HTMLElement,
                    info: {
                      component: column.viewComponent,
                      placement: 'inside-top-left',
                      referenceMargin: 0,
                      props: {
                        ...column.viewProps,
                        isInput: true,
                        size: {
                          metatype: ObjectType.BOX,
                          width: Math.max(100, columnEl?.getBoundingClientRect().width!),
                        },
                        isInline: true,
                        modelValue: readColumnValue(record, column),
                      },
                      dontAnimate: true,
                      onUpdate: (value: any) => writeColumnValue(record, column, value),
                      onApply: (value: any) => writeColumnValue(record, column, value, { debounce: 'tick' }),
                    },
                  });
                }
              }
            "
          >
            <!-- Inner column view -->
            <component
              :is="column.viewComponent"
              v-if="column.viewComponent != null"
              v-bind="column.viewProps"
              :model-value="readColumnValue(record, column)"
              :variant="Variant.STEALTH"
              @update:model-value="(value: any) => writeColumnValue(record, column, value)"
            />
            <!-- No view available (internal bug / missing feature) -->
            <span v-else class="text-danger-600">
              {{ column.viewType != null ? ViewType[column.viewType] : "???" }}
            </span>
          </div>
        </div>
      </div>
    </Scroll>
  </div>
</template>
