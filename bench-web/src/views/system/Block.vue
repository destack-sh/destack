<script lang="tsx" setup>
import { ViewData, NodeReferenceData, NodeType } from "@/proto/wire/";
import { useGetNodes, useLoadedGraph } from "@/system/graph";
import { viewEmits } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(toRef(props, "self"));
const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  computed(() => ({
    roots: [props.nodePtr!],
    options: { descendantTypes: [NodeType.FIELD, NodeType.TRIGGER] },
    enabled: props.nodePtr != null,
  })),
);

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div></div>
</template>
