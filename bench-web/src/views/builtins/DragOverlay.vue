<script lang="ts" setup>
import { supergraph } from "@/globals";
import { toCamelName } from "@/language/core/const";
import { unwrapBlockDefinition } from "@/language/source/block";
import { AnyNodeData, NodeType } from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { _setDragImage, activeDrag } from "@/ui/drag";
import { ICON_BY_NODE_TYPE, IconInline } from "@/ui/icon";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { computed } from "vue";

type DragInfo = {
  unwrappedNodes: AnyNodeData[];
  unnamedNodesByType: Partial<Record<NodeType, AnyNodeData[]>>;
};

const MAX_UNWRAPPED_NODES = 3;

const dragInfo = computed<DragInfo | null>(() => {
  if (activeDrag.value?.kind != "node" && activeDrag.value?.kind != "selection") return null;
  const nodes = activeDrag.value?.nodes ?? [];

  // unwrap blocks
  const unnamedNodesByType: Partial<Record<NodeType, AnyNodeData[]>> = {};
  const unwrappedNodes: AnyNodeData[] = [];
  for (const node of nodes) {
    const nodeType = node.metatype as unknown as NodeType;
    let unwrappedNode: AnyNodeData | null = null;
    if (isNode(node, NodeType.BLOCK) && node.nodePtr != null) {
      unwrappedNode = supergraph.get(node.nodePtr) ?? node;
    } else {
      unwrappedNode = node;
    }

    if (
      unwrappedNodes.length > MAX_UNWRAPPED_NODES ||
      ((unwrappedNode as any).name == null && (unwrappedNode as any).title == null)
    ) {
      unnamedNodesByType[nodeType] ??= [];
      unnamedNodesByType[nodeType].push(unwrappedNode);
    } else {
      unwrappedNodes.push(unwrappedNode);
    }
  }

  return { unwrappedNodes, unnamedNodesByType };
});
</script>
<template>
  <!-- Drag image overlay -->
  <!-- Wrapper to ensure dragImageRef is always set -->
  <div :ref="(ref) => _setDragImage(ref as any)" class="absolute -top-[100px] left-20 py-1 pl-2">
    <div v-if="activeDrag" class="">
      <!-- And wrapper to offset within the image to ensure the text isn't obscured by the cursor -->
      <div v-if="dragInfo != null" class="flex flex-row items-center gap-x-1">
        <NodeReference
          v-for="node in dragInfo.unwrappedNodes"
          :key="node.id"
          :node="node"
          size="regular"
          class="max-w-48 rounded-2xl border border-gray-300 bg-white px-1.5 py-0.5 text-gray-900"
        />
        <span
          v-if="dragInfo.unwrappedNodes.length > 0 && Object.keys(dragInfo.unnamedNodesByType).length > 0"
          class="text-gray-400"
          >+</span
        >
        <div
          v-for="nodeType in Object.keys(dragInfo.unnamedNodesByType)"
          :key="nodeType"
          class="flex flex-row items-center gap-x-1 rounded-2xl border border-gray-300 bg-white px-1.5 py-0.5 text-gray-900"
        >
          <IconInline v-bind="ICON_BY_NODE_TYPE[nodeType as unknown as NodeType]" class="text-gray-700" />
          <span class="">{{ dragInfo.unnamedNodesByType[nodeType as unknown as NodeType]!.length }}</span>
          <span class="">{{ toCamelName(NodeType, nodeType) }}</span>
        </div>
      </div>
      <div v-else>
        <span class="text-danger-600">???</span>
      </div>
    </div>
  </div>
</template>
