<script lang="ts" setup>
import { NodeType, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { getInspectionLayout } from "@/system/lang";
import { canvas, inspectionPtr } from "@/system/space";
import { viewEmits, type ViewExposed } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW> } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: inspectedGraph, connection: inspectedConnection } = useExistingConnection(inspectionPtr);
const inspectedNode = inspectedGraph.getRef(inspectionPtr);

const inspectionLayout = computed(() => {
  if (inspectedNode.value == null) return null;
  const layout = getInspectionLayout(inspectedNode.value);
  return layout;
});

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="inspectedNode && inspectionLayout" class="h-full w-full bg-white p-2">
    <!-- nocheckin: Inspector -->
    <ul class="flex flex-col">
      <template
        v-for="({ title, category, property, component, props, isFullWidth }, i) of inspectionLayout.properties"
        :key="property.id"
      >
        <!-- Category Divider -->
        <div
          v-if="i != 0 && inspectionLayout.properties[i - 1].category != category"
          class="my-1 h-[1px] w-full bg-gray-300"
        />
        <!--  -->
        <li>{{ title }}: {{ property.kind }} {{ property.id }}</li>
      </template>
    </ul>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty/missing state -->
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
