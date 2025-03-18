<script lang="ts" setup>
import { NodeGraph, ReadNodeGraph } from "@/language/core/graph";
import { NodeReferenceData, NodeType, SelectionData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import HistoryNavigator from "@/views/builtins/HistoryNavigator.vue";
import NodePath from "@/views/builtins/NodePath.vue";

const HEADER_HEIGHT = VIEW_DEFAULT_ROOT_HEADER_HEIGHT;

const props = defineProps<{
  self?: TypedNodeReferenceData<NodeType.VIEW>;
  nodePtr?: NodeReferenceData;
  focus?: SelectionData;
  graph?: ReadNodeGraph | null;
}>();
</script>
<template>
  <div class="group flex w-full max-w-full flex-row items-center px-2" :style="{ height: HEADER_HEIGHT + 'px' }">
    <!-- History -->
    <HistoryNavigator :self="self" />
    <!-- Breadcrumb -->
    <NodePath
      v-if="nodePtr && graph"
      :container="nodePtr"
      :self="nodePtr"
      :focus="props.focus?.nodesPtr[0]"
      :graph="graph"
    />
    <!-- Meta & Controls -->
    <div class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-1.5 pl-1">
      <slot name="meta" />
    </div>
  </div>
</template>
