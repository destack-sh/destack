<script lang="ts" setup>
import { createBlock } from "@/language/block";
import { CANVAS_BLOCK_TYPES, toCamelName } from "@/language/const";
import { NAME_TYPE } from "@/language/field";
import { isDescendantOf, walkDescendantsRef, type NodeTreeItem } from "@/language/graph";
import { cloneNode, moveNode, unpackSubnodeProperty, useSubnodeProperty } from "@/language/node";
import { newChangeId } from "@/language/transaction";
import {
  BlockData,
  BlockType,
  CHILD_NODE_TYPES,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  TreeViewPreset,
  ViewData,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import { isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { packagePtr } from "@/system/client";
import { useExistingConnection, type Connection } from "@/system/connection";
import { canvas, inspectionBasePtr, inspectionPtr } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import {
  isDragging,
  startDraggingIfAllowed,
  startSelectingIfAllowed,
  useMultiDropZone,
  useSelectionZone,
} from "@/ui/drag";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { highlightMatches } from "@/ui/search";
import { VIEW_DEFAULT_HEADER_HEIGHT, makeSelection } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import NativeInput from "@/views/content/NativeInput.vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, toRef, watch, type Ref } from "vue";

const DEPTH_OFFSET = 16;
const ITEM_HEIGHT = 28;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
  } & Pick<ViewData, "icon" | "nodePtr" | "size" | "focus" | "selection" | "subnodePacked">
>();

const containerRef: Ref<HTMLElement | null> = ref(null);
const containerSize = useElementSize(containerRef);
const listRef: Ref<HTMLElement | null> = ref(null);
const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const preset = useSubnodeProperty(NodeType.VIEW, ViewType.TREE, toRef(props, "subnodePacked"), "preset");
const filterIsPage = computed(() => {
  if (preset.value == TreeViewPreset.EXPLORE) {
    return true;
  } else if (preset.value == TreeViewPreset.OUTLINE) {
    return false;
  } else {
    return unpackSubnodeProperty(NodeType.VIEW, ViewType.TREE, props.subnodePacked, "filterIsPage");
  }
});
const inspectedNodeTypes = computed(() => {
  if (preset.value == TreeViewPreset.EXPLORE) {
    return [NodeType.BLOCK];
  } else if (preset.value == TreeViewPreset.OUTLINE) {
    return [NodeType.BLOCK, NodeType.FIELD, NodeType.VIEW, NodeType.ACTION];
  } else {
    return unpackSubnodeProperty(NodeType.VIEW, ViewType.TREE, props.subnodePacked, "nodeTypes");
  }
});
const rootPtr = computedValue(() => {
  if (props.nodePtr != null) {
    return props.nodePtr;
  } else if (preset.value == TreeViewPreset.EXPLORE) {
    return packagePtr.value;
  } else if (preset.value == TreeViewPreset.OUTLINE) {
    return inspectionBasePtr.value;
  } else {
    return null;
  }
});
const focusPtr = computedValue(() => {
  if (preset.value == TreeViewPreset.EXPLORE) {
    return inspectionBasePtr.value;
  } else if (preset.value == TreeViewPreset.OUTLINE) {
    return inspectionPtr.value;
  } else {
    return null;
  }
});

const { graph, connection } = useExistingConnection(rootPtr, {
  match: {
    predicate: (c) => {
      // NOTE: hack to exclude Bench connection (which also contains package) :ConnectionMatching
      return !(c as Connection<"get", NodeType>).params.roots.some((r) => r.nodeType == NodeType.BENCH);
    },
  },
});

//
// Visible subtree
//

const expandedNodesPtr = useSubnodeProperty(
  NodeType.VIEW,
  ViewType.TREE,
  toRef(props, "subnodePacked"),
  "expandedNodesPtr",
);
function isExpanded(node: AnyNodeData | NodeReferenceData) {
  return expandedNodesPtr.value?.some((ref) => ref.id == node.id);
}
function toggleExpanded(node: AnyNodeData | NodeReferenceData) {
  const newExpandedNodesPtr = isExpanded(node)
    ? expandedNodesPtr.value?.filter((ref) => ref.id != node.id)
    : [...(expandedNodesPtr.value ?? []), toNodeRef(node as NodeReferenceData)];
  state.update(
    { metatype: NodeType.VIEW, type: ViewType.TREE, subnode: { expandedNodesPtr: newExpandedNodesPtr } },
    { debounce: "long" },
  );
}

function isIncludedSelf(node: AnyNodeData) {
  if (filterIsPage.value) {
    if (isNode(node, NodeType.BLOCK)) {
      return node.type == BlockType.PAGE || CANVAS_BLOCK_TYPES.includes(node.type);
    } else {
      return true;
    }
  } else {
    return !isNode(node, NodeType.BLOCK) || node.type != BlockType.TEXT;
  }
}
function isIncludedChildren(node: AnyNodeData) {
  if (filterIsPage.value) {
    return true;
  } else if (filterIsPage.value === false) {
    // don't descend into pages for outline
    if (node.metatype == ObjectType.BLOCK) return (node as BlockData).type != BlockType.PAGE;
    else return true;
  } else {
    return true; // include everything
  }
}
const { items: expandedItems } = walkDescendantsRef({
  graph: graph,
  rootPtr,
  nodeTypes: inspectedNodeTypes,
  isExpanded,
  isIncludedSelf,
  isIncludedChildren,
  watchSource: () => [props.focus, expandedNodesPtr.value],
});
const expandedNodesRefs: Ref<Record<string, HTMLElement>> = ref({});

const focusedItem = computed(() => {
  if (props.focus?.nodesPtr.length ?? 0 > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    return expandedItems.value.find((item) => item.node.id == focusedId);
  } else {
    return null;
  }
});
const focusedNode = computed(() => focusedItem.value?.node);

//
// Interaction
//

const editingNameRef: Ref<InstanceType<typeof NativeInput>[]> = ref([]);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayRef });

function isFocused(node: { id?: string }): boolean {
  return node.id == focusPtr.value?.id;
}

function focus(anchor?: "next" | "previous" | number | FocusAnchor | NodeReferenceData): void {
  let toFocus: NodeTreeItem<any> | null = null;
  if (anchor == "top") {
    toFocus = expandedItems.value[0];
  } else if (anchor == "bottom") {
    toFocus = expandedItems.value[expandedItems.value.length - 1];
  } else if (anchor == "previous") {
    const idx = expandedItems.value.findIndex((item) => item.node.id == focusedNode.value?.id);
    if (idx > 0) toFocus = expandedItems.value[idx - 1];
  } else if (anchor == "next") {
    const idx = expandedItems.value.findIndex((item) => item.node.id == focusedNode.value?.id);
    if (idx < expandedItems.value.length - 1) toFocus = expandedItems.value[idx + 1];
  } else if (typeof anchor == "number") {
    toFocus = expandedItems.value[anchor];
  }
  if (toFocus != null) doFocus(toFocus.node);
  else queryRef.value?.focus();
}

function doFocus(node: AnyNodeData | NodeReferenceData) {
  if (focusedNode.value?.id != node.id) {
    state.update({ focus: makeSelection([node]) }, { debounce: "tick" });
  }
  queryRef.value?.focus();
  expandedNodesRefs.value[node.id!]?.scrollIntoView({ block: "center", behavior: "instant" });
}

function clear() {
  query.value = "";
}

function fire(node: AnyNodeData) {
  canvas.goToNode(node, { skipSelf: preset.value == TreeViewPreset.OUTLINE });
}

/** Navigate horizontally to expand/collapse */
function onNavigateHorizontal(direction: "left" | "right") {
  if (!focusedItem.value?.hasChildren) return;
  else if (direction == "left") {
    if (isExpanded(focusedNode.value!)) toggleExpanded(focusedNode.value!);
  } else {
    if (!isExpanded(focusedNode.value!)) toggleExpanded(focusedNode.value!);
  }
}

// highlight and focus best match when typing
const nodeTitlesMarked: Ref<(string | null)[]> = ref([]);
const uf = new uFuzzy({ intraMode: 1 });
watch(
  [query],
  () => {
    nodeTitlesMarked.value = [];
    if (!query.value) return;

    // highlight
    const { markedResults, bestMatches } = highlightMatches({
      uf,
      query: query.value,
      candidates: expandedItems.value.map((item) => (item.node as any).name ?? ""),
    });
    nodeTitlesMarked.value = markedResults;

    // auto-select best match
    if (bestMatches.length > 0) focus(bestMatches[0]);
  },
  { immediate: true },
);

// dragging
const { activeDropZone } = useMultiDropZone({
  name: "explore",
  container: listRef,
  targets: expandedNodesRefs,
  orientation: Orientation.VERTICAL,
  hasCenterAnchor: true,
  fallbackToClosest: true,
  kinds: ["node", "selection"],
  metatypes: inspectedNodeTypes,
  allowDrop: (dragged, anchor, targetId) => {
    if (dragged.kind != "node" && dragged.kind != "selection") return false;
    return dragged.nodes.every((node) => {
      const target = targetId != null ? graph.get({ id: targetId }) : null;
      if (target == null || isDescendantOf(graph, target, node)) {
        return false; // circular
      }
      const targetParentType =
        anchor == "center" ? (target.metatype as unknown as NodeType) : target.parentPtr!.nodeType;
      if (!CHILD_NODE_TYPES[targetParentType].includes(node.metatype as unknown as NodeType)) {
        return false; // not a child
      }
      if (
        isNode(target, NodeType.BLOCK) &&
        isNode(dragged.nodes[0], NodeType.BLOCK) &&
        anchor == "center" &&
        target.type != BlockType.PAGE
      ) {
        return false; // can only move blocks into page blocks
      }
      return true;
    });
  },
  onDrop: (dragged, anchor, targetId, event) => {
    if (targetId != null && (dragged.kind == "node" || dragged.kind == "selection")) {
      const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
      const target = graph.getOrError({ id: targetId });
      for (let i = 0; i < dragged.nodes.length; i++) {
        let node = graph.getOrError(dragged.nodes[i]);
        if (event.altKey) {
          // clone node before moving
          node = cloneNode(tx, graph, node, { keepProperties: true});
        }
        moveNode(tx, graph, node, {
          anchor: i == 0 ? anchor : "after",
          target: i == 0 ? target : graph.getOrError(dragged.nodes[i - 1]),
        });
      }
      canvas.goToNode(graph.getOrError(dragged.nodes[dragged.nodes.length - 1]));
    }
  },
});

// actions
const actions: Partial<ActionMapImplementation<"space">> = {
  // navigate
  "space.navigate.open": (action, ctx) => {
    if (ctx.nodes?.[0] == null) return false;
    canvas.goToNode(ctx.nodes[0], { skipSelf: preset.value == TreeViewPreset.OUTLINE });
  },
  // select
  "space.select.all": () => canvas.select(expandedItems.value.map((item) => item.node)),
};

defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <div
    ref="containerRef"
    :class="size == null ? '' : 'h-full w-full'"
    @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)"
  >
    <!-- Magic floating query -->
    <!-- Captures focus for navigation & typing for search/highlight -->
    <div class="relative">
      <div class="absolute -top-4 left-0 px-2 pl-4">
        <input
          ref="queryRef"
          v-model="query"
          class="max-w-60 cursor-default rounded border-0 bg-transparent font-semibold text-gray-900 decoration-2 underline-offset-3 caret-transparent outline-none ring-0 focus:underline focus:ring-0"
          spellcheck="false"
          :data-suppress-actions="'space.navigate' /* allow select & move */"
          @keydown.enter.stop.prevent="
            () => {
              if (focusedNode != null) {
                query = '';
                fire(focusedNode);
              }
            }
          "
          @keydown.up.stop.prevent="focus('previous')"
          @keydown.down.stop.prevent="focus('next')"
          @keydown.right.stop.prevent="onNavigateHorizontal('right')"
          @keydown.left.stop.prevent="onNavigateHorizontal('left')"
        />
      </div>
    </div>

    <!-- Content -->
    <component
      :is="size == null ? 'div' : Scroll"
      v-if="expandedItems.length > 0"
      id="scroll"
      :size="{ width: containerSize.width.value, height: containerSize.height.value - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
      @click.stop="queryRef?.focus()"
    >
      <!-- Nodes -->
      <!-- NOTE :UX: it would be neat to have hover/focused/selected nodes highlighted (everywhere) -->
      <ul
        ref="listRef"
        class="group/list relative mb-1 flex flex-col text-gray-900"
        :style="{ maxWidth: `${containerSize.width.value}px` }"
      >
        <!-- Node -->
        <li
          v-for="({ node, depth, hasChildren }, i) in expandedItems"
          :ref="(ref?: any) => (ref != null ? (expandedNodesRefs[node.id] = ref) : delete expandedNodesRefs[node.id])"
          :key="node.id"
          data-contextmenu-items="space.navigate.open*"
          :data-node-id="node.id"
          :data-node-ck="(node as any).ck"
          :data-node-type="node.metatype"
          class="group/node relative mx-1.5 flex max-w-full flex-row items-center rounded border py-[3px] transition-colors duration-150 hover:cursor-pointer"
          :class="[
            activeDropZone?.targetId == node.id && activeDropZone?.anchor == 'center'
              ? 'border-gray-400'
              : 'border-transparent',
            canvas.isSelected(node)
              ? 'bg-orange-400/20'
              : isFocused(node) || canvas.isHighlighted(node)
                ? 'bg-gray-100'
                : 'hover:bg-gray-100',
            isDragging(node) ? 'opacity-50' : '',
            (node as any).name != null ? '' : 'italic',
          ]"
          :style="{
            paddingLeft: 6 + depth * DEPTH_OFFSET + 'px',
            paddingRight: 8 + 'px',
            height: ITEM_HEIGHT + 'px',
          }"
          role="treeitem"
          data-suppress-drag="select"
          :draggable="true"
          @click.stop="fire(node)"
          @dragstart.stop="(e: DragEvent) => startDraggingIfAllowed(e, node)"
        >
          <!-- Drop indicator -->
          <div
            v-if="activeDropZone?.targetId == node.id && activeDropZone?.anchor != 'center'"
            class="absolute z-10 h-1 rounded bg-gray-400"
            :class="[activeDropZone?.anchor == 'start' ? (i == 0 ? 'top-0' : '-top-[4px]') : '-bottom-[2px]']"
            :style="{
              left: 8 + depth * DEPTH_OFFSET + 'px',
              width: 'calc(100% - ' + (8 + depth * DEPTH_OFFSET) + 'px)',
            }"
          />
          <!-- Icon/Expand button -->
          <button class="group/icon relative mr-1.5 flex-shrink-0" @click.stop="() => toggleExpanded(node)">
            <IconInline
              v-bind="getNodeIcon(node)"
              class="w-5 text-center transition-colors duration-75"
              :class="hasChildren ? 'group-hover/node:opacity-0' : ''"
            />
            <span
              v-if="hasChildren"
              class="absolute left-0 w-5 rounded bg-gray-100 text-gray-400 opacity-0 transition-all duration-75 group-hover/node:opacity-100"
              :class="isExpanded(node) ? 'rotate-90' : 'rotate-9'"
              ><i class="fas fa-chevron-right"
            /></span>
          </button>
          <!-- Name -->
          <span
            class="max-w-full select-none truncate"
            v-html="nodeTitlesMarked[i] ?? (node as any).name ?? toCamelName(NodeType, node.metatype)"
          />
          <!-- Metadata -->
          <NodeMetadata class="ml-1.5" size="regular" :node="node" />
          <!-- Meta -->
          <div class="ml-auto flex flex-row gap-x-1 pl-3 pr-[3px]">
            <!-- Create inside -->
            <button
              v-if="isNode(node, NodeType.BLOCK)"
              role="button"
              class="text-gray-400 opacity-0 hover:text-gray-400 group-hover/node:opacity-100"
              @click.stop="
                () => {
                  const block = createBlock(connection.tx, graph, {
                    anchor: 'inside',
                    target: node,
                    block: { type: BlockType.PAGE },
                  });
                  canvas.goToNode(block);
                  if (!isExpanded(node)) toggleExpanded(node);
                }
              "
            >
              <i class="fas fa-plus" />
            </button>
          </div>
          <!-- ... -->
        </li>

        <!-- Selection overlay -->
        <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
      </ul>
    </component>
    <div v-else class="flex h-full w-full flex-col justify-center text-center">
      <!-- Missing state -->
    </div>
  </div>
</template>
