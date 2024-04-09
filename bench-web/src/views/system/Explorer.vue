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
import { makeSelection, collapseSelection, expandSelection } from "@/views/canvas";
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

const rootPtr = computed(() => {
  if (props.nodePtr != null) return props.nodePtr;
  else if (props.type == ViewType.EXPLORER) return packagePtr.value;
  else if (props.type == ViewType.OUTLINE) return inspectionPtr.value;
  else return null;
});
const { graph, connection } = useExistingConnection(rootPtr, {
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
//

const rootNodes = graph.getChildrenRef(rootPtr, NodeType.BLOCK, { ignoreAncestors: true });
const expandedNodes = rootNodes; /* nocheckin */
const focusedNode = computed(() => {
  if (props.focus?.nodesPtr.length ?? 0 > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    return expandedNodes.value.find((node) => node.id == focusedId);
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
    spaceConnection.tx.update(selfNode, {
      expansion: collapseSelection(selfNode.expansion!, [node]),
    });
  } else {
    spaceConnection.tx.update(selfNode, {
      expansion: expandSelection(selfNode.expansion, [node]),
    });
  }
}

function isExpanded(node: { id?: string; ck?: string }) {
  return props.expansion?.nodesPtr?.some((n) => n.id == node.id) ?? false;
}

function canExpand(node: AnyNodeData) {
  return true; /* nocheckin */
}

const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);
function focus(anchor: "next" | "previous" | number | FocusAnchor | NodeReferenceData): void {
  let toFocus: AnyNodeData | null = null;
  if (anchor == "top") {
    toFocus = expandedNodes.value[0];
  } else if (anchor == "bottom") {
    toFocus = expandedNodes.value[expandedNodes.value.length - 1];
  } else if (anchor == "previous") {
    const idx = expandedNodes.value.indexOf(focusedNode.value!);
    if (idx > 0) toFocus = expandedNodes.value[idx - 1];
  } else if (anchor == "next") {
    const idx = expandedNodes.value.indexOf(focusedNode.value!);
    if (idx < expandedNodes.value.length - 1) toFocus = expandedNodes.value[idx + 1];
  } else if (typeof anchor == "number") {
    toFocus = expandedNodes.value[anchor];
  }
  if (toFocus != null) {
    const selfNode = spaceGraph.getOrFail(self.value) as ViewData;
    spaceConnection.tx.update(selfNode, { focus: makeSelection([toFocus]) });
  }
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
      candidates: expandedNodes.value.map((item) => item.name ?? ""),
    });
    nodeTitleMarked.value = markedResults;

    // auto-select best match
    if (bestMatches.length > 0) focus(bestMatches[0]);
  },
  { immediate: true },
);

const actions: Partial<ActionMapImplementation<"common">> = {};

canvas.registerView(self);
defineExpose<ViewExposed>({ self, actions, focus });
</script>
<template>
  <!-- nocheckin :Incomplete: explorer/outline -->
  <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.sm" class="bg-white">
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
    <ul v-if="type == ViewType.EXPLORER || inspectionPtr != null" class="my-1 flex flex-col text-gray-900">
      <!-- Node -->
      <li
        v-for="(node, i) in expandedNodes"
        :key="node.id"
        class="group mx-1 flex flex-row items-center rounded-md border px-2 py-0.5 hover:cursor-pointer hover:text-primary-900"
        :class="focusedNode?.id == node.id && isFocusAbsolute ? 'border-gray-700' : 'border-transparent'"
        role="treeitem"
        @click="fire(node)"
      >
        <!-- Expand button -->
        <button
          class="hover:bg mr-1 w-4 rounded-sm group"
          @click.stop="toggleExpanded(node)"
          :disabled="type == ViewType.OUTLINE /* NOTE: Outline is always fully expanded */"
        >
          <i
            class="fas fa-chevron-right text-xs transition-transform duration-75"
            :class="[isExpanded(node) ? 'rotate-90' : 'rotate-0', focusedNode?.id == node.id ? '' : 'text-gray-400']"
          />
        </button>
        <!-- Icon / title -->
        <IconInline
          v-bind="node.icon ?? getNodeIcon(node)"
          class="mr-1.5"
          :class="focusedNode?.id == node.id ? 'text-primary-900' : 'text-gray-500 group-hover:text-primary-900'"
        />
        <span
          class="select-none truncate"
          :class="focusedNode?.id == node.id ? 'font-semibold text-primary-900' : 'group-hover:text-primary-900'"
          v-html="nodeTitleMarked[i] ?? node.name"
        />
      </li>
    </ul>
    <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
      <!-- Empty/missing state -->
      <i class="fas fa-empty-set text-gray-500" />
      <span class="text-gray-600">Select Node to Inspect</span>
    </div>
  </Scroll>
</template>
