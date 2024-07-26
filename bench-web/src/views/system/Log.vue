<script lang="ts" setup>
import {
  LogData,
  NodeType,
  ObjectType,
  PROPERTY_ENUM_BY_TYPE,
  PROPERTY_INFOS_BY_TYPE,
  ViewData,
  ViewType,
  type PropertyInfo,
} from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { useGetConnection, type PreparedNodeConnection } from "@/system/connection";
import { FULL_WIDTH_VIEW_TYPES, getPropertyTitle } from "@/system/lang";
import { canvas } from "@/system/space";
import { unpackProtoJson } from "@/system/transaction";
import { getPropertyType, unpackBuiltinObject, type TypeIdentity } from "@/system/value";
import { getViewForValueType } from "@/system/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "isInline">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const { connection, graph } =
  props.preparedConnection ??
  useGetConnection(
    { name: `log.${props.nodePtr?.id}` },
    computed(() => ({
      scope: PACKAGE_SCOPE.value,
      roots: [props.nodePtr!],
      isEnabled: props.nodePtr != null,
    })),
  );
const log = graph.getRef(props.nodePtr, { ignoreAncestors: true }) as Ref<LogData | undefined>;
const newNode = computed(() => {
  if (log.value?.newNodePacked == null || log.value.nodePtr == null) return null;
  else
    return unpackBuiltinObject(
      unpackProtoJson(log.value.newNodePacked),
      log.value.nodePtr.type as unknown as ObjectType,
    );
});
const oldNode = computed(() => {
  if (log.value?.oldNodePacked == null || log.value.nodePtr == null) return null;
  else
    return unpackBuiltinObject(
      unpackProtoJson(log.value.oldNodePacked),
      log.value.nodePtr.type as unknown as ObjectType,
    );
});

type ChangedProperty = {
  property: PropertyInfo;
  type: TypeIdentity;
  isKernel: boolean;
  viewType: ViewType | undefined;
  viewProps: any | undefined;
  isFullWidth: boolean;
  oldValue: any;
  newValue: any;
};
const properties = computed(() => {
  if (log.value?.nodePtr == null && log.value?.properties != null && log.value.properties.length > 0) return [];
  const propertiesIds = log.value!.properties.slice().sort();
  const properties = propertiesIds
    .filter((p) => p >= 30)
    .map((p) => PROPERTY_INFOS_BY_TYPE[log.value!.nodePtr!.type][p]);
  const changedProperties: ChangedProperty[] = [];
  for (const property of properties) {
    const propertyType = getPropertyType(property);
    const propertyName = PROPERTY_ENUM_BY_TYPE[log.value?.nodePtr!.type!]![property.id];
    const propertyView = getViewForValueType(propertyType);
    const changedProperty: ChangedProperty = {
      property,
      type: propertyType,
      isKernel: property.isKernel ?? false,
      viewType: propertyView?.viewType,
      viewProps: { ...propertyView?.props, isInput: false },
      isFullWidth: propertyView != null && FULL_WIDTH_VIEW_TYPES.includes(propertyView?.viewType),
      oldValue: (oldNode.value as any)[propertyName],
      newValue: (newNode.value as any)[propertyName],
    };
    changedProperties.push(changedProperty);
  }
  return changedProperties;
});

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="log">
    <!-- NOTE :Incomplete :UX: Log View is currently only intended for inline display in Feed -->
    <!-- Changed properties if available -->
    <div v-if="properties.length > 0" class="flex flex-col">
      <!-- Changed property -->
      <div
        v-for="{ property, viewType, viewProps, isFullWidth, isKernel, oldValue, newValue } in properties"
        :key="property.id"
        class="py-1"
        :class="[isFullWidth ? 'flex flex-col gap-y-1' : 'flex flex-row flex-wrap items-center gap-x-2.5']"
      >
        <!-- Title -->
        <span class="font-medium">{{ getPropertyTitle(property) }}</span>
        <!-- Diff -->
        <div v-if="isKernel" class="flex-1 text-right">
          <span class="italic text-gray-500">Hidden</span>
        </div>
        <template v-else-if="viewType != null && hasViewComponent(viewType)">
          <!-- Old -->
          <component
            :is="getViewComponent(viewType)"
            v-if="oldValue != null"
            :class="[isFullWidth ? '' : 'ml-auto']"
            :style="{ width: isFullWidth ? '100%' : '' }"
            v-bind="viewProps"
            :model-value="oldValue"
          />
          <div v-else :class="[isFullWidth ? '' : 'ml-auto']"><span class="text-gray-500">Unset</span></div>
          <!-- Arrow -->
          <i
            class="fas fa-arrow-right-long text-sm text-gray-400"
            :class="[isFullWidth ? 'my-0.5 rotate-90 text-center' : '']"
          />
          <!-- New -->
          <component
            :is="getViewComponent(viewType)"
            v-if="newValue != null"
            :class="[isFullWidth ? '' : '']"
            :style="{ width: isFullWidth ? '100%' : '' }"
            v-bind="viewProps"
            :model-value="newValue"
          />
          <div v-else><span class="text-gray-500">Unset</span></div>
        </template>
        <div v-else class="flex flex-1 flex-row items-center justify-end px-1 py-0.5 text-warning-600">
          <i class="fas fa-empty-set" />
          <span class="ml-1.5">No View for Type</span>
        </div>
      </div>
    </div>
    <div v-else>
      <!-- maybe show full node? paths? -->
      <span class="text-gray-600">No Changes</span>
    </div>
    <!-- ... context, details, etc. -->
  </div>
  <Inaccessible v-else class="h-full w-full" :node="nodePtr" :is-connected="connection.isConnected.value" />
</template>
