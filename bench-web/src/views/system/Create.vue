<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionBasePtr, inspectionPtr } from "@/system/space";
import { useExistingConnection } from "@/system/connection";
import { computed, toRef } from "vue";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW> } & Pick<ViewData, "name" | "title" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const basePtr = computed(() => inspectionPtr.value ?? inspectionBasePtr.value);
const { graph: spaceGraph } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(basePtr);
const baseNode = pkgGraph.getRef(basePtr);

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="basePtr" class="h-full w-full bg-white">
    <!-- TODO :Incomplete: Create -->
    <div class="flex h-full w-full flex-col justify-center text-center">
      <span class=""><i class="fas fa-traffic-cone mr-1 text-warning-800" /> Create </span>
      <span class="mt-2 font-mono text-xs text-gray-600">{{ describeNode(basePtr) }}</span>
    </div>
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty state -->
    <span>
      <i class="fas fa-empty-set text-gray-500" />
      <span class="ml-1.5 text-gray-600">Select Node to Inspect</span>
    </span>
  </div>
</template>
