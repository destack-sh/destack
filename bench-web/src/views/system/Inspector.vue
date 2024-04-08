<script lang="tsx" setup>
import { ViewData, NodeReferenceData } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { useExistingConnection } from "@/system/connection";
import { toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: inspectedGraph, connection: inspectedConnection } = useExistingConnection(inspectionPtr);
const inspectedNode = inspectedGraph.getRef(inspectionPtr);

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div class="h-full w-full bg-white" v-if="inspectedNode">
		<!-- nocheckin :Incomplete: inspector -->
    {{ inspectedNode?.id }}
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty/missing state -->
    <i class="fas text-gray-500 fa-empty-set" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
