<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import { describeNode } from "@/proto/wiring";
import { useExistingConnection, useGetNodes } from "@/system/connection";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Inaccessible from "@/views/private/Inaccessible.vue";
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
const page = pkgGraph.getRef(toRef(props, "nodePtr"));
// const blocks = pkgGraph.getDescendantsRef(self, NodeType.BLOCK, )

// sync name with title
// nocheckin :Incomplete: Page

canvas.registerView(self);
defineExpose({ self });
</script>
<template>
  <div v-if="page" :style="{ width: size.width + 'px', height: size.height + 'px' }" class="flex w-full flex-col bg-white">
    <!-- Page header/self -->
    <div class="w-full"></div>
    <!-- Page content -->
    <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md" track-is-overlay>
      Page {{ nodePtr }} -> {{ describeNode(page) }}
    </Scroll>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
