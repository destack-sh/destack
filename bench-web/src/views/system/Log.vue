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
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { useGetConnection, type PreparedNodeConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { getViewForValueType } from "@/ui/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { getViewComponent, hasViewComponent } from "@/views/registry";
import { computed, toRef, type Ref } from "vue";
import { unpackBuiltinObject } from "@/language/value";
import { getPropertyType, type TypeIdentity } from "@/language/field";
import { getPropertyTitle } from "@/language/const";
import { FULL_WIDTH_VIEW_TYPES } from "@/ui/view";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "isInline">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr));

const { connection, graph } =
  props.preparedConnection ??
  useGetConnection(
    { name: `log.${nodePtr.value?.id}` },
    computed(() => ({
      scope: PACKAGE_SCOPE.value,
      roots: [nodePtr.value!],
      isEnabled: nodePtr.value != null,
    })),
  );
const log = graph.getRef(nodePtr.value, { ignoreAncestors: true }) as Ref<LogData | undefined>;

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
const properties: Ref<ChangedProperty[]> = computed(() => {
  return []; // :Incomplete
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
          <!-- ... :Incomplete -->
        </template>
      </div>
    </div>
    <div v-else>
      <!-- maybe show full node? paths? -->
      <span class="text-gray-600">No Changes</span>
    </div>
    <!-- ... context, details, etc. -->
  </div>
  <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="connection" />
</template>
