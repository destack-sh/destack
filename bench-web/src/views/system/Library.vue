<script lang="tsx" setup>
import { ViewData, NodeReferenceData } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { useExistingConnection } from "@/system/connection";
import { computed, toRef } from "vue";
import { describeNode } from "@/proto/wiring";

const props = defineProps<
  { self: NodeReferenceData } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);

const focusedPtr = computed(() => canvas.focusedRootNodePtr.value ?? inspectionPtr.value);

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <!-- Library -->
  <div v-if="focusedPtr" class="h-full w-full bg-white">
    <!-- nocheckin :Incomplete: library -->
    Library for {{ describeNode(focusedPtr) }}
  </div>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty state -->
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
