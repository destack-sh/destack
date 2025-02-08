<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { isDescendantOf } from "@/language/core/graph";
import { cloneNode, moveNode, NodeIn } from "@/language/core/node";
import { uploadFile } from "@/language/resource/file";
import { newChangeId } from "@/language/runtime/transaction";
import { createBlock } from "@/language/source/block";
import {
  BlockData,
  BlockType,
  NodeReferenceData,
  NodeType,
  Orientation,
  PageData,
  RectangleData,
  ViewData,
  type AnyNodeData,
} from "@/proto/wire/";
import { describeNode, isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { bench, canvas } from "@/system/space";
import { BLOCK_CONTEXT_ACTIONS, type ActionMapImplementation } from "@/ui/action";
import {
  isDragging,
  startDraggingIfAllowed,
  startSelectingIfAllowed,
  useMultiDropZone,
  useSelectionZone,
} from "@/ui/drag";
import { ICON_BY_BLOCK_TYPE, IconInline } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { useNodeListActions } from "@/ui/list";
import { pushDefaultMenu } from "@/ui/popover";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import { blurDocument } from "@/utils/element";
import { computedValue } from "@/utils/ref";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RootHeader from "@/views/builtins/RootHeader.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import TextBlockGroup from "@/views/builtins/TextBlockGroup.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Block from "@/views/nodes/Block.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const MIN_BLOCK_WIDTH = 500;
const MAX_BLOCK_WIDTH = 800;
const MIN_GUTTER_WIDTH = 60;
const BLOCK_GAP_Y = 4;

const props = defineProps<
  {
    self: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size: Required<Pick<RectangleData, "width" | "height">>;
    isRoot?: boolean;
  } & Pick<ViewData, "name" | "icon" | "nodePtr" | "focus" | "isMinimal" | "selection">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const nodePtr = computed(() => props.nodePtr);
const state = canvas.registerView(self, id);

const preparedConnection = useExistingConnection(nodePtr);
const { graph, connection } = preparedConnection;
const page = graph.getRef(nodePtr) as Ref<PageData | undefined>;
const blocks = graph.getChildrenRef(nodePtr, NodeType.BLOCK);

const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);
const blockRefs: Ref<Record<string, InstanceType<typeof Block>>> = ref({});
const contentRef = ref<HTMLElement | null>(null);
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);

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
// Layout
//

type BlockGroup =
  | {
      id: string;
      type: "text";
      blocks: BlockData[];
      beforeBlock: BlockData | undefined;
      afterBlock: BlockData | undefined;
    }
  | {
      id: string;
      type: "node";
      block: BlockData;
    };

const groups = computed(() => {
  const groups: BlockGroup[] = [];
  // accumulate successive text blocks into text groups
  for (const [i, block] of blocks.value.entries()) {
    const lastGroup = groups[groups.length - 1];
    if (block.type >= BlockType.PARAGRAPH) {
      if (lastGroup?.type === "text") {
        // expand text group
        lastGroup.blocks.push(block);
        lastGroup.afterBlock = blocks.value[i + 1];
      } else {
        groups.push({
          id: block.id,
          type: "text",
          blocks: [block],
          beforeBlock: blocks.value[i - 1],
          afterBlock: blocks.value[i + 1],
        });
      }
    } else {
      groups.push({ id: block.id, type: "node", block });
    }
  }
  return groups;
});

//
// Interaction
//

const HIGHLIGHTED_BLOCK_TYPES = [
  BlockType.PAGE,
  BlockType.FLOW,
  BlockType.DATABASE,
  BlockType.CHOICE,
];

// selecting
const selectionZone = useSelectionZone({ containerEl: contentRef, overlayEl: selectionOverlayRef });

// dragging
const { activeDropZone } = useMultiDropZone({
  name: "page",
  container: contentRef,
  targets: blockRefs,
  orientation: Orientation.VERTICAL,
  kinds: ["node", "selection", "file"],
  metatypes: [NodeType.BLOCK],
  fallbackToClosest: true,
  allowDrop: (dragged, anchor, targetId) => {
    if (dragged.kind == "file") return true;
    const target = targetId != null ? graph.get({ id: targetId }) : null;
    if (dragged.kind == "node" || dragged.kind == "selection") {
      return (
        target != null &&
        target.id != page.value?.id && // page block is also a block, but 'dropping' there is confusing (moves block outside of page)
        !dragged.nodes.some((node) => isDescendantOf(graph, target, node))
      );
    } else {
      return false;
    }
  },
  onDrop: (dragged, anchor, targetId, event) => {
    if (targetId == null) return; // need target
    if (dragged.kind == "file") {
      // create variable with file
      if (!dragged.files) return;
      const target = graph.getOrError({ id: targetId });
      if (!isNode(target, NodeType.BLOCK)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
      Array.from(dragged.files).forEach(async (file) => {
        if (bench.value == null) throw new Error("no current bench");
        const upload = uploadFile(() => connection.tx, file, { bench: bench.value });
        await upload.completion.wait();
        const block = createBlock(connection.tx, graph, {
          block: { type: BlockType.FILE, nodePtr: toNodeRef(upload.file.value!) },
          anchor: anchor == "start" ? "before" : "after",
          target: target,
        });
        focus(toNodeRef(block));
      });
    } else if (dragged.kind == "node") {
      const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
      let node = graph.getOrError(dragged.node);
      const target = graph.getOrError({ id: targetId });
      if (event.altKey) {
        // clone node before moving
        node = cloneNode(tx, graph, node, { keepProperties: true });
      }
      // move node
      moveNode(tx, graph, node, { anchor, target });
    } else if (dragged.kind == "selection") {
      // move nodes
      const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
      const target = graph.getOrError({ id: targetId });
      for (let i = 0; i < dragged.nodes.length; i++) {
        let node = graph.getOrError(dragged.nodes[i]);
        if (event.altKey) {
          // clone node before moving
          node = cloneNode(tx, graph, node, { keepProperties: true });
        }
        moveNode(tx, graph, node, {
          anchor: i == 0 ? anchor : "after",
          target: i == 0 ? target : graph.getOrError(dragged.nodes[i - 1]),
        });
      }
    }
  },
});

// actions
const actions: Partial<ActionMapImplementation<"list" | "space">> = {
  // edit
  "space.edit.rename": () => {
    nameRef.value?.focusIdentifier();
  },
  ...useNodeListActions({
    nodeType: NodeType.BLOCK,
    self: state.baseViewRef,
    graph: graph,
    list: blocks,
    txFactory: () => connection.tx,
    create: (anchor, node) =>
      createAndFocusBlock({ type: BlockType.PARAGRAPH }, node != null ? anchor : "inside", node ?? page.value!),
  }),
};
function createAndFocusBlock(
  blockIn: Partial<NodeIn<NodeType.BLOCK>>,
  anchor: "before" | "after" | "inside",
  target: PageData | BlockData,
) {
  const block = createBlock(connection.tx, graph, { block: blockIn, anchor, target });
  canvas.inspect({ node: block });
  if (block.type >= BlockType.PARAGRAPH) {
    nextTick(() => focus(block));
  } else if (block.type != BlockType.PAGE) {
    canvas.select([block]);
  } else {
    canvas.goToNode(block);
  }
  return block;
}

/** Focus or create text block at end of page. */
function focusText() {
  const lastBlock = blocks.value[blocks.value.length - 1];
  if (lastBlock.type >= BlockType.PARAGRAPH) {
    focus(lastBlock);
  } else {
    createAndFocusBlock({ type: BlockType.PARAGRAPH }, "after", lastBlock);
  }
}

// focus
// NOTE :UX: focus in Page should scroll into view but that sometimes pushes the root window out of frame somehow..
function focus(anchor?: FocusAnchor | NodeReferenceData | AnyNodeData) {
  let blockEl: InstanceType<typeof Block> | undefined;
  if (typeof anchor != "object") {
    if (anchor != "bottom") {
      blockEl = blockRefs.value[blocks.value[0].id!];
    } else {
      blockEl = blockRefs.value[blocks.value[blocks.value.length - 1].id!];
    }
  } else {
    if (anchor.id == nodePtr.value?.id) {
      // just focus first
      if (blocks.value.length > 0) {
        blockEl = blockRefs.value[blocks.value[0].id!];
      }
    } else {
      blockEl = blockRefs.value[anchor.id!];
    }
  }

  blurDocument(); // nothing to focus directly
  return blockEl?.$el;
}
const isFocusedAbsolute = canvas.isFocusedAbsoluteRef(self);

defineExpose<ViewExposed>({ self, actions, focus });
</script>
<template>
  <div class="flex w-full select-none flex-col bg-white text-gray-900" :class="[page ? '' : 'h-full']">
    <!-- Root header -->
    <RootHeader v-if="!isMinimal && isRoot" :self="self" :node-ptr="nodePtr" :focus="props.focus" :graph="graph">
      <template #meta>
        <button
          class="text-gray-400 hover:bg-gray-100 hover:text-gray-700"
          @click="(e) => pushDefaultMenu('main', page!, e)"
        >
          <i class="fas fa-ellipsis-vertical w-5 text-center" />
        </button>
      </template>
    </RootHeader>
    <!-- Page content -->
    <Scroll
      v-if="page"
      id="body"
      data-contextmenu-items="list.create.above,list.create.below,space.edit.paste"
      :size="{ width: size.width, height: size.height - (isRoot ? VIEW_DEFAULT_ROOT_HEADER_HEIGHT : 0) }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
      @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)"
    >
      <div ref="contentRef" class="flex min-h-full flex-col">
        <!-- Page header (title) -->
        <div
          class="mx-auto mb-5 mt-7 flex flex-row items-center rounded px-0.5"
          :style="{
            width: widths.block + 'px',
          }"
        >
          <NodeReference
            ref="nameRef"
            :orientation="Orientation.VERTICAL"
            :hide-icon="page.icon == null"
            size="title"
            :node="page"
            is-input
            :tx="() => connection.tx"
          />
        </div>

        <!-- Blocks -->
        <div v-for="(group, i) in groups" :key="group.type" class="group/block-group relative my-[3px]" :style="{}">
          <!-- Text block group -->
          <TextBlockGroup
            v-if="group.type == 'text'"
            :id="group.id"
            class="mx-auto rounded px-0.5"
            :blocks="group.blocks"
            :before-block="group.beforeBlock"
            is-input
            :after-block="group.afterBlock"
            :page="page"
            :connection="preparedConnection"
            :style="{
              width: widths.block + 'px',
            }"
          />
          <!-- Block -->
          <div
            v-else
            class="relative mx-auto rounded"
            :style="{
              width: widths.block + 'px',
            }"
          >
            <!-- Drag above/below -->
            <div
              v-if="activeDropZone?.targetId == group.block.id"
              class="absolute z-10 h-1 w-full rounded bg-gray-400"
              :style="getAnchorPositionStyle(activeDropZone?.anchor as 'start' | 'end', i, 4)"
            />
            <Block
              :id="group.block.id"
              :ref="(ref: any) => (ref ? (blockRefs[group.block.id!] = ref) : delete blockRefs[group.block.id!])"
              class="w-full"
              :class="isDragging(group.block) ? 'opacity-50' : ''"
              :node-ptr="toNodeRef(group.block)"
              :prepared-connection="preparedConnection"
              :container-gutter-width="widths.gutter"
              v-bind="state.getChildState(group.block.id)"
              :data-contextmenu-items="BLOCK_CONTEXT_ACTIONS.join(',')"
              :draggable="group.block.type == BlockType.PAGE"
              @dragstart.stop="(e) => startDraggingIfAllowed(e, group.block)"
            />
          </div>
        </div>

        <!-- Footer -->
        <div
          class="mx-auto my-4 flex flex-row justify-center gap-x-1.5"
          :style="{
            width: widths.block + 'px',
          }"
          @click="focusText()"
        >
          <!-- Add blocks -->
          <button
            v-for="blockType in HIGHLIGHTED_BLOCK_TYPES"
            data-suppress-drag="both"
            class="rounded-2xl border border-gray-200 px-2 py-0.5 text-gray-700 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-900"
            @click="() => createAndFocusBlock({ type: blockType as any }, 'inside', page!)"
          >
            <IconInline v-bind="ICON_BY_BLOCK_TYPE[blockType]" class="mr-1.5 w-5 text-center text-gray-700" />
            <span>{{ toCamelName(BlockType, blockType) }}</span>
          </button>
        </div>

        <!-- Padding -->
        <div class="h-[320px]" @click="focusText()" />

        <!-- Selection -->
        <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
      </div>
    </Scroll>
    <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="connection" />
  </div>
</template>
