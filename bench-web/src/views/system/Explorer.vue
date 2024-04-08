<script lang="tsx" setup>
import { ViewData, NodeReferenceData, ViewType, NodeType, BenchType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr } from "@/system/space";
import { useExistingConnection, type GraphConnection } from "@/system/connection";
import { computed, toRef } from "vue";
import { packagePtr } from "@/system/client";
import { describeNode } from "@/proto/wiring";

const props = defineProps<
  { self: NodeReferenceData } & Pick<ViewData, "type" | "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);

const rootPtr = computed(() => {
  if (props.nodePtr != null) return props.nodePtr;
  else if (props.type == ViewType.EXPLORER) return packagePtr.value;
  else if (props.type == ViewType.OUTLINE) return inspectionPtr.value;
  else return null;
});
const { graph, connection } = useExistingConnection(rootPtr, {
  isOptional: true,
  match: {
    predicate: (c) => {
      // NOTE: hack to exclude Bench connection (which also contains package) :ConnectionMatching
      return !(c as GraphConnection<"get", NodeType>).params.roots.some((r) => r.type == NodeType.BENCH);
    },
  },
});
const rootNodes = graph.getChildrenRef(rootPtr, NodeType.BLOCK, { ignoreAncestors: true });

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <ul v-if="type == ViewType.EXPLORER" class="flex h-full w-full flex-col bg-white">
    <!-- nocheckin :Incomplete: explorer -->
    <li v-for="node in rootNodes" :key="node.id">
      {{ node.name }}
    </li>
  </ul>
  <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
    <!-- Empty/missing state -->
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Select Node to Inspect</span>
  </div>
</template>
