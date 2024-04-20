<script lang="ts" setup>
import { ViewData, NodeReferenceData, NodeType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { useExistingConnection } from "@/system/connection";
import { toRef } from "vue";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW> } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
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
    <!-- TODO :Incomplete: Inspector -->
    Inspect:{{ describeNode(inspectedNode) }}
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty/missing state -->
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
