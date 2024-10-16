<script lang="ts" setup>
import { ViewData, NodeType, FieldData, PropertyInfo, TypeInfoData, RecordProperty, RecordData, ViewType } from "@/proto/wire";
import {
  propertyInfo,
  toNodeRefOneOf,
  toPlainNodeRef,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, Ref, toRef } from "vue";
import { SearchConnectionParams, useExistingConnection, useSearchConnection } from "@/system/connection";
import { PACKAGE_SCOPE } from "@/system/client";
import Field from "@/views/system/Field.vue";
import { getPropertyType, getStorageKey, TypeIdentity } from "@/language/field";
import { getPropertyName, getPropertyTitle } from "@/language/const";
import { useElementSize } from "@vueuse/core";
import { assertNever } from "@/utils/functools";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "variant">
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
  if (nodePtr.value == null || block.value == null) throw new Error("No block to add record to");
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

// nocheckin: store column views somewhere
type ColumnView = {
  idx: number;
  id: string;
  title: string;
  type: TypeIdentity;
	viewType?: ViewType;
	viewProps?: any;
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
  const properties = [RecordProperty.id, RecordProperty.createdAt];
  const views: ColumnView[] = [];
  for (const propertyId of properties) {
    const property = propertyInfo(NodeType.RECORD, propertyId);
    const column: ColumnView = {
      kind: "property",
      idx: views.length,
      id: propertyId.toString(),
      title: getPropertyTitle(property),
      type: getPropertyType(property),
      property,
      propertyName: getPropertyName(property),
    };
    views.push(column);
  }
  for (const field of fields.value) {
    const column: ColumnView = {
      kind: "field",
      idx: views.length,
      id: field.ck,
      title: field.name,
      type: field,
      field,
      storageKey: getStorageKey(field),
    };
    views.push(column);
  }
  return views;
});

function getColumnValue(record: RecordData, column: ColumnView) {
  if (column.kind === "property") {
    return (record as any)[column.propertyName];
  } else if (column.kind === "field") {
    return (record.valuePacked as any)?.[column.storageKey];
  } else {
    assertNever(column);
  }
}

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div ref="containerRef">
    <!-- nocheckin: Database -->
    total:{{ page?.total }} isStale:{{ isStale }} isConnected:{{ isConnected }}
    <div class="flex flex-col">
      <!-- Columns -->
      <div class="flex flex-row">
        <div v-for="column in columns" :key="column.id">
          <span>{{ column.title }}</span>
        </div>
      </div>
      <!-- Rows -->
      <div
        v-for="record in records"
        :key="record.id"
        class="flex flex-row"
        :data-node-id="record.id"
        :data-node-ck="record.id"
        :data-node-type="record.metatype"
      >
        <!-- Columns -->
        <div v-for="column in columns">
          {{ getColumnValue(record, column) }}
        </div>
      </div>
    </div>
    <button class="hover:text-primary-900" @click="addRecord">add</button>
  </div>
</template>
