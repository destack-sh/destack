<script lang="ts" setup>
import { createBlock } from "@/language/block";
import { CANVAS_BLOCK_TYPES, toCamelName } from "@/language/const";
import { makeTypeInfo } from "@/language/field";
import { uploadFile } from "@/language/file";
import { isDescendantOf } from "@/language/graph";
import { useNodeListActions } from "@/language/list";
import { moveNode, NodeIn } from "@/language/node";
import { newChangeId } from "@/language/transaction";
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
import { pushDefaultMenu, type PopoverInfoIn } from "@/ui/popover";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { blurDocument } from "@/utils/element";
import { computedValue } from "@/utils/ref";
import HistoryNavigator from "@/views/builtins/HistoryNavigator.vue";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodePath from "@/views/builtins/NodePath.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Block from "@/views/system/Block.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const MIN_BLOCK_WIDTH = 500;
const MAX_BLOCK_WIDTH = 800;
const MIN_GUTTER_WIDTH = 60;
const BLOCK_GAP_Y = 4;

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
const nodePtr = computed(() => props.nodePtr);
const state = canvas.registerView(self, id);

const preparedConnection = useExistingConnection(nodePtr);
const { graph, connection } = preparedConnection;
const page = graph.getRef(nodePtr) as Ref<BlockData | undefined>;
const blocks = graph.getChildrenRef(nodePtr, NodeType.BLOCK);

const historyRef: Ref<InstanceType<typeof HistoryNavigator> | null> = ref(null);
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
// Interaction
//

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
  onDrop: (dragged, anchor, targetId) => {
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
        const variableType = makeTypeInfo({ kind: TypeKind.NODE, benchType: BenchType.FILE });
        const block = createBlock(connection.tx, graph, {
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
      const target = graph.getOrError({ id: targetId });
      moveNode(connection.tx, graph, dragged.node, { anchor, target });
    } else if (dragged.kind == "selection") {
      // move nodes
      const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
      const target = graph.getOrError({ id: targetId });
      for (let i = 0; i < dragged.nodes.length; i++) {
        moveNode(tx, graph, dragged.nodes[i], {
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
      createAndFocusBlock({ type: BlockType.TEXT }, node != null ? anchor : "inside", node ?? page.value!),
  }),
};
function createAndFocusBlock(
  blockIn: Partial<NodeIn<NodeType.BLOCK>> & Required<Pick<NodeIn<NodeType.BLOCK>, "type">>,
  anchor: "before" | "after" | "inside",
  target: BlockData | TypedNodeReferenceData<NodeType.BLOCK>,
) {
  const block = createBlock(connection.tx, graph, { block: blockIn, anchor, target });
  canvas.inspect({ node: block });
  if (block.type == BlockType.TEXT) {
    nextTick(() => focus(block));
  } else if (block.type != BlockType.PAGE) {
    canvas.select([block]);
  } else {
    canvas.goToNode(block);
  }
  return block;
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
    <!-- Meta header -->
    <div
      data-keep-inspection-in-base-view="true"
      class="group flex w-full max-w-full flex-row items-center px-2"
      :style="{ height: (historyRef?.isActive ? VIEW_DEFAULT_BAR_HEADER_HEIGHT : HEADER_HEIGHT) + 'px' }"
    >
      <!-- History -->
      <HistoryNavigator ref="historyRef" :self="self" />
      <!-- Breadcrumb -->
      <NodePath v-if="nodePtr" :container="nodePtr" :focus="$props.focus?.nodesPtr[0]" :self="nodePtr" :graph="graph" />
      <!-- Meta & Controls -->
      <div class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-1.5 pl-1">
        <button
          class="text-gray-400 hover:bg-gray-100 hover:text-gray-700"
          @click="(e) => pushDefaultMenu('main', page!, e)"
        >
          <i class="fas fa-ellipsis-vertical w-5 text-center" />
        </button>
      </div>
    </div>

    <!-- Page content -->
    <Scroll
      v-if="page"
      id="body"
      data-contextmenu-items="list.create.above,list.create.below,space.edit.paste"
      :size="{ width: size.width, height: size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
      @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)"
    >
    <div ref="contentRef" class="flex min-h-full flex-col pb-[320px]">
        <!-- Page header (title) -->
        <div
          class="mx-auto mb-2 mt-5 flex flex-row items-center"
          :style="{
            width: widths.block + 'px',
          }"
        >
          <NodeReference ref="nameRef" size="title" :node="page" is-input :tx="() => connection.tx" />
        </div>

        <!-- Blocks -->
        <div
          v-for="(block, i) in blocks"
          :key="block.id"
          class="group/block-line relative flex min-w-fit flex-row"
          :style="{
            paddingTop: CANVAS_BLOCK_TYPES.includes(block.type) ? '10px' : undefined,
            marginTop: BLOCK_GAP_Y + 'px',
          }"
        >
          <!-- Left gutter -->
          <div
            class="relative flex flex-shrink-0 flex-row items-start justify-end gap-x-1 px-1 text-right transition-colors duration-150"
            :class="[
              CANVAS_BLOCK_TYPES.includes(block.type) ? 'pt-2' : 'pt-1',
              canvas.isInspected(block) || canvas.isHighlighted(block)
                ? 'opacity-100'
                : 'opacity-0 group-focus-within/block-line:opacity-100 group-hover/block-line:opacity-100',
            ]"
            :style="{ width: widths.gutter + 'px' }"
          >
            <!-- Create above / below -->
            <button
              v-menu="
                (): PopoverInfoIn => ({
                  kind: 'view',
                  component: ViewType.PICKER,
                  title: 'Add Block Below',
                  placement: 'bottom',
                  props: { valueType: makeTypeInfo({ benchType: BenchType.BLOCK_TYPE, isRequired: true }) },
                  onApply: (blockType: BlockType) => createAndFocusBlock({ type: blockType }, 'after', block),
                })
              "
              class="ml-2 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
              :style="{}"
              @click="(e) => canvas.select([block])"
            >
              <i class="fas fa-plus" />
            </button>
            <!-- Controls/Drag -->
            <button
              class="rounded text-gray-400 hover:bg-gray-100 hover:text-gray-700"
              :draggable="true"
              data-suppress-drag="select"
              @click="(e) => pushDefaultMenu('main', block, e)"
              @dragstart.stop="(e) => startDraggingIfAllowed(e, block)"
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
              class="absolute z-10 h-1 w-full rounded bg-gray-400"
              :style="getAnchorPositionStyle(activeDropZone?.anchor as 'start' | 'end', i, 4)"
            />

            <!-- Block -->
            <Block
              :id="block.id"
              :ref="(ref: any) => (ref ? (blockRefs[block.id!] = ref) : delete blockRefs[block.id!])"
              class="w-full"
              :class="isDragging(block) ? 'opacity-50' : ''"
              :node-ptr="toNodeRef(block)"
              :prepared-connection="preparedConnection"
              :containerGutterWidth="widths.gutter"
              v-bind="state.getChildState(block.id)"
              :data-contextmenu-items="BLOCK_CONTEXT_ACTIONS.join(',')"
              :draggable="block.type == BlockType.PAGE"
              @dragstart.stop="(e) => startDraggingIfAllowed(e, block)"
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
            data-suppress-drag="both"
            class="rounded-2xl border border-gray-200 px-2 py-0.5 text-gray-700 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-900"
            @click="() => createAndFocusBlock({ type: blockType }, 'inside', page!)"
          >
            <IconInline v-bind="ICON_BY_BLOCK_TYPE[blockType]" class="mr-1.5 w-5 text-center text-gray-700" />
            <span>{{ toCamelName(BlockType, blockType) }}</span>
          </button>
        </div>

        <!-- Selection -->
        <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
      </div>
    </Scroll>
    <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="connection" />
  </div>
</template>
