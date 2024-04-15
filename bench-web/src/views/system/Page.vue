<script lang="tsx" setup>
import { BlockData, BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import { describeNode } from "@/proto/wiring";
import { useExistingConnection, useGetConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits, type FocusAnchor } from "@/views/common";
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
import { type ViewExposed } from "@/views/common";
import type { ViewComponent } from "@/views";

const HEADER_HEIGHT = 24;
const DEPTH_OFFSET = 40;
const MIN_BLOCK_WIDTH = 600;
const MAX_BLOCK_WIDTH = 800;
const MIN_GUTTER_WIDTH = 80;
const ROOT_BLOCK_GAP_Y = 14;
const NESTED_BLOCK_GAP_Y = 8;

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

/** Gets the position for a div anchored at the start/end of the given block */
function getAnchorPosition(anchor: "start" | "end", blockIdx: number, anchorWidth: number) {
  if (anchor == "start") {
    const depth = expandedItems.value[blockIdx]?.depth;
    return {
      top: (depth == 0 ? -ROOT_BLOCK_GAP_Y : NESTED_BLOCK_GAP_Y) / 2 - anchorWidth / 2 + "px",
    };
  } else {
    const depth = expandedItems.value[blockIdx]?.depth;
    const nextDepth = expandedItems.value[blockIdx + 1]?.depth;
    return {
      bottom: (depth == 0 && nextDepth == 0 ? -ROOT_BLOCK_GAP_Y : -NESTED_BLOCK_GAP_Y) / 2 - anchorWidth / 2 + "px",
    };
  }
}

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

// focus
function focus(anchor: FocusAnchor | NodeReferenceData) {
  console.log("Page.focus: nocheckin", props.nodePtr, anchor);
  if (typeof anchor != "object") {
    // ...
  } else {
    if (anchor.id == props.nodePtr?.id) {
      // just focus first
      expandedBlockRefs.value[expandedItems.value[0].nodeRef.id!].$el.scrollIntoView({
        block: "start",
        behavior: "instant",
      });
    } else {
      const block = expandedBlockRefs.value[anchor.id!];
      block?.$el.scrollIntoView({ block: "center", behavior: "instant" });
    }
  }
  return false;
}

canvas.registerView(self);
defineExpose<ViewExposed>({ self, actions, focus });
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
      :focus="props.focus?.nodesPtr[0]"
      :graph="pkgGraph"
    />

    <!-- Page content -->
    <Scroll
      :size="{ width: size.width, height: size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
    >
      <div ref="contentRef" class="mb-16 flex flex-col">
        <!-- Self Block (=this Page block) -->
        <div
          class="mb-2 w-full border-b bg-white py-1.5"
          :class="[
            props.nodePtr?.id == focusedNodePtr?.id ? 'border-primary-900' : 'border-gray-300',
            props.nodePtr?.id == inspectionPtr?.id ? 'bg-primary-100' : '',
          ]"
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
            marginTop: depth == 0 ? ROOT_BLOCK_GAP_Y + 'px' : '0',
          }"
        >
          <!-- Left gutter -->
          <div
            class="relative flex-shrink-0"
            :style="{
              width: widths.gutter + DEPTH_OFFSET * depth + 'px',
              marginTop: depth != 0 ? NESTED_BLOCK_GAP_Y + 'px' : '0',
            }"
          >
            <!-- References -->
            <!-- ... -->
          </div>

          <!-- Block wrapper -->
          <div
            class="group/block-wrapper relative border-gray-100"
            :style="{
              width: widths.block - DEPTH_OFFSET * depth + 'px',
            }"
          >
            <!-- Nested space -->
            <div v-if="depth != 0" class="" :style="{ height: NESTED_BLOCK_GAP_Y + 'px' }" />

            <!-- Create above/below -->
            <div
              v-for="anchor in i < expandedItems.length - 1 ? ['start'] : ['start', 'end']"
              :key="anchor"
              role="button"
              class="group/create absolute z-10 h-[6px] w-full flex-shrink-0 text-center opacity-0 transition-colors duration-100 hover:opacity-100"
              :style="getAnchorPosition(anchor as 'start' | 'end', i, (anchor == 'start' || depth != expandedItems[i + 1]?.depth) ? 8  : 4)"
            >
              <!-- Line with a gap for the button -->
              <div class="relative">
                <div
                  class="absolute left-0 h-[1px] w-[48.5%] translate-y-1 bg-gray-300 transition-colors duration-100 group-hover/create:bg-primary-900"
                />
                <div
                  class="absolute right-0 h-[1px] w-[48.5%] translate-y-1 bg-gray-300 transition-colors duration-100 group-hover/create:bg-primary-900"
                />
              </div>
              <button
                class="-translate-y-[6px] px-1 text-gray-300 transition-colors duration-100 group-hover/create:text-primary-900"
              >
                &plus;
              </button>
            </div>

            <!-- Drag above/below -->
            <div
              v-if="activeDropZone?.targetId == blockPtr.id"
              class="absolute z-10 h-1 w-full rounded-sm bg-primary-400"
              :style="getAnchorPosition(activeDropZone?.anchor as 'start' | 'end', i, 4)"
            />

            <!-- Block -->
            <Block
              :ref="(ref: any) => ref ? (expandedBlockRefs[blockPtr.id!] = ref) : delete expandedBlockRefs[blockPtr.id!]"
              class="rounded-md border bg-white"
              :class="[
                blockPtr.id == focusedNodePtr?.id ? 'border-primary-900' : 'border-gray-300 hover:border-primary-900',
                blockPtr.id == inspectionPtr?.id ? 'bg-primary-100' : '',
              ]"
              :node-ptr="blockPtr"
              :prepared-connection="preparedPkgConnection"
              v-contextmenu="() => {
                return {items: menuActionsLike({wildcard: ['common.edit.*', 'common.move.*']}, {context: {triggerNode: blockPtr}})} as ContextMenuInfo
              }"
              :draggable="true"
              @dragstart="(e: DragEvent) => startDragging(e, pkgGraph, blockPtr)"
            />
          </div>

          <!-- Right gutter -->
          <div
            class="relative flex-shrink-0"
            :style="{
              width: widths.gutter,
              marginTop: depth != 0 ? NESTED_BLOCK_GAP_Y + 'px' : '0',
            }"
          >
            <!-- Activity / Notices / ... -->
          </div>
        </div>
      </div>
    </Scroll>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
