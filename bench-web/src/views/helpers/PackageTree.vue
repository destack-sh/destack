<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { isDescendantOf, walkDescendantsRef, type NodeTreeItem } from "@/language/core/graph";
import { cloneNode, getRootNodes, moveNode } from "@/language/core/node";
import { newChangeId } from "@/language/core/transaction";
import {
  CHILD_NODE_TYPES,
  PAGE_NODE_TYPES,
  NodeReferenceData,
  NodeType,
  Orientation,
  RESOURCE_NODE_TYPES,
  TextLineType,
  ViewData,
  type AnyNodeData,
} from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { packagePtr } from "@/system/client";
import { useAutoConnection } from "@/system/connection";
import { canvas, pagePtr } from "@/system/space";
import type { CommandMapKit } from "@/ui/command";
import {
  isDragging,
  startDraggingIfAllowed,
  startSelectingIfAllowed,
  useMultiDropZone,
  useSelectionZone,
} from "@/ui/drag";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeMetadata from "@/views/builtin/NodeMetadata.vue";
import TextLine from "@/views/content/TextLine.vue";
import { type FocusAnchor, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { useElementSize } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

const DEPTH_OFFSET = 16;
const ITEM_HEIGHT = 30;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const NODE_TYPES = PAGE_NODE_TYPES.filter((n) => !RESOURCE_NODE_TYPES.includes(n) && n != NodeType.THREAD);

const props = defineProps<{} & Pick<ViewData, "icon" | "nodePtr" | "size" | "focusPtr" | "selection">>();

const containerRef: Ref<HTMLElement | null> = ref(null);
const containerSize = useElementSize(containerRef);
const listRef: Ref<HTMLElement | null> = ref(null);
const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);

const nodeTypes = computed(() => NODE_TYPES);
const rootPtr = computedValue(() => props.nodePtr ?? packagePtr.value);
const focusPtr = computedValue(() => packagePtr.value);
const { graph, connection } = useAutoConnection(rootPtr);

//
// Visible subtree
//

const expandedNodesPtr = ref<NodeReferenceData[]>([]);

function isExpanded(node: AnyNodeData | NodeReferenceData) {
  return expandedNodesPtr.value.some((ptr) => ptr.id == node.id) ?? false;
}
function toggleExpanded(node: AnyNodeData | NodeReferenceData) {
  if (expandedNodesPtr.value.some((ptr) => ptr.id == node.id)) {
    expandedNodesPtr.value = expandedNodesPtr.value.filter((ptr) => ptr.id != node.id);
  } else {
    expandedNodesPtr.value = [...expandedNodesPtr.value, toNodeRef(node)];
  }
}

function includes(node: AnyNodeData) {
  return true;
}
function includesChildren(node: AnyNodeData) {
  return true; // include everything
}
const { items: expandedItems } = walkDescendantsRef({
  graph: graph,
  rootPtr,
  nodeTypes: nodeTypes,
  isExpanded,
  includes,
  includesChildren,
  watchSource: () => [props.focusPtr, expandedNodesPtr.value],
});
const expandedNodesRefs: Ref<Record<string, HTMLElement>> = ref({});

const focusedItem = computed(() => {
  if (props.focusPtr != null) {
    const focusedId = props.focusPtr.id;
    return expandedItems.value.find((item) => item.node.id == focusedId);
  } else {
    return null;
  }
});
const focusedNode = computed(() => focusedItem.value?.node);

//
// Interaction
//

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
}

function fire(node: AnyNodeData) {
  canvas.goToNode(node);
}

// dragging
const { activeDropZone } = useMultiDropZone({
  name: "explore",
  container: listRef,
  targetsInOrder: computed(() => expandedItems.value.map((item) => item.node.id)),
  targetsById: expandedNodesRefs,
  orientation: Orientation.VERTICAL,
  hasCenterAnchor: true,
  fallbackToClosest: true,
  kinds: ["node", "selection"],
  metatypes: nodeTypes,
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
      return true;
    });
  },
  onDrop: (dragged, anchor, targetId, event) => {
    if (targetId != null && (dragged.kind == "node" || dragged.kind == "selection")) {
      const tx = connection.tx.with({ change: { key: newChangeId(), title: "Move" } });
      const target = graph.getOrError({ id: targetId });
      const nodes = getRootNodes(dragged.nodes);
      for (let i = 0; i < nodes.length; i++) {
        let node = graph.getOrError(nodes[i]);
        if (event.altKey) {
          // clone node before moving
          node = cloneNode(tx, graph, node, { keepProperties: true });
        }
        moveNode(tx, graph, node, {
          anchor: i == 0 ? anchor : "after",
          target: i == 0 ? target : graph.getOrError(nodes[i - 1]),
        });
      }
      canvas.goToNode(graph.getOrError(nodes[nodes.length - 1]));
    }
  },
});

// actions
const commands: Partial<CommandMapKit<"space">> = {
  // navigate
  "space.navigate.open": (action, ctx) => {
    if (ctx.nodes?.[0] == null) return false;
    canvas.goToNode(ctx.nodes[0]);
  },
  // select
  "space.select.all": () => canvas.select(expandedItems.value.map((item) => item.node)),
};

defineExpose<Omit<ViewExpose, "id" | "self">>({ commands, focus });
</script>
<template>
  <div
    ref="containerRef"
    :class="size == null ? '' : 'h-full w-full'"
    @mousedown="(e) => startSelectingIfAllowed(selectionZone, e)"
  >
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
        class="group/list relative flex flex-col text-gray-900"
        :style="{ maxWidth: `${containerSize.width.value}px` }"
      >
        <!-- Node -->
        <li
          v-for="({ node, depth }, i) in expandedItems"
          :ref="(ref?: any) => (ref != null ? (expandedNodesRefs[node.id] = ref) : delete expandedNodesRefs[node.id])"
          :key="node.id"
          data-contextmenu-items="space.navigate.open"
          :data-node-type="node.metatype"
          :data-node-id="node.id"
          :data-node-ck="(node as any).ck"
          :data-node-bench-id="(node as any).benchPtr?.id"
          class="group/node relative mx-3 flex max-w-full flex-row items-center rounded-sm border transition-colors duration-75 hover:cursor-pointer"
          :class="[
            activeDropZone?.targetId == node.id && activeDropZone?.anchor == 'center'
              ? 'border-gray-400'
              : 'border-transparent',
            canvas.isSelected(node)
              ? 'bg-amber-400/20'
              : isFocused(node) || canvas.isHighlighted(node)
                ? 'bg-gray-100'
                : 'hover:bg-gray-100',
            isDragging(node) ? 'opacity-50' : '',
            pagePtr?.id == node.id ? 'bg-gray-100' : '',
          ]"
          :style="{
            paddingLeft: 6 + depth * DEPTH_OFFSET + 'px',
            paddingRight: 6 + 'px',
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
            class="absolute z-10 h-1 rounded-sm bg-gray-400"
            :class="[activeDropZone?.anchor == 'start' ? (i == 0 ? 'top-0' : '-top-[4px]') : '-bottom-[2px]']"
            :style="{
              left: 8 + depth * DEPTH_OFFSET + 'px',
              width: 'calc(100% - ' + (8 + depth * DEPTH_OFFSET) + 'px)',
            }"
          />
          <!-- Icon/Expand button -->
          <button
            class="group/icon relative mr-1 shrink-0 cursor-pointer"
            aria-hidden
            @click.stop="() => toggleExpanded(node)"
          >
            <IconInline
              v-bind="getNodeIcon(node)"
              class="w-5 text-center transition-colors duration-75 group-hover/node:opacity-0"
            />
            <span
              class="absolute left-0 w-5 rounded-sm bg-gray-100 text-gray-400 opacity-0 transition-all duration-75 group-hover/node:opacity-100"
              :class="isExpanded(node) ? 'rotate-90' : 'rotate-9'"
            >
              <i class="fas fa-chevron-right" />
            </span>
          </button>
          <!-- Name -->
          <span
            v-if="(node as any).name != null && (node as any).name != ''"
            class="max-w-full truncate select-none"
            v-html="(node as any).name"
          />
          <TextLine
            v-else-if="(node as any).title != null"
            :model-value="(node as any).title"
            :force-line-type="TextLineType.PARAGRAPH"
            class="max-w-full truncate select-none"
            truncate
            is-small
            :placeholder="toCamelName(NodeType, node.metatype)"
          />
          <span
            v-else
            class="max-w-full truncate text-gray-400 select-none"
            v-html="toCamelName(NodeType, node.metatype)"
          />
          <!-- Metadata -->
          <NodeMetadata class="ml-1.5" size="sm" :node="node" />
          <!-- Meta -->
          <div class="ml-auto flex flex-row gap-x-1 pr-[7px] pl-3">
            <!-- ... -->
          </div>
          <!-- ... -->
        </li>

        <!-- Selection overlay -->
        <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
      </ul>
    </component>
    <!-- Empty -->
    <div
      v-else
      class="flex h-full w-full flex-col justify-center px-4"
      :style="{
        height: `${ITEM_HEIGHT}px`,
      }"
    >
      <!-- Empty -->
      <span class="text-gray-400">Nothing</span>
    </div>
  </div>
</template>
