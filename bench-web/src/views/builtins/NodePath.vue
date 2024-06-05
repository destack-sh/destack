<script lang="ts" setup>
import { NodeType } from "@/proto/wire";
import type { NodeReferenceData } from "@/proto/wire";
import type { ReadNodeGraph } from "@/system/graph";
import { IconInline, getNodeIcon } from "@/system/icon";
import { canvas } from "@/system/space";
import { startDragging } from "@/utils/drag";
import { computed } from "vue";

const props = defineProps<{
  self?: NodeReferenceData;
  focus?: NodeReferenceData;
  graph: ReadNodeGraph;
}>();

const ancestors = props.graph.getAncestorsRef(
  computed(() => props.focus ?? props.self),
  { includeSelf: true, metatypes: [NodeType.BLOCK, NodeType.VIEW, NodeType.STEP, NodeType.FIELD] },
);
const path = computed(() => ancestors.value.slice().reverse());
</script>
<template>
  <!-- Breadcrumb -->
  <div class="flex flex-row items-center gap-x-1 truncate">
    <template v-for="(node, i) in path" :key="i">
      <!-- Node -->
      <button
        class="flex cursor-pointer flex-row items-center rounded px-1 hover:bg-primary-100 hover:text-primary-900"
        role="button"
        :class="node.id == self?.id || node.id == focus?.id ? 'text-primary-900' : 'text-gray-600'"
        :draggable="true"
        @click.stop="canvas.goToNode(node)"
        @dragstart.stop="(e: DragEvent) => startDragging(e, graph, node)"
      >
        <IconInline v-bind="getNodeIcon(node)" class="mr-1.5" />
        <span class="">{{ node.name }}</span>
      </button>
      <!-- Separator -->
      <i v-if="i < path.length - 1" class="fas fa-chevron-right text-gray-400" />
    </template>
  </div>
</template>
