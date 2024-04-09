<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData, ViewType, type AnyNodeData } from "@/proto/wire";
import type { ActionMapImplementation } from "@/system/action";
import { packagePtr } from "@/system/client";
import { useExistingConnection, type GraphConnection } from "@/system/connection";
import { IconInline } from "@/system/icon";
import { getNodeIcon } from "@/system/lang";
import { highlightMatches } from "@/system/search";
import { canvas, inspectionPtr } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { menuActionsLike, type MenuContext } from "@/utils/menu";
import { manualSubRef, toValueRef } from "@/utils/ref";
import { collapseSelection, expandSelection, makeSelection } from "@/views/canvas";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { computed, ref, toRef, watch, type Ref } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "type" | "name" | "title" | "text" | "icon" | "nodePtr" | "focus" | "selection" | "expansion"
  >
>();

const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);

const rootPtr = toValueRef(
  computed(() => {
    if (props.nodePtr != null) return props.nodePtr;
    else if (props.type == ViewType.EXPLORER) return packagePtr.value;
    else if (props.type == ViewType.OUTLINE) return inspectionPtr.value;
    else return null;
  }),
);
const nodeTypes = computed(() => {
  if (props.type == ViewType.EXPLORER) return [NodeType.BLOCK];
  else if (props.type == ViewType.OUTLINE) return [NodeType.BLOCK, NodeType.FIELD, NodeType.VIEW, NodeType.STEP];
  else return [];
});
const { graph: inspectedGraph, connection: inspectedConnection } = useExistingConnection(rootPtr, {
  isOptional: true,
  match: {
    predicate: (c) => {
      // NOTE: hack to exclude Bench connection (which also contains package) :ConnectionMatching
      return !(c as GraphConnection<"get", NodeType>).params.roots.some((r) => r.type == NodeType.BENCH);
    },
  },
});

//
// Visible subtree
// TODO
//

type NodeTreeItem = { node: AnyNodeData; depth: number; isFocusedAbsolute: boolean; canExpand: boolean };
const expandedNodesRefs: Record<string, HTMLElement> = {};

const _expandedNodesSubs: Array<() => void> = [];
const _expandedNodesUnsub = () => {
  _expandedNodesSubs.forEach((sub) => sub());
  _expandedNodesSubs.length = 0;
};
function getExpandedNodes(): NodeTreeItem[] {
  _expandedNodesUnsub();
  if (rootPtr.value == null) return [];

  const items: NodeTreeItem[] = [];
  function walkDescendants(node: AnyNodeData, depth: number) {
    const children = nodeTypes.value.flatMap((type) => inspectedGraph.getChildren(node, type));
    const item = { node, depth, isFocusedAbsolute: false, canExpand: children.length > 0 };
    if (depth >= 0) items.push(item); // ignore root

    // descend
    if (depth < 0 || isExpanded(node)) {
      children.forEach((child) => walkDescendants(child, depth + 1));
      nodeTypes.value.forEach((nodeType) =>
        _expandedNodesSubs.push(
          inspectedGraph.subscribeChildren(node, nodeType, updateExpandedNodes, { ignoreAncestors: true }),
        ),
      );
    }
  }

  const root = inspectedGraph.getMaybe(rootPtr.value);
  _expandedNodesSubs.push(inspectedGraph.subscribe(rootPtr.value, updateExpandedNodes, { ignoreAncestors: true }));
  if (root != null) walkDescendants(root, -1);

  return items;
}
const { ref: expandedNodes, trigger: updateExpandedNodes } = manualSubRef(getExpandedNodes, _expandedNodesUnsub);
watch(() => [rootPtr.value, props.focus, props.expansion], updateExpandedNodes);

const focusedNode = computed(() => {
  if (props.focus?.nodesPtr.length ?? 0 > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    return expandedNodes.value.find((item) => item.node.id == focusedId)?.node;
  } else {
    return null;
  }
});

//
// Interaction
//

function toggleExpanded(node: AnyNodeData | NodeReferenceData) {
  const selfNode = spaceGraph.getOrFail(self.value) as ViewData;
  if (isExpanded(node)) {
    spaceConnection.tx.updateDebounced(selfNode, {
      expansion: collapseSelection(selfNode.expansion!, [node]),
    });
  } else {
    spaceConnection.tx.updateDebounced(selfNode, {
      expansion: expandSelection(selfNode.expansion, [node]),
    });
  }
}

function isExpanded(node: { id?: string; ck?: string }) {
  return props.type == ViewType.OUTLINE || props.expansion?.nodesPtr?.some((n) => n.id == node.id);
}

const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);
function focus(anchor: "next" | "previous" | number | FocusAnchor | NodeReferenceData): void {
  let toFocus: NodeTreeItem | null = null;
  if (anchor == "top") {
    toFocus = expandedNodes.value[0];
  } else if (anchor == "bottom") {
    toFocus = expandedNodes.value[expandedNodes.value.length - 1];
  } else if (anchor == "previous") {
    const idx = expandedNodes.value.findIndex((item) => item.node.id == focusedNode.value?.id);
    if (idx > 0) toFocus = expandedNodes.value[idx - 1];
  } else if (anchor == "next") {
    const idx = expandedNodes.value.findIndex((item) => item.node.id == focusedNode.value?.id);
    if (idx < expandedNodes.value.length - 1) toFocus = expandedNodes.value[idx + 1];
  } else if (typeof anchor == "number") {
    toFocus = expandedNodes.value[anchor];
  }
  if (toFocus != null) doFocus(toFocus.node);
  else queryRef.value?.focus();
}

function doFocus(node: AnyNodeData | NodeReferenceData) {
  const selfNode = spaceGraph.getOrFail(self.value) as ViewData;
  spaceConnection.tx.updateDebounced(selfNode, { focus: makeSelection([node]) });
  focusInComponent(node.id!);
}

function focusInComponent(nodeId: string) {
  queryRef.value?.focus();
  expandedNodesRefs[nodeId]?.scrollIntoView({ block: "center", behavior: "instant" });
}

function clear() {
  query.value = "";
}

function fire(node: AnyNodeData) {
  canvas.goToNode(node);
}

/** Navigate horizontally to expand/collapse */
function onNavigateHorizontal(direction: "left" | "right") {
  if (direction == "left") {
    if (isExpanded(focusedNode.value!)) toggleExpanded(focusedNode.value!);
  } else {
    if (!isExpanded(focusedNode.value!)) toggleExpanded(focusedNode.value!);
  }
}

// highlight and focus best match when typing
const nodeTitleMarked: Ref<(string | null)[]> = ref([]);
const uf = new uFuzzy({ intraMode: 1 });
watch(
  [query],
  () => {
    nodeTitleMarked.value = [];
    if (!query.value) return;

    // highlight
    const { markedResults, bestMatches } = highlightMatches({
      uf,
      query: query.value,
      candidates: expandedNodes.value.map((item) => (item.node as any).name ?? ""),
    });
    nodeTitleMarked.value = markedResults;

    // auto-select best match
    if (bestMatches.length > 0) focus(bestMatches[0]);
  },
  { immediate: true },
);

const actions: Partial<ActionMapImplementation<"common">> = {
  "common.edit.delete": {
    enabled: computed(() => focusedNode.value != null),
    action: () => {
      if (focusedNode.value != null) {
        inspectedConnection.tx.softDelete(focusedNode.value);
      }
    },
  },
  // <!-- nocheckin :Incomplete: explorer/outline actions -->
};

canvas.registerView(self);
defineExpose<ViewExposed>({ self, actions, focus });
</script>
<template>
  <Scroll
    :size="size"
    :orientation="Orientation.VERTICAL"
    :track-width="ScrollbarWidth.md"
    track-is-overlay
    class="bg-white"
    @click.stop="queryRef?.focus()"
  >
    <!-- Magic floating query -->
    <!-- Captures focus for navigation & typing for search/highlight -->
    <div class="relative">
      <div class="absolute -top-5 left-0 px-2 pl-4">
        <input
          ref="queryRef"
          class="max-w-60 cursor-default rounded-md border-0 bg-transparent font-semibold text-gray-900 decoration-2 underline-offset-2 caret-transparent outline-none ring-0 focus:underline focus:ring-0"
          v-model="query"
          spellcheck="false"
          @keydown.enter.stop.prevent="focusedNode != null && fire(focusedNode)"
          @keydown.up.stop.prevent="focus('previous')"
          @keydown.down.stop.prevent="focus('next')"
          @keydown.right.stop.prevent="onNavigateHorizontal('right')"
          @keydown.left.stop.prevent="onNavigateHorizontal('left')"
        />
      </div>
    </div>

    <!-- Nodes -->
    <ul v-if="expandedNodes.length > 0" class="my-1 flex flex-col text-gray-900">
      <!-- Node -->
      <li
        :ref="(ref?: any) => ref != null ? (expandedNodesRefs[node.id] = ref) : (delete expandedNodesRefs[node.id])"
        v-for="({ node, depth, isFocusedAbsolute: isItemFocusedAbsolute, canExpand }, i) in expandedNodes"
        :key="node.id"
        class="group mx-1 flex flex-row items-center rounded-md border py-0.5 hover:cursor-pointer hover:bg-primary-100 hover:text-primary-900"
        :class="[
          focusedNode?.id == node.id && isFocusAbsolute ? 'border-gray-300' : 'border-transparent',
          isItemFocusedAbsolute ? 'bg-gray-100' : '',
        ]"
        :style="{ paddingLeft: 8 + depth * 12 + 'px', paddingRight: 4 + 'px' }"
        role="treeitem"
        @click.stop="fire(node)"
        v-contextmenu="(context: MenuContext) => (doFocus(node), {items: menuActionsLike({wildcard: ['common.sense.*','common.edit.*']}), context: {...context, triggerNode: node}})"
      >
        <!-- Expand button (or placeholder) -->
        <button
          v-if="canExpand"
          class="group mr-1 w-5 rounded-md hover:bg-primary-200 hover:text-primary-900"
          :class="focusedNode?.id == node.id ? '' : 'text-gray-400'"
          @click.stop="toggleExpanded(node), doFocus(node)"
        >
          <i
            class="fas fa-chevron-right dxuration-75 transition-transform"
            :class="[isExpanded(node) ? 'rotate-90' : 'rotate-0']"
          />
        </button>
        <!-- Icon / title -->
        <IconInline
          v-bind="(node as any).icon ?? getNodeIcon(node)"
          class="mr-1.5"
          :class="[
            isItemFocusedAbsolute ? 'text-primary-900' : 'text-gray-500 group-hover:text-primary-900',
            canExpand ? '' : 'ml-6',
          ]"
        />
        <span
          class="select-none truncate"
          :class="isItemFocusedAbsolute ? 'font-semibold text-primary-900' : 'group-hover:text-primary-900'"
          v-html="nodeTitleMarked[i] ?? (node as any).name ?? node.id"
        />
        <!-- Status/Notices/...? -->
        <!-- ... -->
      </li>
    </ul>
    <div
      v-else-if="type == ViewType.EXPLORER || inspectionPtr != null"
      class="flex h-full w-full flex-col justify-center bg-white text-center"
    >
      <!-- Empty state -->
      <i class="fas fa-empty-set text-gray-500" />
      <span class="text-gray-600">Nothing Here</span>
    </div>
    <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
      <!-- Missing state -->
      <i class="fas fa-empty-set text-gray-500" />
      <span class="text-gray-600">Select Node to Inspect</span>
    </div>
  </Scroll>
</template>
