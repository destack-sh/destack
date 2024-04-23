<script lang="ts" setup>
import { BoxData, NodeType, ViewData, ViewType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { getInspectionLayout } from "@/system/lang";
import { canvas, inspectionPtr } from "@/system/space";
import { getViewComponent } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "nodePtr"
  >
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
  <div v-if="inspectedNode && inspectionLayout" class="mx-auto h-full w-full min-w-[320px] max-w-[640px] bg-white py-3">
    <!-- nocheckin: Inspector -->
    <ul class="flex flex-col gap-y-2">
      <template
        v-for="({ title, category, property, viewType, props, isFullWidth }, i) of inspectionLayout.properties"
        :key="property.id"
      >
        <!-- Category Header -->
        <div v-if="i != 0 && inspectionLayout.properties[i - 1].category != category" class="mt-2">
          <div class="mb-3 h-[1px] w-full bg-gray-200" />
          <span class="px-5 font-semibold text-gray-900">{{ category }}</span>
        </div>
        <!-- Property -->
        <li class="px-5" :class="[isFullWidth ? 'flex flex-col' : 'flex flex-row flex-wrap gap-x-5']">
          <!-- Title & Controls -->
          <span>
            <span class="py-1 font-medium text-gray-700">{{ title }}</span>
          </span>
          <!-- Value -->
          <component
            v-if="viewType != null && getViewComponent(viewType)"
            :is="getViewComponent(viewType)"
            class="ml-auto"
            :class="isFullWidth ? 'w-full' : ''"
            v-bind="props"
          />
          <div v-else class="ml-auto text-danger-600">{{ viewType != null ? ViewType[viewType] : '<not found>' }}</div>
        </li>
      </template>
    </ul>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty/missing state -->
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
