<script lang="ts" setup>
import { ViewData, NodeReferenceData, NodeType, type PropertyInfo, PROPERTY_INFOS_BY_TYPE } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { useExistingConnection } from "@/system/connection";
import { computed, toRef, type Ref } from "vue";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { Casing, toCasing } from "@/utils/string";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW> } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: inspectedGraph, connection: inspectedConnection } = useExistingConnection(inspectionPtr);
const inspectedNode = inspectedGraph.getRef(inspectionPtr);

type InspectedProperty = {
  title: string;
  property: PropertyInfo;
};
const inspectedProperties: Ref<InspectedProperty[]> = computed(() => {
  if (inspectedNode.value == null) return [];
  const propertyInfos = PROPERTY_INFOS_BY_TYPE[inspectedNode.value.metatype];
  const properties = Object.values(propertyInfos)
    .filter((p) => p.id >= 30 && !p.isInternal && !p.isAutoset && !p.isComputed && !p.isSystem)
    .map((property) => {
      let cleanName = property.name;
      if (cleanName.endsWith("_ptr")) cleanName = cleanName.slice(0, -4);
      const title = toCasing(cleanName, Casing.CAMEL, true);
      return {
        title,
        property,
      };
    });
  return properties;
});

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="inspectedNode" class="h-full w-full bg-white p-2">
    <!-- TODO :Incomplete: Inspector -->
    Inspect:{{ describeNode(inspectedNode) }}
    <ul class="flex flex-col">
      <li v-for="{ title, property } of inspectedProperties" :key="property.id">
        {{ title }}: {{ property.kind }} {{ property.id }}
      </li>
    </ul>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty/missing state -->
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
