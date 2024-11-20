<script lang="ts" setup>
import { BLOCK_CONTEXT_ACTIONS, createBlock, useFlatNodeMoveActions } from "@/language/block";
import { HEAVY_BLOCK_TYPES, toCamelName } from "@/language/const";
import { makeTypeInfo } from "@/language/field";
import { uploadFile } from "@/language/file";
import { isDescendantOf } from "@/language/graph";
import { cloneNode, moveNode, NodeIn } from "@/language/node";
import { packValue } from "@/language/value";
import {
  BenchType,
  BlockData,
  BlockType,
  NodeReferenceData,
  NodeType,
  Orientation,
  RectangleData,
  TypeKind,
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
import { runtime } from "@/system/runtime";
import { bench, canvas } from "@/system/space";
import { type ActionContext, type ActionMapImplementation } from "@/ui/action";
import { isDragging, startDraggingIfAllowed, useMultiDropZone } from "@/ui/drag";
import { ICON_BY_BLOCK_TYPE, IconInline } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { menuActionsLike, type PopoverContext, type PopoverInfo, type PopoverInfoIn } from "@/ui/popover";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { blurDocument } from "@/utils/element";
import { computedValue } from "@/utils/ref";
import HistoryNavigator from "@/views/builtins/HistoryNavigator.vue";
import IconName from "@/views/builtins/IconName.vue";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodePath from "@/views/builtins/NodePath.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Block from "@/views/system/Block.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MIN_BLOCK_WIDTH = 500;
const MAX_BLOCK_WIDTH = 800;
const MIN_GUTTER_WIDTH = 60;
const BLOCK_GAP_Y = 8;

const props = defineProps<
  {
    self: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size: Required<Pick<RectangleData, "width" | "height">>;
  } & Pick<ViewData, "name" | "icon" | "nodePtr" | "focus" | "variant" | "selection">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr));
const state = canvas.registerView(self, id);

const { graph: spaceGraph } = useExistingConnection(self);
const selfView = spaceGraph.getRef(self);
const preparedPkgConnection = useGetConnection(
  { name: `page.${nodePtr.value?.id}` },
  computed(() => ({
    scope: PACKAGE_SCOPE.value,
    roots: [nodePtr.value!],
    descendantTypes: [NodeType.BLOCK],
    isEnabled: nodePtr.value != null,
  })),
);
const { graph: pkgGraph, connection: pkgConnection } = preparedPkgConnection;
const page = pkgGraph.getRef(nodePtr) as Ref<BlockData | undefined>;
const blocks = pkgGraph.getChildrenRef(nodePtr, NodeType.BLOCK);

const historyRef: Ref<InstanceType<typeof HistoryNavigator> | null> = ref(null);
const blockRefs: Ref<Record<string, InstanceType<typeof Block>>> = ref({});
const contentRef = ref<HTMLElement | null>(null);
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);

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
  targets: blockRefs,
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
      if (!dragged.files) return;
      const target = pkgGraph.getOrError({ id: targetId });
      if (!isNode(target, NodeType.BLOCK)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
      Array.from(dragged.files).forEach(async (file) => {
        if (bench.value == null) throw new Error("no current bench");
        const upload = uploadFile(() => pkgConnection.tx, file, { bench: bench.value });
        await upload.completion.wait();
        const variableType = makeTypeInfo({ kind: TypeKind.NODE, benchType: BenchType.FILE });
        const block = createBlock(pkgConnection.tx, pkgGraph, {
          block: {
            type: BlockType.VALUE,
            subnode: {
              valueType: variableType,
              valuePacked: packValue(toNodeRef(upload.file.value!), variableType),
            },
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
  "common.create.above": (action, context) => {
    let { block } = getBlockFromContext(context);
    if (block == null) block = blocks.value[0];
    if (block == null) return false;
    createAndFocusBlock({ type: BlockType.TEXT }, "before", block);
  },
  "common.create.below": (action, context) => {
    let { block } = getBlockFromContext(context);
    if (block == null) block = blocks.value[blocks.value.length - 1];
    if (block == null) return false;
    createAndFocusBlock({ type: BlockType.TEXT }, "after", block);
  },
  // edit
  "common.edit.duplicate": (action, context) => {
    const { block } = getBlockFromContext(context);
    if (block == null) return false;
    const duplicate = cloneNode(pkgConnection.tx, pkgGraph, block, { includeChildren: true });
    nextTick(() => focus(duplicate));
  },
  "common.edit.delete": (action, context) => {
    const { block } = getBlockFromContext(context);
    if (block == null) return false;
    pkgConnection.tx.delete(block);
  },
  // navigation
  "common.navigate.up": (action, context) => {
    const { block, idx } = getBlockFromContext(context);
    let toFocus = blocks.value[idx - 1];
    if (block == null) {
      if (nodePtr.value?.id == focusedNodePtr.value?.id) toFocus = blocks.value[blocks.value.length - 1];
      else return false;
    }
    if (toFocus != null) canvas.focus({ node: toFocus, view: self.value });
  },
  "common.navigate.down": (action, context) => {
    const { block, idx } = getBlockFromContext(context);
    let toFocus = blocks.value[idx + 1];
    if (block == null) {
      if (nodePtr.value?.id == focusedNodePtr.value?.id) toFocus = blocks.value[0];
      else return false;
    }
    if (toFocus != null) canvas.focus({ node: toFocus, view: self.value });
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
  "session.run.start": (action, context) => {
    const { block } = getBlockFromContext(context);
    if (block == null) return false;
    runtime.createRun(block);
  },
};
function createAndFocusBlock(
  blockIn: Partial<NodeIn<NodeType.BLOCK>> & Required<Pick<NodeIn<NodeType.BLOCK>, "type">>,
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
      blockEl = blockRefs.value[blocks.value[0].id!];
      blockEl?.$el?.scrollIntoView({ block: "start", behavior: "instant" });
    } else {
      blockEl = blockRefs.value[blocks.value[blocks.value.length - 1].id!];
      blockEl?.$el?.scrollIntoView({ block: "end", behavior: "instant" });
    }
  } else {
    if (anchor.id == nodePtr.value?.id) {
      // just focus first
      if (blocks.value.length > 0) {
        blockEl = blockRefs.value[blocks.value[0].id!];
        blockEl.$el?.scrollIntoView({ block: "nearest", behavior: "instant" });
      }
    } else {
      blockEl = blockRefs.value[anchor.id!];
      blockEl?.$el?.scrollIntoView({ block: "nearest", behavior: "instant" });
    }
  }

  blurDocument(); // nothing to focus directly
  return blockEl?.$el;
}
const isFocusedAbsolute = canvas.isFocusedAbsoluteRef(self);

defineExpose<ViewExposed>({ self, actions, focus });
</script>
<template>
  <div v-if="page" class="flex w-full select-none flex-col bg-white text-gray-900">
    <!-- Meta header -->
    <div
      data-keep-inspection-in-base-view="true"
      class="group flex w-full max-w-full flex-row items-center px-2"
      :style="{ height: (historyRef?.isActive ? VIEW_DEFAULT_BAR_HEADER_HEIGHT : HEADER_HEIGHT) + 'px' }"
    >
      <!-- History -->
      <HistoryNavigator ref="historyRef" :self="self" />
      <!-- Breadcrumb -->
      <NodePath
        v-if="nodePtr"
        :container="nodePtr"
        :focus="$props.focus?.nodesPtr[0]"
        :self="nodePtr"
        :graph="pkgGraph"
      />
      <!-- Meta & Controls -->
      <div class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-1.5 pl-1">
        <!-- ... -->
        <button
          v-menu="
            (): PopoverInfo => ({
              kind: 'menu',
              placement: 'bottom-left',
              offset: 'referenceWidth',
              items: menuActionsLike(BLOCK_CONTEXT_ACTIONS, { context: { triggerNode: page } }),
            })
          "
          class="text-gray-400 hover:bg-gray-100 hover:text-gray-700"
        >
          <i class="fas fa-ellipsis w-5 text-center" />
        </button>
      </div>
    </div>

    <!-- Page content -->
    <Scroll
      id="body"
      v-contextmenu="
        (context: PopoverContext): PopoverInfo => ({
          kind: 'menu',
          placement: 'bottom-right',
          items: menuActionsLike(
            ['common.create.above', 'common.create.below', 'common.edit.paste', 'message.chat.message'],
            { context: { ...context, triggerNode: page } },
          ),
        })
      "
      :size="{ width: size.width, height: size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
    >
      <div ref="contentRef" class="mb-[320px] flex min-h-full flex-col">
        <!-- Page header (title) -->
        <div
          class="mx-auto mb-2 mt-5 flex flex-row items-center"
          :style="{
            width: widths.block + 'px',
          }"
        >
          <!-- Icon -->
          <IconName ref="iconNameRef" size="title" :node="page" :tx="() => pkgConnection.tx" />
        </div>

        <!-- Blocks -->
        <div
          v-for="(block, i) in blocks"
          :key="block.id"
          class="group/block-line relative flex min-w-fit flex-row"
          :style="{
            paddingTop: HEAVY_BLOCK_TYPES.includes(block.type) ? '10px' : undefined,
            marginTop: BLOCK_GAP_Y + 'px',
          }"
        >
          <!-- Left gutter -->
          <div
            class="relative flex flex-shrink-0 flex-row items-start justify-end gap-x-1 px-1 text-right opacity-0 transition-colors duration-150 group-hover/block-line:opacity-100"
            :class="HEAVY_BLOCK_TYPES.includes(block.type) ? 'pt-2' : 'pt-1'"
            :style="{ width: widths.gutter + 'px' }"
          >
            <!-- Create above / below -->
            <button
              v-menu="
                (): PopoverInfoIn => ({
                  component: ViewType.PICKER,
                  placement: 'bottom',
                  props: { valueType: makeTypeInfo({ benchType: BenchType.BLOCK_TYPE, isRequired: true }) },
                  onApply: (blockType: BlockType) => createAndFocusBlock({ type: blockType }, 'after', block),
                })
              "
              class="ml-2 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
              :style="{}"
            >
              <i class="fas fa-plus" />
            </button>
            <!-- Drag -->
            <button
              v-menu="
                (): PopoverInfo => ({
                  kind: 'menu',
                  placement: 'bottom-left',
                  offset: 'referenceWidth',
                  items: menuActionsLike(BLOCK_CONTEXT_ACTIONS, { context: { triggerNode: nodePtr } }),
                })
              "
              class="text-gray-400 hover:bg-gray-100 hover:text-gray-700"
              :draggable="true"
              @dragstart.stop="(e) => startDraggingIfAllowed(e, pkgGraph, block)"
            >
              <i class="fas fa-grip-vertical w-5 text-center" />
            </button>
          </div>

          <!-- Block wrapper -->
          <div
            class="group/block-wrapper relative rounded"
            :style="{
              width: widths.block + 'px',
            }"
          >
            <!-- Drag above/below -->
            <div
              v-if="activeDropZone?.targetId == block.id"
              class="absolute z-10 h-1 w-full rounded bg-primary-500"
              :style="getAnchorPositionStyle(activeDropZone?.anchor as 'start' | 'end', i, 4)"
            />

            <!-- Block -->
            <Block
              :id="block.id"
              :ref="(ref: any) => (ref ? (blockRefs[block.id!] = ref) : delete blockRefs[block.id!])"
              v-contextmenu="
                (context: PopoverContext): PopoverInfo => ({
                  kind: 'menu',
                  placement: 'bottom-right',
                  items: menuActionsLike(BLOCK_CONTEXT_ACTIONS, { context: { ...context, triggerNode: block } }),
                })
              "
              class="w-full"
              :class="isDragging(block) ? 'opacity-50' : ''"
              :node-ptr="toNodeRefOneOf(block)"
              :prepared-connection="preparedPkgConnection"
              v-bind="state.getChildState(block.id)"
              :draggable="block.type == BlockType.PAGE"
              @dragstart.stop="(e) => startDraggingIfAllowed(e, pkgGraph, block)"
            />
          </div>
        </div>

        <!-- Footer -->
        <div
          class="mx-auto mt-8 flex flex-row justify-center gap-x-1.5"
          :style="{
            width: widths.block + 'px',
          }"
        >
          <!-- Add blocks -->
          <button
            v-for="blockType in [BlockType.TEXT, BlockType.CHOICE, BlockType.DATABASE, BlockType.FLOW, BlockType.PAGE]"
            class="rounded-2xl border border-gray-200 px-2 text-gray-700 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-900"
            @click="() => createAndFocusBlock({ type: blockType }, 'inside', page!)"
          >
            <IconInline v-bind="ICON_BY_BLOCK_TYPE[blockType]" class="mr-1.5 w-5 text-center text-gray-700" />
            <span>{{ toCamelName(BlockType, blockType) }}</span>
          </button>
        </div>
      </div>
    </Scroll>
  </div>
  <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="pkgConnection" />
</template>
