<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, ViewData } from "@/proto/wire";
import { useLoadedGraph } from "@/system/connection";
import { viewEmits } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon"
  >
>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(toRef(props, "self"));
const tabs = spaceGraph.getChildrenRef(toRef(props, "self"), NodeType.VIEW);

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div class="text-center" :style="{ width: size.width + 'px', height: size.height + 'px' }">
    {{ tabs.length }} tabs
  </div>
</template>
