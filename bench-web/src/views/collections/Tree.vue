<script lang="ts" setup>
import { INLINE_NODE_TYPES, RESOURCE_NODE_TYPES, toCamelName } from "@/language/core/const";
import { isDescendantOf, walkDescendantsRef, type NodeTreeItem } from "@/language/core/graph";
import { cloneNode, moveNode, useSubnodeProperty } from "@/language/core/node";
import { newChangeId } from "@/language/runtime/transaction";
import { createBlock } from "@/language/source/block";
import {
  BlockType,
  CHILD_NODE_TYPES,
  NodeReferenceData,
  NodeType,
  Orientation,
  TextLineType,
  TreeViewPreset,
  ViewData,
  ViewType,
  type AnyNodeData,
} from "@/proto/wire";
import { isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { packagePtr } from "@/system/client";
import { useExistingConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapKit } from "@/ui/action";
import {
  isDragging,
  startDraggingIfAllowed,
  startSelectingIfAllowed,
  useMultiDropZone,
  useSelectionZone,
} from "@/ui/drag";
import { IconInline, getNodeIcon } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { VIEW_DEFAULT_HEADER_HEIGHT, makeSelection } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeMetadata from "@/views/builtins/NodeMetadata.vue";
import Title from "@/views/builtins/Title.vue";
import { type FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { useElementSize } from "@vueuse/core";
import { computed, ref, toRef, type Ref } from "vue";

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
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const preset = useSubnodeProperty(NodeType.VIEW, ViewType.TREE, toRef(props, "subnodePacked"), "preset");
const nodeTypes = computed(() => {
  if (preset.value == TreeViewPreset.PACKAGE) {
    // only inline nodes without resources since that would include all Package resources
    //  (we may actually want resource nodes *inside* Pages, not the Package?)
    return INLINE_NODE_TYPES.filter((n) => !RESOURCE_NODE_TYPES.includes(n));
  } else {
    return [];
  }
});
const rootPtr = computedValue(() => {
  if (props.nodePtr != null) {
    return props.nodePtr;
  } else if (preset.value == TreeViewPreset.PACKAGE) {
    return packagePtr.value;
  } else {
    return null;
  }
});
const focusPtr = computedValue(() => {
  if (preset.value == TreeViewPreset.PACKAGE) {
    return packagePtr.value;
  } else {
    return null;
  }
});

const { graph, connection } = useExistingConnection(rootPtr);

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

function fire(node: AnyNodeData) {
  canvas.goToNode(node, { skipSelf: preset.value == TreeViewPreset.OUTLINE });
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
      canvas.goToNode(graph.getOrError(dragged.nodes[dragged.nodes.length - 1]));
    }
  },
});

// actions
const actions: Partial<ActionMapKit<"space">> = {
  // navigate
  "space.navigate.open": (action, ctx) => {
    if (ctx.nodes?.[0] == null) return false;
    canvas.goToNode(ctx.nodes[0], { skipSelf: preset.value == TreeViewPreset.OUTLINE });
  },
  // select
  "space.select.all": () => canvas.select(expandedItems.value.map((item) => item.node)),
};

defineExpose<ViewExpose>({ self, id, actions, focus });
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
        class="group/list relative mb-1 flex flex-col text-gray-900"
        :style="{ maxWidth: `${containerSize.width.value}px` }"
      >
        <!-- Node -->
        <li
          v-for="({ node, depth }, i) in expandedItems"
          :ref="(ref?: any) => (ref != null ? (expandedNodesRefs[node.id] = ref) : delete expandedNodesRefs[node.id])"
          :key="node.id"
          data-contextmenu-items="space.navigate.open"
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
          <button class="group/icon relative mr-1 flex-shrink-0" aria-hidden @click.stop="() => toggleExpanded(node)">
            <IconInline
              v-bind="getNodeIcon(node)"
              class="w-5 text-center transition-colors duration-75 group-hover/node:opacity-0"
            />
            <span
              class="absolute left-0 w-5 rounded bg-gray-100 text-gray-400 opacity-0 transition-all duration-75 group-hover/node:opacity-100"
              :class="isExpanded(node) ? 'rotate-90' : 'rotate-9'"
            >
              <i class="fas fa-chevron-right" />
            </span>
          </button>
          <!-- Name -->
          <span
            v-if="(node as any).name != null && (node as any).name != ''"
            class="max-w-full select-none truncate"
            v-html="(node as any).name"
          />
          <Title
            v-else-if="(node as any).title != null"
            :model-value="(node as any).title"
            :force-line-type="TextLineType.PARAGRAPH"
            class="max-w-full select-none truncate"
            truncate
            is-small
            :placeholder="toCamelName(NodeType, node.metatype)"
          />
          <span
            v-else
            class="max-w-full select-none truncate text-gray-400"
            v-html="toCamelName(NodeType, node.metatype)"
          />
          <!-- Metadata -->
          <NodeMetadata class="ml-1.5" size="sm" :node="node" />
          <!-- Meta -->
          <div class="ml-auto flex flex-row gap-x-1 pl-3 pr-[7px]">
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
