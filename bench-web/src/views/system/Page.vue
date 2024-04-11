<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import { describeNode } from "@/proto/wiring";
import { useExistingConnection, useGetNodes } from "@/system/connection";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "text" | "icon" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  { name: `page.${props.nodePtr?.id}` },
  computed(() => ({
    roots: [props.nodePtr!],
    options: { descendantTypes: [NodeType.BLOCK] },
    enabled: props.nodePtr != null,
  })),
);
const page = pkgGraph.getRef(props.nodePtr);
// const blocks = pkgGraph.getDescendantsRef(self, NodeType.BLOCK, )

canvas.registerView(self);
defineExpose({ self });
</script>
<template>
  <Scroll
    v-if="page"
    :size="size"
    :orientation="Orientation.VERTICAL"
    :track-width="ScrollbarWidth.md"
    track-is-overlay
    class="bg-white"
  >
    Page {{ nodePtr }} -> {{ describeNode(page) }}
  </Scroll>
  <div v-else class="flex h-full w-full flex-col justify-center text-center">
    TODO :Incomplete: not accessible: {{ nodePtr }}
  </div>
</template>
