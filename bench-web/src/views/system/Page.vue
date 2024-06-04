<script lang="ts" setup>
import {
  BenchType,
  BlockData,
  BlockType,
  BoxData,
  NodeReferenceData,
  NodeType,
  Orientation,
  RunKind,
  RunStatus,
  Variant,
  ViewData,
  ViewType,
} from "@/proto/wire/";
import { makeNode, toNodeReference, type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionContext, ActionMapImplementation } from "@/system/action";
import { useHierarchicalNodeMoveActions } from "@/system/block";
import { useExistingConnection, useGetConnection } from "@/system/connection";
import { getGroupedChildrenRef, isDescendantOf, walkDescendantsRef, type NodeTreeItem } from "@/system/graph";
import { ICON_BY_BLOCK_TYPE, IconInline } from "@/system/icon";
import { EXPOSED_BLOCK_TYPES, RUNNABLE_BLOCK_TYPES, createBlock, moveNode, toCamelName } from "@/system/lang";
import { canvas, inspectionPtr } from "@/system/space";
import { makeTypeInfo } from "@/system/value";
import { startDragging, useMultiDropZone } from "@/utils/drag";
import { blurDocument } from "@/utils/element";
import { ScrollbarWidth } from "@/utils/layout";
import { menuActionsLike, type PopoverContext, type PopoverInfo, type PopoverInfoIn } from "@/utils/menu";
import { computedValue } from "@/utils/ref";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NavigationBar from "@/views/builtins/NavigationBar.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Block from "@/views/system/Block.vue";
import { computed, nextTick, ref, toRef, watch, type Ref } from "vue";

const HEADER_HEIGHT = 36;
const DEPTH_OFFSET = 40;
const MIN_BLOCK_WIDTH = 500;
const MAX_BLOCK_WIDTH = 800;
const MIN_GUTTER_WIDTH = 44;
const ROOT_BLOCK_GAP_Y = 8;
const NESTED_BLOCK_GAP_Y = 8;
const HANDLE_WIDTH = 6;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "text" | "icon" | "nodePtr" | "focus" | "selection" | "expansion"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const selfView = spaceGraph.getRef(self);
const preparedPkgConnection = useGetConnection(
  { name: `page.${props.nodePtr?.id}` },
  computed(() => ({
    roots: [props.nodePtr!],
    options: { descendantTypes: [NodeType.BLOCK] },
    isEnabled: props.nodePtr != null,
  })),
);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const page = pkgGraph.getRef(toRef(props, "nodePtr")) as Ref<BlockData | undefined>;
const selfBlockRef = ref<InstanceType<typeof Block> | null>(null);
const { items: expandedBlocks } = walkDescendantsRef({
  graph: pkgGraph,
  rootPtr: toRef(props, "nodePtr"),
  nodeTypes: ref([NodeType.BLOCK]),
  isExpanded: () => true,
  isIncludedSelf: () => true,
  isIncludedChildren: (node) => !node.isPage,
});
const expandedBlocksWithSelf: Ref<NodeTreeItem<NodeType.BLOCK>[]> = computed(() => {
  if (page.value == null) {
    return [];
  } else {
    const selfItem: NodeTreeItem<NodeType.BLOCK> = {
      id: page.value.id,
      node: page.value,
      nodePtr: toNodeReference(page.value),
      depth: 0,
      hasChildren: true,
    };
    return [selfItem, ...expandedBlocks.value];
  }
});
const expandedBlockRefs: Ref<Record<string, InstanceType<typeof Block>>> = ref({});
const contentRef = ref<HTMLElement | null>(null);
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);

// messages / notices
const { childrenByParentId: threadsByBlockId } = getGroupedChildrenRef({
  graph: pkgGraph,
  parentPtrs: expandedBlocksWithSelf,
  childTypes: [NodeType.MESSAGE],
});

// size block/gutter horizontally (try to fit both until min block width, ignoring depth)
const widths = computed(() => {
  // divide space between block and gutter up to target gutter width
  const blockWidth = Math.min(MAX_BLOCK_WIDTH, Math.max(MIN_BLOCK_WIDTH, props.size.width - MIN_GUTTER_WIDTH * 2));
  const gutterWidth = Math.max(MIN_GUTTER_WIDTH, (props.size.width - blockWidth) / 2);
  return { block: blockWidth, gutter: gutterWidth };
});

/** Gets the position for a div anchored at the start/end of the given block */
function getAnchorPosition(anchor: "start" | "end", blockIdx: number, anchorWidth: number) {
  if (anchor == "start") {
    const depth = expandedBlocks.value[blockIdx]?.depth;
    return {
      top: (depth == 0 ? -ROOT_BLOCK_GAP_Y : NESTED_BLOCK_GAP_Y) / 2 - anchorWidth / 2 + "px",
    };
  } else {
    const depth = expandedBlocks.value[blockIdx]?.depth;
    const nextDepth = expandedBlocks.value[blockIdx + 1]?.depth;
    return {
      bottom: (depth == 0 && nextDepth == 0 ? -ROOT_BLOCK_GAP_Y : -NESTED_BLOCK_GAP_Y) / 2 - anchorWidth / 2 + "px",
    };
  }
}

// sync view title with page name
// NOTE: syncing page view titles with their block's names only when active means they may be stale sometimes.
watch(
  () => page.value?.name,
  () => {
    if (page.value != null && selfView.value != null && page.value?.name != selfView.value?.title) {
      pkgConnection.tx.update(selfView.value, { title: page.value?.name }, { debounce: "long" });
    }
  },
  { immediate: true },
);

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
      const target = pkgGraph.getOrError({ id: targetId });
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, anchor, target);
    }
  },
});

// actions
const getBlockFromContext = (ctx: ActionContext | undefined): { block: BlockData | null; idx: number } => {
  let block = expandedBlocks.value.find((item) => item.nodePtr.id == ctx?.triggerNode?.id)?.node;
  if (!block) block = expandedBlocks.value.find((item) => item.nodePtr.id == focusedNodePtr.value?.id)?.node;
  if (!block) return { block: null, idx: -1 };
  const idx = expandedBlocks.value.findIndex((item) => item.nodePtr.id == block!.id);
  return { block, idx };
};
const actions: Partial<ActionMapImplementation<"common">> = {
  // create
  "common.create.above": {
    action: (action, context) => {
      const { block } = getBlockFromContext(context);
      if (block == null) return false;
      createAndFocusBlock({ type: BlockType.TEXT }, "before", block);
    },
  },
  "common.create.below": {
    action: (action, context) => {
      const { block } = getBlockFromContext(context);
      if (block == null) return false;
      createAndFocusBlock({ type: BlockType.TEXT }, "after", block);
    },
  },
  // edit
  "common.edit.archive": {
    action: (action, context) => {
      const { block } = getBlockFromContext(context);
      if (block == null) return false;
      pkgConnection.tx.archive(block);
    },
  },
  "common.edit.delete": {
    action: (action, context) => {
      const { block } = getBlockFromContext(context);
      if (block == null) return false;
      pkgConnection.tx.softDelete(block);
    },
  },
  // navigation
  "common.navigate.up": {
    action: (action, context) => {
      const { block, idx } = getBlockFromContext(context);
      let toFocus = expandedBlocks.value[idx - 1]?.nodePtr;
      if (block == null) {
        if (props.nodePtr?.id == focusedNodePtr.value?.id)
          toFocus = expandedBlocks.value[expandedBlocks.value.length - 1]?.nodePtr;
        else return false;
      }
      if (toFocus != null) canvas.focus(spaceConnection.tx, { node: toFocus, view: self.value });
    },
  },
  "common.navigate.down": {
    action: (action, context) => {
      const { block, idx } = getBlockFromContext(context);
      let toFocus = expandedBlocks.value[idx + 1]?.nodePtr;
      if (block == null) {
        if (props.nodePtr?.id == focusedNodePtr.value?.id) toFocus = expandedBlocks.value[0]?.nodePtr;
        else return false;
      }
      if (toFocus != null) canvas.focus(spaceConnection.tx, { node: toFocus, view: self.value });
    },
  },
  // move
  ...useHierarchicalNodeMoveActions({
    graph: pkgGraph,
    basePtr: toRef(props, "nodePtr"),
    txFactory: () => pkgConnection.tx,
    expandedItems: expandedBlocks,
    getItemFromContext: (context) => {
      const { idx } = getBlockFromContext(context);
      const item = expandedBlocks.value[idx];
      return { item, idx };
    },
  }),
};
function createAndFocusBlock(
  blockIn: { type: BlockType } & Partial<BlockData>,
  anchor: "before" | "after" | "inside",
  targetPtr: BlockData | TypedNodeReferenceData<NodeType.BLOCK>,
) {
  const block = createBlock(pkgConnection.tx, pkgGraph, blockIn, anchor, targetPtr);
  nextTick(() => focus(toNodeReference(block)));
}

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  if (typeof anchor != "object") {
    if (anchor != "bottom") {
      const block = expandedBlockRefs.value[expandedBlocks.value[0].nodePtr.id!];
      block?.$el.scrollIntoView({ block: "start", behavior: "instant" });
    } else {
      const block = expandedBlockRefs.value[expandedBlocks.value[expandedBlocks.value.length - 1].nodePtr.id!];
      block?.$el.scrollIntoView({ block: "end", behavior: "instant" });
    }
  } else {
    if (anchor.id == props.nodePtr?.id) {
      // just focus first
      if (expandedBlocks.value.length > 0) {
        expandedBlockRefs.value[expandedBlocks.value[0].nodePtr.id!]?.$el.scrollIntoView({
          block: "nearest",
          behavior: "instant",
        });
      }
    } else {
      const block = expandedBlockRefs.value[anchor.id!];
      block?.$el.scrollIntoView({ block: "nearest", behavior: "instant" });
    }
  }

  blurDocument(); // nothing to focus directly
  return true;
}
const isFocusedAbsolute = canvas.isFocusedAbsoluteRef(self);

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
      :width="size.width"
      :height="HEADER_HEIGHT"
      :self="nodePtr"
      :focus="props.focus?.nodesPtr[0]"
      :graph="pkgGraph"
      class="border-gray-200"
      data-keep-inspection-in-base="true"
    />

    <!-- Page content -->
    <Scroll
      :size="{ width: size.width, height: size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
    >
      <div ref="contentRef" class="mb-16 flex flex-col">
        <!-- TODO :UX: indicate 'self' block in Page view better -->
        <!--  (while still retaining all the functionality of a full block 'line') -->
        <!-- Block 'line' -->
        <div
          v-for="({ nodePtr: blockPtr, node: block, depth }, i) in expandedBlocksWithSelf"
          :key="blockPtr.id"
          class="group/block-line relative flex min-w-fit flex-row"
          :class="i == 0 ? 'mb-4' : ''"
          :style="{
            marginTop: depth == 0 ? ROOT_BLOCK_GAP_Y + 'px' : '0',
          }"
        >
          <!-- Left gutter -->
          <div
            class="relative flex flex-shrink-0 flex-row items-start justify-end gap-x-2 text-right"
            :style="{
              width: widths.gutter + DEPTH_OFFSET * depth + 'px',
              marginTop: (depth != 0 ? NESTED_BLOCK_GAP_Y : 0) + 6 + 'px',
            }"
          >
            <!-- Activity / Run / ... -->
            <!-- Run -->
            <button
              v-if="RUNNABLE_BLOCK_TYPES.includes(block.type)"
              class="text-gray-400 hover:text-primary-900"
              :class="inspectionPtr?.id == blockPtr?.id ? '' : 'opacity-0 group-hover/block-line:opacity-100'"
              data-keep-inspection-in-base="true"
              @click="
                () => {
                  // nocheckin: session.* action handling (in Block/Step/Page/...)
                  const run = makeNode({
                    metatype: NodeType.RUN,
                    parentPtr: block.packagePtr,
                    packagePtr: block.packagePtr,
                    kind: RunKind.BLOCK,
                    status: RunStatus.SCHEDULED,
                    blockPtr: blockPtr,
                  });
                  run.rootPtr = toNodeReference(run);
                  pkgConnection.tx.create(run);
                }
              "
            >
              <i class="fas fa-play" />
            </button>
            <!-- Handle -->
            <div
              class="h-full rounded transition-colors duration-75"
              :class="
                inspectionPtr?.id == blockPtr?.id
                  ? 'bg-primary-900'
                  : focusedNodePtr?.id == blockPtr?.id
                    ? 'bg-gray-300'
                    : 'bg-transparent group-hover/block-line:bg-gray-200'
              "
              :style="{ width: HANDLE_WIDTH + 'px' }"
            />
          </div>

          <!-- Block wrapper -->
          <div
            class="group/block-wrapper relative rounded border-gray-100"
            :style="{
              width: widths.block - DEPTH_OFFSET * depth + 'px',
            }"
          >
            <!-- Nested space -->
            <div v-if="depth != 0" class="" :style="{ height: NESTED_BLOCK_GAP_Y + 'px' }" />

            <!-- Create above/below (in between blocks) -->
            <div
              v-for="anchor in i < expandedBlocks.length - 1 ? (i == 0 ? [] : ['start']) : ['start', 'end']"
              :key="anchor"
              v-menu="
                (): PopoverInfoIn => ({
                  component: ViewType.PICKER,
                  placement: 'bottom',
                  props: { valueType: makeTypeInfo({ benchType: BenchType.BLOCK_TYPE, isRequired: true }) },
                  onApply: (blockType: BlockType) =>
                    createAndFocusBlock({ type: blockType }, anchor == 'start' ? 'before' : 'after', blockPtr),
                })
              "
              role="button"
              class="absolute h-[6px] w-full flex-shrink-0 text-center text-gray-300 opacity-0 transition-colors duration-75 hover:z-10 hover:text-gray-300 hover:opacity-100 data-[popover=true]:text-primary-900 data-[popover=true]:opacity-100"
              :style="
                getAnchorPosition(
                  anchor as 'start' | 'end',
                  i,
                  anchor == 'start' || depth != expandedBlocks[i + 1]?.depth ? 8 : 4,
                )
              "
              data-keep-inspection-in-base="true"
            >
              <!-- Line with a gap for the button -->
              <div class="relative">
                <svg class="translate-y-1" width="100%" height="1px" viewBox="0 0 100 1" preserveAspectRatio="none">
                  <path d="M0,0.5 L49,0.5" fill="none" stroke="currentColor" stroke-width="1" />
                  <path d="M100,0.5 L51,0.5" fill="none" stroke="currentColor" stroke-width="1" />
                </svg>
                <button class="-translate-y-[8px] px-1 text-primary-900">&plus;</button>
              </div>
            </div>

            <!-- Drag above/below -->
            <div
              v-if="activeDropZone?.targetId == blockPtr.id"
              class="absolute z-10 h-1 w-full rounded-sm bg-primary-400"
              :style="getAnchorPosition(activeDropZone?.anchor as 'start' | 'end', i, 4)"
            />

            <!-- Block -->
            <Block
              :ref="
                (ref: any) => (ref ? (expandedBlockRefs[blockPtr.id!] = ref) : delete expandedBlockRefs[blockPtr.id!])
              "
              v-contextmenu="
                (context: PopoverContext): PopoverInfo => ({
                  kind: 'menu',
                  placement: 'bottom-right',
                  items: menuActionsLike(
                    [
                      'common.edit.rename',
                      'common.edit.morph',
                      'common.edit.move',
                      'common.edit.duplicate',
                      'common.edit.archive',
                      'common.edit.delete',
                      'message.handle.startThread',
                      'block.*',
                    ],
                    {
                      context: { ...context, triggerNode: blockPtr },
                    },
                  ),
                })
              "
              class="w-full px-2 py-1.5 data-[dragging=true]:opacity-50"
              borderless
              :variant="Variant.STEALTH"
              :node-ptr="blockPtr"
              :prepared-connection="preparedPkgConnection"
              :draggable="true"
              @dragstart.stop="(e: DragEvent) => startDragging(e, pkgGraph, blockPtr)"
            />
          </div>

          <!-- Right gutter -->
          <div
            class="relative flex flex-shrink-0 flex-row items-start justify-start px-0.5 transition-colors duration-75"
            :style="{
              width: widths.gutter,
              marginTop: (depth != 0 ? NESTED_BLOCK_GAP_Y : 0) + 6 + 'px',
            }"
          >
            <!-- Messages -->
            <button
              v-menu="
                (context: PopoverContext): PopoverInfoIn => ({
                  component: ViewType.CHAT,
                  placement: 'bottom',
                  container: 'containingRoot',
                  containerMargin: 12,
                  props: {
                    variant: Variant.COMPACT,
                    nodePtr: toNodeReference(threadsByBlockId[blockPtr.id!]?.at(-1)!) ?? blockPtr,
                  },
                })
              "
              class="rounded transition-colors duration-75 hover:text-primary-900 data-[popover=true]:text-primary-900"
              :class="[
                inspectionPtr?.id == blockPtr?.id || threadsByBlockId[blockPtr.id!]?.length
                  ? ''
                  : 'opacity-0 group-hover/block-line:opacity-100',
                threadsByBlockId[blockPtr.id!]?.length ? 'text-gray-700' : 'text-gray-400',
              ]"
              data-keep-inspection-in-base="true"
            >
              <i class="fas fa-message w-5 text-center" />
            </button>
          </div>
        </div>

        <!-- Footer -->
        <!-- Quick create -->
        <div
          v-if="page != null"
          class="group/footer mx-auto mb-8 mt-6 flex flex-row gap-x-1 rounded border border-gray-200 bg-white px-2 py-1"
        >
          <template v-for="blockType in EXPOSED_BLOCK_TYPES" :key="blockType">
            <button
              v-tooltip="{
                title: `Create ${toCamelName(BlockType, blockType)} Block`,
                showDelay: 200,
                hideDelay: 100,
                small: true,
                referenceMargin: 8,
              }"
              class="rounded px-2 py-1 text-base transition-colors duration-100 hover:bg-gray-100 hover:text-primary-900"
              :class="isFocusedAbsolute ? 'text-gray-600' : 'text-gray-400 group-hover/footer:text-gray-500'"
              @click="
                () => {
                  createAndFocusBlock({ type: blockType }, 'inside', page!);
                }
              "
            >
              <IconInline v-bind="ICON_BY_BLOCK_TYPE[blockType]" />
            </button>
          </template>
        </div>
      </div>
    </Scroll>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
