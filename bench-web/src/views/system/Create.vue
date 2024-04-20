<script lang="ts" setup>
import { ViewData, NodeReferenceData, NodeType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionBasePtr, inspectionPtr } from "@/system/space";
import { useExistingConnection } from "@/system/connection";
import { computed, toRef } from "vue";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW> } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const basePtr = computed(() => inspectionPtr.value ?? inspectionBasePtr.value);
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: inspectedGraph, connection: inspectedConnection } = useExistingConnection(basePtr);
const baseNode = inspectedGraph.getRef(basePtr);

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div v-if="basePtr" class="h-full w-full bg-white">
    <!-- TODO :Incomplete: Creator -->
    Create:{{ describeNode(basePtr) }}
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty state -->
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
