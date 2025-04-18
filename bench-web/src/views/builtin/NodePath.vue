<script lang="ts" setup>
import type { ReadNodeGraph } from "@/language/core/graph";
import type { NodeReferenceData, PackageData, PageData } from "@/proto/wire";
import { INLINE_NODE_TYPES, NodeType } from "@/proto/wire";
import { canvas } from "@/system/space";
import { startDragging } from "@/ui/drag";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { computed, toRef } from "vue";

const props = defineProps<{
  container: NodeReferenceData;
  focus?: NodeReferenceData | null;
  self: NodeReferenceData | null;
  graph: ReadNodeGraph;
}>();

const METATYPES = [...INLINE_NODE_TYPES, NodeType.THREAD];

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
const selfIndex = computed(() => path.value.findIndex((node) => node.id == props.container.id));
</script>
<template>
  <!-- Breadcrumb -->
  <div class="flex flex-row items-center truncate">
    <template v-for="(node, i) in path" :key="i">
      <!-- Node -->
      <button
        class="flex cursor-pointer flex-row items-center rounded-sm px-0.5 hover:bg-gray-100"
        :class="[i > 0 ? 'ml-1' : '', i > selfIndex && selfIndex != -1 ? 'text-gray-400' : 'text-gray-900']"
        role="button"
        :data-node-id="node.id"
        :data-node-ck="(node as any).ck"
        :data-node-type="node.metatype"
        :data-node-bench-id="(node as PackageData).benchPtr?.id"
        :draggable="true"
        @click.stop="canvas.goToNode(node)"
        @dragstart.stop="(e: DragEvent) => startDragging(e, node)"
      >
        <NodeReference
          :node="node"
          size="sm"
          class="max-w-[200px] truncate"
          :isLight="selfIndex != -1 && i != selfIndex"
          :isIconLight="selfIndex != -1 && i > selfIndex"
        />
      </button>
      <!-- Separator -->
      <span v-if="i < path.length - 1" class="ml-1.5 text-gray-400">/</span>
    </template>
    <span v-if="path.length == 0" class="text-gray-400">???</span>
  </div>
</template>
