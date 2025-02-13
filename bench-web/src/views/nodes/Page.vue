<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { isDescendantOf } from "@/language/core/graph";
import { cloneNode, moveNode, NodeIn } from "@/language/core/node";
import { STANDARD_TEXT_LINE_TYPES } from "@/language/core/text";
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
import { describeNode, isNode, isNodeRef, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { bench, canvas } from "@/system/space";
import { type ActionMapImplementation } from "@/ui/action";
import { isDragging, isSelecting, startSelectingIfAllowed, useMultiDropZone, useSelectionZone } from "@/ui/drag";
import { ICON_BY_BLOCK_TYPE, IconInline } from "@/ui/icon";
import { isDraggingGlobal, ScrollbarWidth } from "@/ui/layout";
import { useNodeListActions } from "@/ui/list";
import { pushDefaultMenu } from "@/ui/popover";
import { useTextEditor } from "@/ui/prosemirror/editor";
import { PageContext, providePageContext } from "@/ui/prosemirror/page";
import { useHighlightPlugin, useLineHandlePlugin, usePlaceholderPlugin, useTooltipPlugin } from "@/ui/prosemirror/view";
import { useTextPageInterface } from "@/ui/prosemirror/wiring";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import PageHeader from "@/views/builtins/PageHeader.vue";
import RootHeader from "@/views/builtins/RootHeader.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import TextTooltip from "@/views/builtins/TextTooltip.vue";
import { NavigationDirection, type FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { useEventListener } from "@vueuse/core";
import { computed, getCurrentInstance, nextTick, ref, shallowRef, toRef, watch, type Ref } from "vue";

const MIN_BLOCK_WIDTH = 500;
const MAX_BLOCK_WIDTH = 800;
const MIN_GUTTER_WIDTH = 64;
const MIN_FOOTER_PADDING = 200;

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
const vueInstance = getCurrentInstance();
if (vueInstance == null) throw new Error("no vue instance in Page");

const preparedConnection = useExistingConnection(nodePtr);
const { graph, connection } = preparedConnection;
const page = graph.getRef(nodePtr) as Ref<PageData | undefined>;
const blocks = graph.getChildrenRef(nodePtr, NodeType.BLOCK);
const pageContext: PageContext = {
  page,
  blocks,
  blocksRefById: shallowRef({}),
  preparedConnection,
  gutterWidth: computed(() => widths.value.gutter),
};
providePageContext(pageContext);

const pageHeaderRef: Ref<InstanceType<typeof PageHeader> | null> = ref(null);
const textRef = ref<HTMLElement | null>(null);
const contentRef = ref<HTMLElement | null>(null);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);

// size block/gutter horizontally (try to fit both until min block width)
const widths = computed(() => {
  // divide space between block and gutter up to target gutter width
  const blockWidth = Math.min(MAX_BLOCK_WIDTH, Math.max(MIN_BLOCK_WIDTH, props.size.width - MIN_GUTTER_WIDTH * 2));
  const gutterWidth = Math.max(MIN_GUTTER_WIDTH, (props.size.width - blockWidth) / 2);
  return { block: blockWidth, gutter: gutterWidth };
});
const isEmpty = computed(
  () => blocks.value.length == 0 || blocks.value.every((b) => b.type >= BlockType.PARAGRAPH && b.text == null),
);

//
// Text
//

// highlighting
const selectedBlockIds = computedValue(() => {
  const selectedBlockIds: string[] = [];
  for (const block of blocks.value) {
    if (canvas.isSelected(block) || (block.nodePtr != null && canvas.isSelected(block.nodePtr))) {
      selectedBlockIds.push(block.id!);
    }
  }
  return selectedBlockIds;
});
const draggingBlockIds = computedValue(() => {
  const draggingBlockIds: string[] = [];
  for (const block of blocks.value) {
    if (isDragging(block)) {
      draggingBlockIds.push(block.id!);
    }
  }
  return draggingBlockIds;
});
const highlightPlugin = useHighlightPlugin({ selectedBlockIds, draggingBlockIds });
watch([selectedBlockIds, draggingBlockIds], () => {
  updatePlugin(highlightPlugin);
});

// placeholder
const placeholderPlugin = usePlaceholderPlugin({
  defaultPlaceholder: "Write or '/' for commands...",
});

// handle
const lineHandlePlugin = useLineHandlePlugin();

// editor
const tooltipPlugin = useTooltipPlugin({
  component: TextTooltip,
  parentComponent: vueInstance,
  container: contentRef,
  gutterWidth: computed(() => widths.value.gutter),
});
const textInterface = useTextPageInterface({
  page,
  blocks,
  graph,
  txFactory: () => connection.tx,
});
const {
  focus: focusInText,
  actions: textActions,
  lineRefsById,
  updatePlugin,
} = useTextEditor({
  textRef,
  text: textInterface,
  isInput: true,
  suppressEnter: false,
  suppressDrop: true, // handled manually (block-by-block) below
  navigate: (direction: NavigationDirection) => {
    if (direction == "left" || direction == "up") {
      pageHeaderRef.value?.focus?.("top");
    }
  },
  deleteSelf: () => emit("deleteSelf"),
  plugins: [highlightPlugin, tooltipPlugin, placeholderPlugin, lineHandlePlugin],
  parentComponent: vueInstance,
  pageContext,
  onTransaction: (view, prevState, newState) => {
    // auto deselect nodes if anything was edited by the user
    const selectionChanged = !(
      prevState &&
      prevState.doc.eq(newState.doc) &&
      prevState.selection.eq(newState.selection)
    );
    if (selectionChanged && !isSelecting()) {
      canvas.deselect();
    }
  },
});

//
// Interaction
//

const SUGGESTED_BLOCK_TYPES = [BlockType.PAGE, BlockType.FLOW, BlockType.DATABASE, BlockType.CHOICE];

// selecting
const selectionZone = useSelectionZone({ containerEl: contentRef, overlayEl: selectionOverlayRef });

// dragging
const { activeDropZone } = useMultiDropZone({
  name: "page",
  container: contentRef,
  targetsById: lineRefsById,
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
    const target = graph.getOrError({ id: targetId });
    console.log("onDrop", { dragged, anchor, targetId, target, event });
    if (dragged.kind == "file") {
      // create variable with file
      if (!dragged.files) return;
      if (!isNode(target, NodeType.BLOCK)) throw new Error(`unexpected target node type: ${describeNode(target)}`);
      addFiles(dragged.files, anchor == "start" ? "before" : "after", target);
    } else if (dragged.kind == "node") {
      const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
      let node = graph.getOrError(dragged.node);
      if (event.altKey) {
        // clone node before moving
        node = cloneNode(tx, graph, node, { keepProperties: true });
      }
      // move node
      moveNode(tx, graph, node, { anchor, target });
    } else if (dragged.kind == "selection") {
      // move nodes
      const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
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
const activeDropAnchorPosition = computed(() => {
  if (activeDropZone.value?.targetRect == null) return { y: 0 };
  const { anchor, targetRect } = activeDropZone.value;
  if (anchor == "start") {
    return { y: targetRect.top };
  } else {
    return { y: targetRect.bottom };
  }
});

// files
// TODO :UX: paste files with immediate preview
function addFiles(files: FileList | File[], anchor: "before" | "after" | "inside", target: PageData | BlockData) {
  Array.from(files).forEach(async (file) => {
    if (bench.value == null) throw new Error("no current bench");
    const upload = uploadFile(() => connection.tx, file, { bench: bench.value });
    await upload.completion.wait();
    const block = createBlock(connection.tx, graph, {
      block: { type: BlockType.FILE, nodePtr: toNodeRef(upload.file.value!) },
      anchor: anchor,
      target: target,
    });
    focus(toNodeRef(block));
  });
}

// clipboard
useEventListener(contentRef, "paste", (event) => {
  if (event.clipboardData == null) return;
  const files = Array.from(event.clipboardData.files);
  if (files.length == 0) return;
  const focusedBlock = props.focus != null ? graph.get(props.focus.nodesPtr[0]) : null;
  if (isNode(focusedBlock, NodeType.BLOCK)) {
    addFiles(files, "before", focusedBlock);
  } else {
    addFiles(files, "inside", page.value!);
  }
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
  ...textActions,
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
    if (firstBlock == null) {
      createAndFocusBlock({ type: BlockType.PARAGRAPH }, "inside", page.value!);
    } else if (firstBlock.type >= BlockType.PARAGRAPH && STANDARD_TEXT_LINE_TYPES.includes(firstBlock.text?.type!)) {
      focus(firstBlock, "top");
    } else {
      createAndFocusBlock({ type: BlockType.PARAGRAPH }, "before", firstBlock);
    }
  } else {
    const lastBlock = blocks.value[blocks.value.length - 1];
    if (lastBlock == null) {
      createAndFocusBlock({ type: BlockType.PARAGRAPH }, "inside", page.value!);
    } else if (lastBlock.type >= BlockType.PARAGRAPH && STANDARD_TEXT_LINE_TYPES.includes(lastBlock.text?.type!)) {
      focus(lastBlock, "bottom");
    } else {
      createAndFocusBlock({ type: BlockType.PARAGRAPH }, "after", lastBlock);
    }
  }
}

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData | AnyNodeData, innerAnchor?: FocusAnchor) {
  if (anchor == null || typeof anchor == "string") {
    // just focus page
    focusText("bottom");
  } else {
    // focus block
    focusInText(!isNodeRef(anchor) ? toNodeRef(anchor) : anchor);
  }
}

const isSelectingPage = computed(() => isSelecting());
const canSelect = computed(() => !isSelectingPage.value && !isDraggingGlobal.value);

defineExpose<ViewExpose>({ self, actions, focus });
</script>
<template>
  <div class="flex w-full select-none flex-col bg-white text-gray-900" :class="[page ? '' : 'h-full']">
    <!-- Root header -->
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus="props.focus" :graph="graph">
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
      <div
        ref="contentRef"
        class="flex min-h-full flex-col focus:outline-none focus:ring-0"
        :class="[canSelect ? '' : 'cursor-default select-none']"
        :style="{ minHeight: size.height - (isRoot ? VIEW_DEFAULT_ROOT_HEADER_HEIGHT : 0) + 'px' }"
      >
        <!-- Page header (title) -->
        <PageHeader
          ref="pageHeaderRef"
          :width="widths.block"
          :node="page"
          :connection="preparedConnection"
          is-input
          @navigate="
            (direction) => {
              if (direction == 'right' || direction == 'down' || direction == 'enter') {
                focusText('top');
              }
            }
          "
        />

        <!-- Text/Blocks -->
        <div
          ref="textRef"
          class="pm-text stealth relative mx-auto rounded hover:cursor-text"
          data-suppress-actions="space.move.left,space.move.right"
          :style="{
            width: widths.block + 'px',
          }"
        >
          <!-- Dragging anchor -->
          <!-- NOTE :Cleanup: turn dragging anchor into prosemirror plugin?  -->
          <div
            v-if="activeDropZone"
            class="z-40 h-[4px] bg-orange-400"
            :class="activeDropAnchorPosition.y > 0 ? 'fixed' : 'absolute'"
            :style="{
              top: activeDropAnchorPosition.y > 0 ? activeDropAnchorPosition.y - 2 + 'px' : undefined,
              width: widths.block + 'px',
            }"
          />
        </div>

        <!-- Padding -->
        <div
          class="transform transition-all duration-300"
          :class="!isEmpty ? 'pt-auto' : 'pt-0'"
          :style="{ height: MIN_FOOTER_PADDING / 2 + 'px' }"
          @click="focusText()"
        />

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
            v-for="blockType in SUGGESTED_BLOCK_TYPES"
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
        <div class="" :style="{ height: MIN_FOOTER_PADDING / 2 + 'px' }" @click="focusText()" />

        <!-- Selection -->
        <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
      </div>
    </Scroll>
    <Inaccessible v-else class="h-full w-full" :node="nodePtr" :connection="connection" />
  </div>
</template>
