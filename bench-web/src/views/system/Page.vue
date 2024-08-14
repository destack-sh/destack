<script lang="ts" setup>
import { createBlock, useFlatNodeMoveActions } from "@/language/block";
import { toCamelName } from "@/language/const";
import { uploadFile } from "@/language/file";
import { getGroupedChildrenRef, isDescendantOf } from "@/language/graph";
import { makeRun } from "@/language/session";
import { cloneNode, moveNode } from "@/language/node";
import { packValueJson } from "@/language/value";
import {
  BenchType,
  BlockData,
  BlockType,
  BoxData,
  NodeReferenceData,
  NodeType,
  Orientation,
  TypeKind,
  Variant,
  ViewData,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire/";
import {
  describeNode,
  isNode,
  toNodeRef,
  toNodeRefOneOf,
  unwrapProtoOneOf,
  type TypedNodeReferenceData,
} from "@/proto/wiring";
import { PACKAGE_SCOPE } from "@/system/client";
import { useExistingConnection, useGetConnection } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import { fireActionById, type ActionContext, type ActionMapImplementation } from "@/ui/action";
import { DEFAULT_HEADER_HEIGHT } from "@/ui/canvas";
import { startDragging, useMultiDropZone } from "@/ui/drag";
import { ICON_BY_BLOCK_TYPE, IconInline } from "@/ui/icon";
import { EXPOSED_BLOCK_TYPES } from "@/ui/inspect";
import { ScrollbarWidth } from "@/ui/layout";
import { menuActionsLike, type PopoverContext, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import { blurDocument } from "@/utils/element";
import { computedValue } from "@/utils/ref";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodePath from "@/views/builtins/NodePath.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Block from "@/views/system/Block.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";
import { makeTypeInfo } from "@/language/field";

const HEADER_HEIGHT = DEFAULT_HEADER_HEIGHT;
const MIN_BLOCK_WIDTH = 500;
const MAX_BLOCK_WIDTH = 800;
const MIN_GUTTER_WIDTH = 44;
const BLOCK_GAP_Y = 8;
const HANDLE_WIDTH = 6;
const SEPARATOR_WIDTH = 6;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "text" | "icon" | "nodePtr" | "focus" | "variant" | "selection" | "expansion"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr));

const { graph: spaceGraph } = useExistingConnection(self);
const selfView = spaceGraph.getRef(self);
const preparedPkgConnection = useGetConnection(
  { name: `page.${nodePtr.value?.id}` },
  computed(() => ({
    scope: PACKAGE_SCOPE.value,
    roots: [nodePtr.value!],
    options: { descendantTypes: [NodeType.BLOCK] },
    isEnabled: nodePtr.value != null,
  })),
);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const page = pkgGraph.getRef(nodePtr) as Ref<BlockData | undefined>;
const blocks = pkgGraph.getChildrenRef(nodePtr, NodeType.BLOCK);
const blocksWithSelf: Ref<BlockData[]> = computed(() => {
  if (page.value == null) {
    return [];
  } else {
    return [page.value, ...blocks.value];
  }
});
const expandedBlockRefs: Ref<Record<string, InstanceType<typeof Block>>> = ref({});
const contentRef = ref<HTMLElement | null>(null);
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);

// messages / notices
const { childrenByParentId: threadsByBlockId } = getGroupedChildrenRef({
  graph: pkgGraph,
  parentPtrs: blocksWithSelf,
  childTypes: [NodeType.MESSAGE],
});

// size block/gutter horizontally (try to fit both until min block width)
const widths = computed(() => {
  // divide space between block and gutter up to target gutter width
  const blockWidth = Math.min(MAX_BLOCK_WIDTH, Math.max(MIN_BLOCK_WIDTH, props.size.width - MIN_GUTTER_WIDTH * 2));
  const gutterWidth = Math.max(MIN_GUTTER_WIDTH, (props.size.width - blockWidth) / 2);
  return { block: blockWidth, gutter: gutterWidth };
});

/** Gets the position for a div anchored at the start/end of the given block */
function getAnchorPositionStyle(anchor: "start" | "end", blockIdx: number, anchorWidth: number) {
  if (anchor == "start") {
    return {
      top: -BLOCK_GAP_Y / 2 - anchorWidth / 2 + "px",
    };
  } else {
    return {
      bottom: -BLOCK_GAP_Y / 2 - anchorWidth / 2 + "px",
    };
  }
}

//
// Interaction
//

// dragging
const { activeDropZone } = useMultiDropZone({
  name: "page",
  container: contentRef,
  targets: expandedBlockRefs,
  orientation: Orientation.VERTICAL,
  kinds: ["node", "file"],
  metatypes: [NodeType.BLOCK],
  fallbackToClosest: true,
  allowDrop: (dragged, anchor, targetId) => {
    const target = targetId != null ? pkgGraph.get({ id: targetId }) : null;
    return (
      dragged.kind == "file" ||
      (dragged.kind == "node" &&
        target != null &&
        target.id != page.value?.id && // page block is also a block, but 'dropping' there is confusing (moves block outside of page)
        !isDescendantOf(pkgGraph, target, dragged.node))
    );
  },
  onDrop: (dragged, anchor, targetId) => {
    if (targetId == null) return; // need target
    if (dragged.kind == "file") {
      // create variable with file
      const container = page.value;
      if (container == null || !dragged.files) return;
      const target = pkgGraph.getOrError({ id: targetId });
      if (!isNode(target, NodeType.BLOCK)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
      Array.from(dragged.files).forEach(async (file) => {
        const upload = uploadFile(() => pkgConnection.tx, file, { parent: container });
        await upload.completion.wait();
        const variableType = makeTypeInfo({ kind: TypeKind.NODE, benchType: BenchType.FILE });
        const block = createBlock(pkgConnection.tx, pkgGraph, {
          block: {
            type: BlockType.VARIABLE,
            valueType: variableType,
            valuePacked: packValueJson(toNodeRef(upload.file.value!), variableType),
          },
          anchor: anchor == "start" ? "before" : "after",
          target: target,
        });
        focus(toNodeRef(block));
      });
    } else if (dragged.kind == "node") {
      // move node
      const target = pkgGraph.getOrError({ id: targetId });
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor, target });
    }
  },
});

// actions
const getBlockFromContext = (ctx: ActionContext | undefined): { block: BlockData | null; idx: number } => {
  let blockIdx: number | undefined = undefined;
  if (blockIdx === undefined && ctx?.triggerNode?.id != null)
    blockIdx = blocks.value.findIndex((block) => block.id == ctx!.triggerNode!.id);
  if (blockIdx === undefined && focusedNodePtr.value?.id != null)
    blockIdx = blocks.value.findIndex((block) => block.id == focusedNodePtr.value!.id);
  if (blockIdx === undefined) return { block: null, idx: -1 };
  const block = blocks.value[blockIdx];
  return { block, idx: blockIdx };
};
const actions: Partial<ActionMapImplementation<"common" | "session">> = {
  // create
  "common.create.above": {
    action: (action, context) => {
      let { block } = getBlockFromContext(context);
      if (block == null) block = blocks.value[0];
      if (block == null) return false;
      createAndFocusBlock({ type: BlockType.TEXT }, "before", block);
    },
  },
  "common.create.below": {
    action: (action, context) => {
      let { block } = getBlockFromContext(context);
      if (block == null) block = blocks.value[blocks.value.length - 1];
      if (block == null) return false;
      createAndFocusBlock({ type: BlockType.TEXT }, "after", block);
    },
  },
  // edit
  "common.edit.duplicate": {
    action: (action, context) => {
      const { block } = getBlockFromContext(context);
      if (block == null) return false;
      const duplicate = cloneNode(pkgConnection.tx, pkgGraph, block, { includeChildren: true });
      nextTick(() => focus(duplicate));
    },
  },
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
      pkgConnection.tx.delete(block);
    },
  },
  // navigation
  "common.navigate.up": {
    action: (action, context) => {
      const { block, idx } = getBlockFromContext(context);
      let toFocus = blocks.value[idx - 1];
      if (block == null) {
        if (nodePtr.value?.id == focusedNodePtr.value?.id) toFocus = blocks.value[blocks.value.length - 1];
        else return false;
      }
      if (toFocus != null) canvas.focus({ node: toFocus, view: self.value });
    },
  },
  "common.navigate.down": {
    action: (action, context) => {
      const { block, idx } = getBlockFromContext(context);
      let toFocus = blocks.value[idx + 1];
      if (block == null) {
        if (nodePtr.value?.id == focusedNodePtr.value?.id) toFocus = blocks.value[0];
        else return false;
      }
      if (toFocus != null) canvas.focus({ node: toFocus, view: self.value });
    },
  },
  // move
  ...useFlatNodeMoveActions({
    graph: pkgGraph,
    txFactory: () => pkgConnection.tx,
    getNodeFromContext: (context) => {
      const { block, idx } = getBlockFromContext(context);
      return { node: block, idx };
    },
  }),
  // session
  "session.run.start": {
    action: (action, context) => {
      const { block } = getBlockFromContext(context);
      if (block == null) return false;
      pkgConnection.tx.create(makeRun(block, pkgGraph));
    },
  },
};
function createAndFocusBlock(
  blockIn: { type: BlockType } & Partial<BlockData>,
  anchor: "before" | "after" | "inside",
  target: BlockData | TypedNodeReferenceData<NodeType.BLOCK>,
) {
  const block = createBlock(pkgConnection.tx, pkgGraph, { block: blockIn, anchor, target });
  nextTick(() => focus(block));
}

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData | AnyNodeData) {
  let blockEl: InstanceType<typeof Block> | undefined;
  if (typeof anchor != "object") {
    if (anchor != "bottom") {
      blockEl = expandedBlockRefs.value[blocks.value[0].id!];
      blockEl?.$el?.scrollIntoView({ block: "start", behavior: "instant" });
    } else {
      blockEl = expandedBlockRefs.value[blocks.value[blocks.value.length - 1].id!];
      blockEl?.$el?.scrollIntoView({ block: "end", behavior: "instant" });
    }
  } else {
    if (anchor.id == nodePtr.value?.id) {
      // just focus first
      if (blocks.value.length > 0) {
        blockEl = expandedBlockRefs.value[blocks.value[0].id!];
        blockEl.$el?.scrollIntoView({ block: "nearest", behavior: "instant" });
      }
    } else {
      blockEl = expandedBlockRefs.value[anchor.id!];
      blockEl?.$el?.scrollIntoView({ block: "nearest", behavior: "instant" });
    }
  }

  blurDocument(); // nothing to focus directly
  return blockEl?.$el;
}
const isFocusedAbsolute = canvas.isFocusedAbsoluteRef(self);

canvas.registerView(self);
defineExpose<ViewExposed>({ self, actions, focus });
</script>
<template>
  <div v-if="page" class="flex w-full flex-col bg-white text-gray-900">
    <!-- TODO :UX: entire Page/Block/.. design -->
    <!-- (should it be more notebook like or more page like? where should extra block interactions & metadata go?,
          what about a line with multiple block 'columns' in it? how to navigate around these blocks? ...) -->

    <!-- Header -->
    <div
      data-keep-inspection-in-base="true"
      class="group flex w-full max-w-full flex-row pl-2 pr-3"
      :style="{ height: HEADER_HEIGHT + 'px' }"
    >
      <!-- Breadcrumb -->
      <NodePath :container="nodePtr" :focus="$props.focus?.nodesPtr[0]" :graph="pkgGraph" />
      <!-- Meta & Controls -->
      <div class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-1.5 pl-1">
        <!-- Search -->
        <button class="h-fit text-gray-400 hover:text-primary-900" @click="fireActionById('common.search.findInView')">
          <i class="fas fa-magnifying-glass w-5 text-center" />
        </button>
      </div>
    </div>

    <!-- Page content -->
    <Scroll
      v-contextmenu="
        (context: PopoverContext): PopoverInfo => ({
          kind: 'menu',
          placement: 'bottom-right',
          items: menuActionsLike(
            ['common.create.above', 'common.create.below', 'common.edit.paste', 'message.chat.message'],
            {
              context: { ...context, triggerNode: page },
            },
          ),
        })
      "
      :size="{ width: size.width, height: size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
    >
      <div ref="contentRef" class="mb-16 flex min-h-full flex-col">
        <!-- Block full width line -->
        <template v-for="(block, i) in blocksWithSelf" :key="block.id">
          <div
            class="group/block-line relative flex min-w-fit flex-row"
            :style="{
              marginTop: BLOCK_GAP_Y + 'px',
            }"
          >
            <!-- Left gutter -->
            <div
              class="relative flex flex-shrink-0 flex-row items-start justify-end gap-x-2 text-right"
              :style="{ width: widths.gutter + 'px', marginTop: SEPARATOR_WIDTH + 'px' }"
            >
              <!-- Activity / Run / ... -->
              <!-- Run -->
              <button
                v-if="expandedBlockRefs[block.id!]?.isRunnable"
                class="text-gray-400 hover:text-primary-900"
                :class="inspectionPtr?.id == block?.id ? '' : 'opacity-0 group-hover/block-line:opacity-100'"
                data-keep-inspection-in-base="true"
                @click="() => pkgConnection.tx.create(makeRun(block, pkgGraph))"
              >
                <i class="fas fa-play" />
              </button>
              <!-- Handle -->
              <div
                class="h-full rounded transition-colors duration-75"
                :class="
                  inspectionPtr?.id == block?.id
                    ? 'bg-primary-900'
                    : focusedNodePtr?.id == block?.id
                      ? 'bg-gray-300'
                      : 'bg-transparent group-hover/block-line:bg-gray-200'
                "
                :style="{ width: HANDLE_WIDTH + 'px' }"
              />
            </div>

            <!-- Block wrapper -->
            <div
              class="group/block-wrapper relative rounded"
              :style="{
                width: widths.block + 'px',
              }"
            >
              <!-- Separator: create above/below (in between and around blocks) -->
              <div
                v-for="anchor in i == 0 ? [] : i < blocks.length - 1 ? ['start', 'end'] : ['start', 'end']"
                :key="anchor"
                v-menu="
                  (): PopoverInfoIn => ({
                    component: ViewType.PICKER,
                    placement: 'bottom',
                    props: { valueType: makeTypeInfo({ benchType: BenchType.BLOCK_TYPE, isRequired: true }) },
                    onApply: (blockType: BlockType) =>
                      createAndFocusBlock({ type: blockType }, anchor == 'start' ? 'before' : 'after', block),
                  })
                "
                role="button"
                class="absolute w-full flex-shrink-0 text-center text-gray-200 opacity-0 transition-colors duration-75 hover:z-10 hover:text-gray-300 hover:opacity-100 data-[popover=true]:text-primary-900 data-[popover=true]:opacity-100"
                :style="{
                  ...getAnchorPositionStyle(anchor as 'start' | 'end', i, SEPARATOR_WIDTH),
                  height: `${SEPARATOR_WIDTH}px`,
                }"
                data-keep-inspection-in-base="true"
              >
                <!-- Line with a gap for the button -->
                <div class="relative">
                  <svg class="translate-y-1" width="100%" height="1px" viewBox="0 0 100 1" preserveAspectRatio="none">
                    <path d="M0,0.5 L48.5,0.5" fill="none" stroke="currentColor" stroke-width="1" />
                    <path d="M100,0.5 L51.5,0.5" fill="none" stroke="currentColor" stroke-width="1" />
                  </svg>
                  <i class="fas fa-plus -translate-y-[6px] px-1" />
                </div>
              </div>

              <!-- Drag above/below -->
              <div
                v-if="activeDropZone?.targetId == block.id"
                class="absolute z-10 h-1 w-full rounded-sm bg-primary-900"
                :style="getAnchorPositionStyle(activeDropZone?.anchor as 'start' | 'end', i, 4)"
              />

              <!-- Block -->
              <Block
                :ref="(ref: any) => (ref ? (expandedBlockRefs[block.id!] = ref) : delete expandedBlockRefs[block.id!])"
                v-contextmenu="
                  (context: PopoverContext): PopoverInfo => ({
                    kind: 'menu',
                    placement: 'bottom-right',
                    items: menuActionsLike(
                      [
                        'common.edit.rename',
                        'common.edit.morph',
                        'common.edit.duplicate',
                        'common.edit.archive',
                        'common.edit.delete',
                        'session.run.start',
                        'message.chat.message',
                      ],
                      { context: { ...context, triggerNode: block } },
                    ),
                  })
                "
                class="w-full px-2 py-1.5 data-[dragging=true]:opacity-50"
                borderless
                :variant="Variant.STEALTH"
                :node-ptr="toNodeRefOneOf(block)"
                :prepared-connection="preparedPkgConnection"
                :draggable="true"
                @dragstart.stop="(e: DragEvent) => startDragging(e, pkgGraph, block)"
              />
            </div>

            <!-- Right gutter -->
            <div
              class="relative flex flex-shrink-0 flex-row items-start justify-start px-0.5 transition-colors duration-75"
              :style="{ width: widths.gutter, marginTop: SEPARATOR_WIDTH + 'px' }"
            >
              <!-- Messages -->
              <button
                v-if="false /* NOTE :Incomplete: Messages */"
                v-menu="
                  (context: PopoverContext): PopoverInfoIn => ({
                    component: ViewType.CHAT,
                    placement: 'bottom',
                    container: 'containingRoot',
                    containerMargin: 12,
                    props: {
                      variant: Variant.COMPACT,
                      nodePtr: toNodeRefOneOf(threadsByBlockId[block.id!]?.at(-1)!) ?? block,
                    },
                  })
                "
                class="rounded transition-colors duration-75 hover:text-primary-900 data-[popover=true]:text-primary-900"
                :class="[
                  inspectionPtr?.id == block?.id || threadsByBlockId[block.id!]?.length
                    ? ''
                    : 'opacity-0 group-hover/block-line:opacity-100',
                  threadsByBlockId[block.id!]?.length ? 'text-gray-700' : 'text-gray-400',
                ]"
                data-keep-inspection-in-base="true"
              >
                <i class="fas fa-message w-5 text-center" />
              </button>
            </div>
          </div>

          <!-- Top block spacer (top block == page) -->
          <div v-if="i == 0" class="mx-auto my-3 w-full px-2" :style="{ width: widths.block + 'px' }">
            <div class="h-[1px] w-full bg-gray-200" />
          </div>
        </template>

        <!-- Footer -->
        <!-- Quick create -->
        <div
          v-if="page != null"
          class="group/footer mx-auto mb-8 mt-6 flex flex-row gap-x-1 rounded border border-gray-200 bg-white px-2 py-1"
          data-keep-inspection-in-base="true"
        >
          <template v-for="blockType in EXPOSED_BLOCK_TYPES" :key="blockType">
            <button
              v-tooltip="{
                title: `${toCamelName(BlockType, blockType)}`,
                showDelay: 200,
                hideDelay: 100,
                small: true,
                referenceMargin: 8,
                group: 'page.footer',
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
  <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="pkgConnection" />
</template>
