<script lang="ts" setup>
import { supergraph } from "@/globals";
import { toCamelName } from "@/language/core/const";
import { AnyNodeData, NodeType, NodeTypeOptionInfo } from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { _setDragImage, activeDrag } from "@/ui/drag";
import { IconInline, makeIcon } from "@/ui/icon";
import NodeReference from "@/views/builtin/NodeReference.vue";
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
  const seenNodesById: Record<string, AnyNodeData> = {};
  const unnamedNodesByType: Partial<Record<NodeType, AnyNodeData[]>> = {};
  const unwrappedNodes: AnyNodeData[] = [];
  for (const node of nodes) {
    // unwrap
    const nodeType = node.metatype as unknown as NodeType;
    let unwrappedNode: AnyNodeData | null = null;
    if (isNode(node, NodeType.BLOCK) && node.nodePtr != null) {
      unwrappedNode = supergraph.get(node.nodePtr) ?? node;
    } else {
      unwrappedNode = node;
    }
    // deduplicate nodes (may unwrap to same node multiple times)
    if (seenNodesById[unwrappedNode.id] != null) continue;
    seenNodesById[unwrappedNode.id] = unwrappedNode;
    // add
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
          size="sm"
          class="max-w-[200px] truncate rounded-2xl border border-gray-300 bg-white px-1.5 py-0.5 text-gray-900"
        />
        <span
          v-if="dragInfo.unwrappedNodes.length > 0 && Object.keys(dragInfo.unnamedNodesByType).length > 0"
          class="text-gray-400"
          >+
        </span>
        <div
          v-for="nodeType in Object.keys(dragInfo.unnamedNodesByType)"
          :key="nodeType"
          class="flex flex-row items-center gap-x-1.5 rounded-2xl border border-gray-300 bg-white px-1.5 py-0.5 text-gray-900"
        >
          <span v-if="dragInfo.unnamedNodesByType[nodeType as unknown as NodeType]!.length > 0" class="">
            {{ dragInfo.unnamedNodesByType[nodeType as unknown as NodeType]!.length }}
          </span>
          <IconInline
            v-bind="makeIcon(NodeTypeOptionInfo[nodeType as unknown as NodeType]!.icon!)"
            class="text-gray-700"
          />
          <span class="">{{ toCamelName(NodeType, nodeType) }}</span>
        </div>
      </div>
      <div v-else>
        <span class="text-danger-600">???</span>
      </div>
    </div>
  </div>
</template>
