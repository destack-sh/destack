<script lang="tsx" setup>
import { BlockData, BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import { describeNode } from "@/proto/wiring";
import { useExistingConnection, useGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Inaccessible from "@/views/private/Inaccessible.vue";
import { computed, ref, toRef, type Ref } from "vue";
import { isDescendantOf, moveNode, walkDescendantsRef } from "@/system/graph";
import NavigationBar from "@/views/private/NavigationBar.vue";
import type { ActionMapImplementation } from "@/system/action";
import Block from "@/views/system/Block.vue";
import { computedValue } from "@/utils/ref";
import { menuActionsLike, type ContextMenuInfo } from "@/utils/menu";
import { startDragging, useMultiDropZone } from "@/utils/drag";

const HEADER_HEIGHT = 24;
const DEPTH_OFFSET = 40;
const MIN_BLOCK_WIDTH = 500;
const MAX_BLOCK_WIDTH = 1000;
const MIN_GUTTER_WIDTH = 80;
const MIN_TOTAL_WIDTH = MIN_BLOCK_WIDTH + MIN_GUTTER_WIDTH * 2;
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
const preparedPkgConnection = useGetConnection(
  { name: `page.${props.nodePtr?.id}` },
  computed(() => ({
    roots: [props.nodePtr!],
    options: { descendantTypes: [NodeType.BLOCK] },
    enabled: props.nodePtr != null,
  })),
);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const page = pkgGraph.getRef(toRef(props, "nodePtr")) as Ref<BlockData> | undefined;
const { items: expandedItems } = walkDescendantsRef({
  graph: pkgGraph,
  rootPtr: toRef(props, "nodePtr"),
  nodeTypes: ref([NodeType.BLOCK]),
  isExpanded: () => true,
  isIncludedSelf: () => true,
  isIncludedChildren: (node) => !node.isPage,
});
const expandedBlockRefs: Ref<Record<string, InstanceType<typeof Block>>> = ref({});
const contentRef = ref<HTMLElement | null>(null);
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);

// size block/gutter horizontally (try to fit both until min block width, ignoring depth)
const widths = computed(() => {
  const blockWidth = Math.min(MAX_BLOCK_WIDTH, Math.max(MIN_BLOCK_WIDTH, props.size.width - MIN_GUTTER_WIDTH * 2));
  const gutterWidth = Math.max(MIN_GUTTER_WIDTH, (props.size.width - blockWidth) / 2);
  return { block: blockWidth, gutter: gutterWidth };
});

// sync title with page name
// nocheckin

//
// Interaction
//

// dragging
const { activeDropZone } = useMultiDropZone({
  name: "page",
  container: contentRef,
  targets: expandedBlockRefs,
  orientation: Orientation.VERTICAL,
  kinds: ["node"],
  metatypes: [NodeType.BLOCK],
  fallbackToClosest: true,
  allowDrop: (dragged, anchor, targetId) => {
    const target = pkgGraph.get({ id: targetId });
    return dragged.kind == "node" && target != null && !isDescendantOf(pkgGraph, target, dragged.node);
  },
  onDrop: (dragged, anchor, targetId) => {
    if (targetId != null && dragged.kind == "node") {
      const target = pkgGraph.getOrFail({ id: targetId });
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, target, anchor);
    }
  },
});

// actions
// nocheckin
const actions: Partial<ActionMapImplementation<"common">> = {};

canvas.registerView(self);
defineExpose({ self });
</script>
<template>
  <div
    v-if="page"
    :style="{ width: size.width + 'px', height: size.height + 'px' }"
    class="flex w-full flex-col bg-white text-gray-900"
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
      <div ref="contentRef" class="flex flex-col">
        <!-- Self Block (=this Page block) -->
        <div
          class="mb-2 w-full border-b bg-white px-2 py-3"
          :class="props.nodePtr?.id == focusedNodePtr?.id ? 'border-primary-900' : 'border-gray-300'"
        >
          <Block
            class=""
            :style="{ width: widths.block + 'px', marginLeft: widths.gutter + 'px' }"
            :node-ptr="props.nodePtr"
            :prepared-connection="preparedPkgConnection"
          />
        </div>

        <!-- In-page Blocks -->
        <!-- Block 'line' -->
        <div
          v-for="({ nodeRef: blockPtr, depth }, i) in expandedItems"
          :key="blockPtr.id"
          class="group/block-line relative flex flex-row"
          :style="{
            marginTop: depth === 0 ? ROOT_BLOCK_GAP_Y + 'px' : '0',
          }"
        >
          <!-- Left gutter -->
          <div
            class="relative"
            :style="{
              width: widths.gutter + DEPTH_OFFSET * depth + 'px',
            }"
          >
            <!-- Create above/below -->
            <!-- nocheckin: button in place 'context' menu? -->
            <button
              v-for="dir in ['above', 'below']"
              :key="dir"
              class="!hover:opacity-100 absolute right-0.5 rounded-md border border-gray-400 bg-white px-[3px] text-gray-600 opacity-0 hover:bg-primary-200 hover:text-gray-700 group-hover/block-line:opacity-40"
              :class="dir === 'above' ? '-top-[17px]' : '-bottom-[17px]'"
            >
              <i class="fas fa-plus" />
            </button>
          </div>

          <!-- Block wrapper -->
          <div
            class="group/block-wrapper relative"
            :style="{
              width: widths.block - DEPTH_OFFSET * depth + 'px',
            }"
          >
            <!-- Drag above/below -->
            <div
              v-if="activeDropZone?.targetId == blockPtr.id"
              class="absolute z-10 h-1 rounded-sm bg-primary-400"
              :style="{
                left: 8 + depth * DEPTH_OFFSET + 'px',
                width: 'calc(100% - ' + (8 + depth * DEPTH_OFFSET) + 'px)',
                [activeDropZone?.anchor == 'start' ? 'top' : 'bottom']:
                  depth == 0 ? -ROOT_BLOCK_GAP_Y / 2 - 2 + 'px' : '-2px',
              }"
            />

            <!-- Block -->
            <Block
              :ref="(ref: any) => ref ? (expandedBlockRefs[blockPtr.id!] = ref) : delete expandedBlockRefs[blockPtr.id!]"
              class="border bg-white p-2"
              :class="[
                // nocheckin: make rounded corners match for nested blocks
                //  maybe also make border thicker or thin gray rectangle to show they're connected while maintaining some spacing?
                depth == 0 ? 'rounded-md' : '',
                blockPtr.id == focusedNodePtr?.id ? 'border-primary-900' : 'border-gray-300 hover:border-gray-400',
              ]"
              :node-ptr="blockPtr"
              :prepared-connection="preparedPkgConnection"
              v-contextmenu="() => {
                return {items: menuActionsLike({wildcard: ['common.edit.*', 'common.move.*']}, {context: {triggerNode: blockPtr}})} as ContextMenuInfo
              }"
              @dragstart="(e: DragEvent) => startDragging(e, pkgGraph, blockPtr)"
            />
          </div>

          <!-- Right gutter -->
          <div
            class="relative"
            :style="{
              width: widths.gutter,
            }"
          >
            <!-- Activity / Notices / etc. -->
          </div>
        </div>
      </div>
    </Scroll>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
