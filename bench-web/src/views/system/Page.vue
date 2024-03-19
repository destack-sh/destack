<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import { useGetNodes, useLoadedGraph } from "@/system/connection";
import { spaceRegistry } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());

const self = toRef(props, "self");
const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(self);
const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  computed(() => ({
    roots: [props.nodePtr!],
    options: { descendantTypes: [NodeType.BLOCK] },
    enabled: props.nodePtr != null,
  })),
);
// const blocks = pkgGraph.getDescendantsRef(self, NodeType.BLOCK, )

spaceRegistry.registerCurrent(self);
defineExpose({ self });
</script>
<template>
  <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md">
    <!-- Placeholder content for testing vertical scrolling -->
    <div class="flex h-[200%] w-full items-center justify-center bg-gray-100">
      <span class="text-4xl font-bold">{{ size }}</span>
    </div>
  </Scroll>
</template>
