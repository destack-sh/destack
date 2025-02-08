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
import { computedValue } from "@/utils/ref";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import PageHeader from "@/views/builtins/PageHeader.vue";
import RootHeader from "@/views/builtins/RootHeader.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import TextBlockGroup from "@/views/builtins/TextBlockGroup.vue";
import { NavigationDirection, type FocusAnchor, type ViewEmits, type ViewExposed } from "@/views/common";
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
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const nodePtr = computed(() => props.nodePtr);
const state = canvas.registerView(self, id);

const preparedConnection = useExistingConnection(nodePtr);
const { graph, connection } = preparedConnection;
const page = graph.getRef(nodePtr) as Ref<PageData | undefined>;
const blocks = graph.getChildrenRef(nodePtr, NodeType.BLOCK);

const nodeBlockRefs: Ref<Record<string, InstanceType<typeof Block>>> = ref({});
const pageHeaderRef: Ref<InstanceType<typeof PageHeader> | null> = ref(null);
const textBlockGroupRefs: Ref<Record<string, InstanceType<typeof TextBlockGroup>>> = ref({});
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
      blocks: BlockData[];
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
      groups.push({ id: block.id, type: "node", block, blocks: [block] });
    }
  }
  return groups;
});
const isEmpty = computed(
  () => blocks.value.length == 0 || blocks.value.every((b) => b.type >= BlockType.PARAGRAPH && b.text == null),
);
function getGroupRef(group: BlockGroup) {
  if (group.type == "text") {
    return textBlockGroupRefs.value[group.id!];
  } else {
    return nodeBlockRefs.value[group.block.id!];
  }
}

//
// Interaction
//

const HIGHLIGHTED_BLOCK_TYPES = [BlockType.PAGE, BlockType.FLOW, BlockType.DATABASE, BlockType.CHOICE];

// selecting
const selectionZone = useSelectionZone({ containerEl: contentRef, overlayEl: selectionOverlayRef });

// dragging
const { activeDropZone } = useMultiDropZone({
  name: "page",
  container: contentRef,
  targets: nodeBlockRefs,
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
    pageHeaderRef.value?.focusIdentifier("left");
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
function focusText(anchor: "top" | "bottom" = "bottom") {
  if (anchor == "top") {
    const firstBlock = blocks.value[0];
    if (firstBlock.type >= BlockType.PARAGRAPH) {
      focus(firstBlock);
    } else {
      createAndFocusBlock({ type: BlockType.PARAGRAPH }, "before", firstBlock);
    }
  } else {
    const lastBlock = blocks.value[blocks.value.length - 1];
    if (lastBlock.type >= BlockType.PARAGRAPH) {
      focus(lastBlock);
    } else {
      createAndFocusBlock({ type: BlockType.PARAGRAPH }, "after", lastBlock);
    }
  }
}

// navigate
function navigateFromGroup(group: BlockGroup, direction: NavigationDirection) {
  const navigableGroups = groups.value.filter((g) => g.type == "text");
  const groupIdx = navigableGroups.findIndex((g) => g.id == group.id);
  console.log("navigateFromGroup", { group, groupIdx, direction });
  if (direction == "up" || direction == "left") {
    const prevGroup = navigableGroups[groupIdx - 1];
    if (prevGroup != null) {
      const prevGroupRef = getGroupRef(prevGroup);
      prevGroupRef?.focus?.("bottom");
    } else {
      pageHeaderRef.value?.focusIdentifier("right");
    }
  } else if (direction == "down" || direction == "right") {
    const nextGroup = navigableGroups[groupIdx + 1];
    if (nextGroup != null) {
      const nextGroupRef = getGroupRef(nextGroup);
      nextGroupRef?.focus?.("top");
    } else {
      focusText("bottom");
    }
  }
}

function deleteGroup(group: BlockGroup) {
  navigateFromGroup(group, "up");
  const tx = connection.tx.with({ change: { key: newChangeId(), title: "Delete" } });
  group.blocks.forEach((b) => tx.delete(b));
}

// focus
// NOTE :UX: focus in Page should scroll into view but that sometimes pushes the root window out of frame somehow..
function focus(anchor?: FocusAnchor | NodeReferenceData | AnyNodeData) {
  let block: BlockData | undefined;
  if (typeof anchor != "object") {
    if (anchor != "bottom") {
      block = blocks.value[0];
    } else {
      block = blocks.value[blocks.value.length - 1];
    }
  } else {
    block = graph.get(anchor) as BlockData | undefined;
  }

  if (block == null) {
    // just focus page
    focusText("bottom");
  } else {
    // focus containing group
    if (block.type >= BlockType.PARAGRAPH) {
      const group = groups.value.find((g) => g.type == "text" && g.blocks.some((b) => b.id == block.id));
      const groupRef = textBlockGroupRefs.value[group!.id!];
      groupRef?.focus?.();
    } else {
      const groupRef = nodeBlockRefs.value[block.id!];
      groupRef?.focus?.();
    }
  }
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
      @mousedown="(e: MouseEvent) => startSelectingIfAllowed(selectionZone, e)"
    >
      <div ref="contentRef" class="flex min-h-full flex-col">
        <!-- Page header (title) -->
        <PageHeader
          ref="pageHeaderRef"
          :width="widths.block"
          :node="page"
          :connection="preparedConnection"
          is-input
          @navigate="
            (direction) => {
              if (direction == 'right' || direction == 'down') {
                focusText('top');
              }
            }
          "
        />

        <!-- Blocks -->
        <div v-for="(group, i) in groups" :key="group.type" class="group/block-group relative my-[3px]" :style="{}">
          <!-- Text block group -->
          <TextBlockGroup
            v-if="group.type == 'text'"
            :id="group.id"
            :ref="(ref: any) => (ref ? (textBlockGroupRefs[group.id!] = ref) : delete textBlockGroupRefs[group.id!])"
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
            @navigate="(direction: NavigationDirection) => navigateFromGroup(group, direction)"
            @delete-self="deleteGroup(group)"
          />
          <!-- Block -->
          <div
            v-else
            class="relative mx-auto my-1 rounded"
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
              :ref="
                (ref: any) => (ref ? (nodeBlockRefs[group.block.id!] = ref) : delete nodeBlockRefs[group.block.id!])
              "
              class="w-full"
              :class="isDragging(group.block) ? 'opacity-50' : ''"
              :node-ptr="toNodeRef(group.block)"
              :prepared-connection="preparedConnection"
              :container-gutter-width="widths.gutter"
              v-bind="state.getChildState(group.block.id)"
              :data-contextmenu-items="BLOCK_CONTEXT_ACTIONS.join(',')"
              :draggable="group.block.type == BlockType.PAGE"
              @dragstart.stop="(e: DragEvent) => startDraggingIfAllowed(e, group.block)"
              @navigate="(direction: NavigationDirection) => navigateFromGroup(group, direction)"
              @delete-self="deleteGroup(group)"
            />
          </div>
        </div>

        <!-- Footer -->
        <div
          class="group/footer mx-auto flex flex-row justify-center gap-x-2.5 py-8"
          :style="{
            width: widths.block + 'px',
          }"
          @click.stop="focusText()"
        >
          <!-- Add blocks -->
          <button
            v-for="blockType in HIGHLIGHTED_BLOCK_TYPES"
            data-suppress-drag="both"
            class="rounded-2xl border border-gray-200 px-2 py-0.5 transition-colors duration-150"
            :class="[
              isEmpty
                ? 'bg-gray-100 text-gray-800 hover:bg-gray-200 hover:text-gray-900'
                : 'text-gray-400 hover:bg-gray-100 hover:text-gray-900 group-hover/footer:text-gray-700',
            ]"
            @click.stop="() => createAndFocusBlock({ type: blockType as any }, 'inside', page!)"
          >
            <IconInline
              v-bind="ICON_BY_BLOCK_TYPE[blockType]"
              class="mr-1.5 w-5 text-center transition-colors duration-150"
              :class="isEmpty ? 'text-gray-700' : 'text-gray-400 group-hover/footer:text-gray-700'"
            />
            <span class="text-small">{{ toCamelName(BlockType, blockType) }}</span>
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
