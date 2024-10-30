<script lang="ts" setup>
import { NodeType } from "@/proto/wire";
import type { NodeReferenceData } from "@/proto/wire";
import type { ReadNodeGraph } from "@/language/graph";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { toCamelName } from "@/language/const";
import { canvas } from "@/system/space";
import { startDragging } from "@/ui/drag";
import { computed, toRef } from "vue";

const props = defineProps<{
  container?: NodeReferenceData;
  focus?: NodeReferenceData | null;
  graph: ReadNodeGraph;
}>();

const METATYPES = [NodeType.BLOCK, NodeType.VIEW, NodeType.STEP, NodeType.FIELD];

const ancestorsFocus = props.graph.getAncestorsRef(toRef(props, "focus"), { includeSelf: true, metatypes: METATYPES });
const ancestorsSelf = props.graph.getAncestorsRef(toRef(props, "container"), {
  includeSelf: true,
  metatypes: METATYPES,
});
const path = computed(() => {
  const ancestors =
    ancestorsFocus.value.length > ancestorsSelf.value.length ? ancestorsFocus.value : ancestorsSelf.value;
  return ancestors.slice().reverse();
});
</script>
<template>
  <!-- Breadcrumb -->
  <div class="flex flex-row items-center gap-x-1 truncate">
    <template v-for="(node, i) in path" :key="i">
      <!-- Node -->
      <button
        class="flex cursor-pointer flex-row items-center rounded-sm px-0.5 hover:bg-gray-100 hover:text-primary-900"
        role="button"
        :data-node-id="node.id"
        :data-node-ck="(node as any).ck"
        :data-node-type="node.metatype"
        :class="
          node.id == container?.id || node.id == focus?.id || canvas.isInspected(node)
            ? 'text-primary-900'
            : canvas.isHighlighted(node)
              ? 'text-primary-700'
              : 'text-gray-600'
        "
        :draggable="true"
        @click.stop="canvas.goToNode(node)"
        @dragstart.stop="(e: DragEvent) => startDragging(e, graph, node)"
      >
        <IconInline v-bind="getNodeIcon(node)" class="mr-1.5 w-5" />
        <span class="">{{ (node as any).name ?? toCamelName(NodeType, node.metatype) }}</span>
      </button>
      <!-- Separator -->
      <i v-if="i < path.length - 1" class="fas fa-chevron-right text-gray-400" />
    </template>
  </div>
</template>
