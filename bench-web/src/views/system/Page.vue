<script lang="tsx" setup>
import { ViewData, NodeReferenceData, NodeType } from "@/proto/wire/";
import { useGetNodes, useLoadedGraph } from "@/system/connection";
import { viewEmits } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
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

defineExpose({ self });
</script>
<template>
  <div></div>
</template>
