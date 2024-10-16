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
import {
  BenchType,
  FieldData,
  FieldType,
  IconData,
  NodeType,
  ObjectType,
  PropertyInfo,
  RecordData,
  RecordProperty,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { propertyInfo, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { SearchConnectionParams, useExistingConnection, useSearchConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { getNodeIcon, getTypeIcon, IconInline } from "@/ui/icon";
import { PopoverInfoIn, pushPopover } from "@/ui/popover";
import { getViewForValueType, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { assertNever } from "@/utils/functools";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Icon from "@/views/content/Icon.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import { getViewComponent } from "@/views/registry";
import { useElementSize } from "@vueuse/core";
import { computed, ref, Ref, toRef } from "vue";

const ACTION_HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const ROW_HEIGHT = 32;
const MIN_COLUMN_WIDTH = 32;
const SELECT_COLUMN_WIDTH = 32;
const ROW_PADDING_X = 8; // per side, so ROW_PADDING*2 per side
const ROW_PADDING_Y = 4; // per side, so ROW_PADDING*2 per side

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "variant" | "isInput">
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
      first: 10,
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

function getMinColumnWidth(type: TypeIdentity, viewType: ViewType | undefined) {
  return 50; // nocheckin: size column widths for types
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
    };
    columns.push(column);
  }

  // assign widths according to type
  for (const column of columns) {
    column.width = getMinColumnWidth(column.type, column.viewType);
  }

  // grow columns to fit container (if possible)
  const totalWidth = columns.reduce((acc, column) => acc + column.width, 0);
  const containerWidth = containerSize.width.value;
  if (totalWidth < containerWidth && columns.length > 0) {
    const scale = containerWidth / totalWidth;
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
    return (record.valuePacked as any)?.[column.storageKey];
  } else {
    assertNever(column);
  }
}

function writeColumnValue(record: RecordData, column: ColumnView, value?: any) {
  throw new Error("nocheckin: writeColumnValue");
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div ref="containerRef">
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
          <span class="text-gray-400 group-hover/button:text-primary-900">Add filter</span>
        </button>
        <button class="group/button rounded px-1 hover:bg-gray-100">
          <i class="fa fa-plus mr-1.5 text-center text-gray-300 group-hover/button:text-primary-900" />
          <span class="text-gray-400 group-hover/button:text-primary-900">Add sort</span>
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

        <!-- Pagination -->
        <span v-if="page?.total != null">{{ page?.total }}</span>

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
          <span>Add field</span>
        </button>
      </div>
      <!-- Add record -->
      <button class="group/button rounded px-1 hover:bg-gray-100" @click="() => addRecord()">
        <i class="fa fa-plus mr-1.5 text-center text-gray-700 group-hover/button:text-primary-900" />
        <span class="group-hover/button:text-primary-900">Add record</span>
      </button>
    </div>

    <!-- Body (scroll horizontally) -->
    <div class="flex flex-col border-gray-200">
      <!-- Column headers -->
      <div
        class="flex flex-row items-center border-b border-gray-200"
        :style="{
          height: `${ROW_HEIGHT}px`,
        }"
      >
        <!-- Column header -->
        <div
          v-for="(column, i) in columns"
          :key="column.id"
          class="flex h-full cursor-pointer items-center border-transparent px-2 py-1 hover:bg-gray-100"
          :class="[i > 0 ? 'border-l' : '']"
          :style="{
            width: `${column.width}px`,
          }"
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
            class="font-medium"
            :model-value="column.title"
            :value-type="NAME_TYPE"
            :variant="Variant.STEALTH"
            is-input
            @update:model-value="
              (newValue) => pkgConnection.tx.update(column.field, { name: newValue }, { debounce: 'long' })
            "
          />
          <span v-else class="font-medium">{{ column.title }}</span>
        </div>
      </div>

      <!-- Rows (scroll vertically)-->
      <!-- nocheckin: show status for rows if loading/error -->
      <div
        v-for="(record, j) in records"
        :key="record.id"
        class="flex flex-row border-b border-gray-200"
        :class="[]"
        :data-node-id="record.id"
        :data-node-ck="record.id"
        :data-node-type="record.metatype"
      >
        <!-- Columns -->
        <div
          v-for="(column, i) in columns"
          class="cursor-pointer border-gray-200 px-2 py-1 text-gray-900"
          :class="[i > 0 ? 'border-l' : '']"
          :style="{
            width: `${column.width}px`,
            minHeight: `${ROW_HEIGHT}px`,
          }"
          @click="
            (event) => {
              if (column.viewType == ViewType.TOGGLE) {
                const value = readColumnValue(record, column);
                writeColumnValue(record, column, !value);
              } else {
                pushPopover({
                  trigger: event.target as HTMLElement,
                  reference: event.target as HTMLElement,
                  info: {
                    component: column.viewComponent,
                    placement: 'inside-top-left',
										referenceMargin: 0,
                    props: {
                      ...column.viewProps,
											isInput: true,
											// size: { metatype: ObjectType.BOX, width: Math.max(MIN_WIDTH, buttonRef?.getBoundingClientRect().width!) },
											isInline: true,
                      modelValue: readColumnValue(record, column),
                    },
                    onApply: (value) => writeColumnValue(record, column, value),
                  },
                });
              }
            }
          "
        >
          <component
            :is="column.viewComponent"
            v-if="column.viewComponent != null"
            v-bind="column.viewProps"
            :model-value="readColumnValue(record, column)"
            :variant="Variant.STEALTH"
            @update:model-value="writeColumnValue(record, column)"
          />
          <span v-else class="text-danger-600">{{ column.viewType != null ? ViewType[column.viewType] : "???" }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
