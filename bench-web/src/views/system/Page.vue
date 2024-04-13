<script lang="tsx" setup>
import { BlockData, BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import { describeNode } from "@/proto/wiring";
import { useExistingConnection, useGetNodes } from "@/system/connection";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Inaccessible from "@/views/private/Inaccessible.vue";
import { computed, ref, toRef, type Ref } from "vue";
import { walkDescendantsRef } from "@/system/graph";
import NavigationBar from "@/views/private/NavigationBar.vue";

const HEADER_HEIGHT = 24;
const DEPTH_OFFSET = 40;
const MIN_BLOCK_WIDTH = 400;
const MAX_BLOCK_WIDTH = 1000;
const MIN_GUTTER_WIDTH = 40;
const ROOT_BLOCK_GAP_Y = 12;

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "text" | "icon" | "nodePtr" | "focus" | "selection" | "expansion"
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
const page = pkgGraph.getRef(toRef(props, "nodePtr")) as Ref<BlockData> | undefined;
const { items: expandedItems } = walkDescendantsRef({
  graph: pkgGraph,
  rootPtr: toRef(props, "nodePtr"),
  nodeTypes: ref([NodeType.BLOCK]),
  isExpanded: () => true,
  isIncludedSelf: () => true,
  isIncludedChildren: (node) => !node.isPage,
});

// size block/gutter horzintally (try to fit both until min block width, ignoring depth)
const widths = computed(() => {
  const blockWidth = Math.min(MAX_BLOCK_WIDTH, Math.max(MIN_BLOCK_WIDTH, props.size.width - MIN_GUTTER_WIDTH * 2));
  const gutterWidth = Math.max(MIN_GUTTER_WIDTH, (props.size.width - blockWidth) / 2);
  return { block: blockWidth, gutter: gutterWidth };
});

// sync title with page name
// nocheckin :Incomplete: Page

canvas.registerView(self);
defineExpose({ self });
</script>
<template>
  <div
    v-if="page"
    :style="{ width: size.width + 'px', height: size.height + 'px' }"
    class="flex w-full flex-col bg-white"
  >
    <!-- Page header -->
    <NavigationBar
      class="border-b border-gray-300"
      :width="size.width"
      :height="HEADER_HEIGHT"
      :self="nodePtr"
      :focus="focus?.nodesPtr[0]"
      :graph="pkgGraph"
    />

    <!-- Page content -->
    <Scroll
      :size="{ width: size.width, height: size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
    >
      <div class="flex flex-col">
        <!-- Self block -->
        <div class="w-full border-b border-gray-300 bg-white p-2">
          <div class="" :style="{ width: widths.block + 'px', marginLeft: widths.gutter + 'px' }">
            {{ page.name }}
          </div>
        </div>

        <!-- In-page Blocks -->
        <template v-for="{ node: block, depth } in expandedItems" :key="block.id">
          <div
            class="rounded-md border border-gray-300 bg-white p-2"
            :style="{
              width: widths.block - DEPTH_OFFSET * depth + 'px',
              marginLeft: widths.gutter + DEPTH_OFFSET * depth + 'px',
              marginTop: depth === 0 ? ROOT_BLOCK_GAP_Y + 'px' : '0',
            }"
          >
            {{ block.name }}
          </div>
        </template>
      </div>
    </Scroll>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
