<script lang="ts" setup>
import { LogData, NodeType, PROPERTY_INFOS_BY_TYPE, ViewData, ViewType, type PropertyInfo } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { useGetConnection, type PreparedNodeConnection } from "@/system/connection";
import { FULL_WIDTH_VIEW_TYPES, getPropertyTitle } from "@/system/lang";
import { canvas } from "@/system/space";
import { unpackNodeDelta } from "@/system/transaction";
import { getPropertyType, type TypeIdentity } from "@/system/value";
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
  else return unpackNodeDelta(log.value.newNodePacked, log.value.nodePtr.type);
});
const oldNode = computed(() => {
  if (log.value?.oldNodePacked == null || log.value.nodePtr == null) return null;
  else return unpackNodeDelta(log.value.oldNodePacked, log.value.nodePtr.type);
});

type ChangedProperty = {
  property: PropertyInfo;
  type: TypeIdentity;
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
    const propertyView = getViewForValueType(propertyType);
    const changedProperty: ChangedProperty = {
      property,
      type: propertyType,
      viewType: propertyView?.viewType,
      viewProps: propertyView?.props,
      isFullWidth: propertyView != null && FULL_WIDTH_VIEW_TYPES.includes(propertyView?.viewType),
      oldValue: (oldNode.value as any)[property.name],
      newValue: (newNode.value as any)[property.name],
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
        v-for="{ property, viewType, viewProps, isFullWidth, oldValue, newValue } in properties"
        :key="property.id"
        class="py-1"
        :class="[isFullWidth ? 'flex flex-col gap-y-1' : 'flex flex-row  items-center gap-x-2.5']"
      >
        <!-- Title -->
        <span class="font-medium">{{ getPropertyTitle(property) }}</span>
        <!-- Diff -->
        <template v-if="viewType != null && hasViewComponent(viewType)">
          <!-- Old -->
          <component
            :is="getViewComponent(viewType)"
            v-if="oldValue != null"
            :class="['ml-auto flex-shrink-0', isFullWidth ? '' : '']"
            :style="{ width: isFullWidth ? '100%' : 'calc(45%)' }"
            v-bind="viewProps"
            :model-value="oldValue"
          />
          <div v-else><span class="text-gray-500">Unset</span></div>
          <!-- Arrow -->
          <i
            class="fas fa-arrow-right-long text-sm text-gray-400"
            :class="[isFullWidth ? 'rotate-90 text-center' : '']"
          />
          <!-- New -->
          <component
            :is="getViewComponent(viewType)"
            v-if="newValue != null"
            :class="['ml-auto flex-shrink-0', isFullWidth ? '' : '']"
            :style="{ width: isFullWidth ? '100%' : 'calc(45%)' }"
            v-bind="viewProps"
            :model-value="newValue"
          />
          <div v-else><span class="text-gray-500">Unset</span></div>
        </template>
        <div v-else class="flex flex-row items-center px-1 py-0.5 text-warning-600">
          <i class="fas fa-empty-set" />
          <span class="ml-1.5">No View for Type Type</span>
        </div>
      </div>
    </div>
    <div v-else>
      <!-- maybe show full node? paths? -->
      <span class="text-gray-500">No Changes</span>
    </div>
    <!-- ... context, details, etc. -->
  </div>
  <Inaccessible v-else class="h-full w-full" :node="nodePtr" :is-connected="connection.isConnected.value" />
</template>
