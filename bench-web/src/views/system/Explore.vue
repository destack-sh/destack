<script lang="ts" setup>
import {
  ObjectType,
  BlockData,
  BoxData,
  NodeReferenceData,
  NodeType,
  Orientation,
  ViewData,
  ViewType,
  type AnyNodeData,
  CHILD_NODE_TYPES,
  BlockType,
} from "@/proto/wire";
import { isNode, type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionContext, ActionMapImplementation } from "@/system/action";
import { useHierarchicalNodeMoveActions } from "@/system/block";
import { packagePtr } from "@/system/client";
import { useExistingConnection, type GraphConnection } from "@/system/connection";
import { isDescendantOf, walkDescendantsRef, type NodeTreeItem } from "@/system/graph";
import { IconInline, getNodeIcon } from "@/system/icon";
import { createBlock, moveNode } from "@/system/lang";
import { highlightMatches } from "@/system/search";
import { inspectionBasePtr, canvas, inspectionPtr, pkg } from "@/system/space";
import { startDragging, useMultiDropZone } from "@/utils/drag";
import { ScrollbarWidth } from "@/utils/layout";
import { menuActionsLike, type PopoverContext, type PopoverInfo } from "@/utils/menu";
import { computedValue } from "@/utils/ref";
import { makeSelection, useExpansion } from "@/views/canvas";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { computed, ref, toRef, watch, type Ref } from "vue";

const DEPTH_OFFSET = 12;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "type" | "name" | "title" | "text" | "icon" | "nodePtr" | "focus" | "selection" | "expansion"
  >
>();

const containerRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);

const rootPtr = computedValue(() => {
  if (props.nodePtr != null) return props.nodePtr;
  else if (props.type == ViewType.EXPLORE) return packagePtr.value;
  else if (props.type == ViewType.OUTLINE) return inspectionBasePtr.value;
  else return null;
});
const focusPtr = computedValue(() => {
  if (props.type == ViewType.EXPLORE) return inspectionBasePtr.value;
  else if (props.type == ViewType.OUTLINE) return inspectionPtr.value;
  else return null;
});

const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(rootPtr, {
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

const { toggleExpanded, isExpanded } = useExpansion({
  graph: spaceGraph,
  connection: spaceConnection,
  self,
  isDefaultExpanded: props.type == ViewType.OUTLINE,
});
const inspectedNodeTypes = computed(() => {
  if (props.type == ViewType.EXPLORE) return [NodeType.BLOCK];
  else if (props.type == ViewType.OUTLINE) return [NodeType.BLOCK, NodeType.FIELD, NodeType.VIEW, NodeType.STEP];
  else return [];
});
function isIncludedSelf(node: AnyNodeData) {
  if (props.type == ViewType.EXPLORE) {
    if (node.metatype == ObjectType.BLOCK) return (node as BlockData).isPage;
    else return true;
  } else if (props.type == ViewType.OUTLINE) {
    return true; // include everything
  } else {
    throw new Error(`unexpected view type: ${props.type}`);
  }
}
function isIncludedChildren(node: AnyNodeData) {
  if (props.type == ViewType.EXPLORE) {
    return true;
  } else if (props.type == ViewType.OUTLINE) {
    // don't descend into pages for outline
    if (node.metatype == ObjectType.BLOCK) return !(node as BlockData).isPage;
    else return true;
  } else {
    throw new Error(`unexpected view type: ${props.type}`);
  }
}
const { items: expandedItems } = walkDescendantsRef({
  graph: pkgGraph,
  rootPtr,
  nodeTypes: inspectedNodeTypes,
  isExpanded,
  isIncludedSelf,
  isIncludedChildren,
  watchSource: () => [props.focus, props.expansion],
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

function isFocusedAbsolute(node: { id?: string }): boolean {
  return node.id == focusPtr.value?.id;
}

const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);
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
    const selfNode = spaceGraph.getOrError(self.value) as ViewData;
    spaceConnection.tx.updateDebounced(selfNode, { focus: makeSelection([node]) });
  }
  queryRef.value?.focus();
  expandedNodesRefs.value[node.id!]?.scrollIntoView({ block: "center", behavior: "instant" });
}

function clear() {
  query.value = "";
}

function fire(node: AnyNodeData) {
  canvas.goToNode(node, { where: "nextFrameRoot", skipSelf: props.type == ViewType.OUTLINE });
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
  container: containerRef,
  targets: expandedNodesRefs,
  orientation: Orientation.VERTICAL,
  hasCenterAnchor: true,
  fallbackToClosest: true,
  kinds: ["node"],
  metatypes: inspectedNodeTypes,
  allowDrop: (dragged, anchor, targetId) => {
    if (dragged.kind != "node") return false;
    const target = pkgGraph.get({ id: targetId });
    if (target == null || isDescendantOf(pkgGraph, target, dragged.node)) return false;
    const targetParentType = anchor == "center" ? (target.metatype as unknown as NodeType) : target.parentPtr!.type;
    if (!CHILD_NODE_TYPES[targetParentType].includes(dragged.node.type)) return false;
    return true;
  },
  onDrop: (dragged, anchor, targetId) => {
    if (targetId != null && dragged.kind == "node") {
      const target = pkgGraph.getOrError({ id: targetId });
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, anchor, target);
    }
  },
});

// actions
const hasFocusedNode = computed(() => focusedNode.value != null);
const getItemFromContext = (contet: ActionContext): { item: NodeTreeItem<any> | null; idx: number } => {
  let item = expandedItems.value.find((item) => item.node.id == contet.triggerNode?.id);
  if (!item) item = expandedItems.value.find((item) => item.node.id == focusedItem.value?.node.id);
  if (!item) return { item: null, idx: -1 };
  const idx = expandedItems.value.indexOf(item);
  return { item, idx };
};
const actions: Partial<ActionMapImplementation<"common">> = {
  "common.sense.focus": {
    isEnabled: hasFocusedNode,
    action: () =>
      canvas.goToNode(focusedNode.value!, { where: "currentRoot", skipSelf: props.type == ViewType.OUTLINE }),
  },
  "common.sense.focusInSplit": {
    isEnabled: hasFocusedNode,
    action: () =>
      canvas.goToNode(focusedNode.value!, { where: "nextFrameRoot", skipSelf: props.type == ViewType.OUTLINE }),
  },
  // <!-- TODO :Incomplete: Explorer/Outline actions -->
  // common.edit.rename, ...
  "common.edit.archive": {
    isEnabled: hasFocusedNode,
    action: () => pkgConnection.tx.archive(focusedNode.value!),
  },
  "common.edit.delete": {
    isEnabled: hasFocusedNode,
    action: () => pkgConnection.tx.softDelete(focusedNode.value!),
  },
  ...useHierarchicalNodeMoveActions({
    graph: pkgGraph,
    basePtr: rootPtr,
    txFactory: () => pkgConnection.tx,
    expandedItems,
    getItemFromContext,
  }),
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
          v-model="query"
          class="max-w-60 cursor-default rounded border-0 bg-transparent font-semibold text-gray-900 decoration-2 underline-offset-2 caret-transparent outline-none ring-0 focus:underline focus:ring-0"
          spellcheck="false"
          :data-suppress-actions="'common.edit,common.navigate' /* allow select & move */"
          @keydown.enter.stop.prevent="focusedNode != null && fire(focusedNode)"
          @keydown.up.stop.prevent="focus('previous')"
          @keydown.down.stop.prevent="focus('next')"
          @keydown.right.stop.prevent="onNavigateHorizontal('right')"
          @keydown.left.stop.prevent="onNavigateHorizontal('left')"
        />
      </div>
    </div>

    <!-- Nodes -->
    <ul v-if="expandedItems.length > 0" ref="containerRef" class="group/list my-1 flex flex-col text-gray-900">
      <!-- Node -->
      <li
        v-for="({ node, depth, hasChildren }, i) in expandedItems"
        :ref="(ref?: any) => (ref != null ? (expandedNodesRefs[node.id] = ref) : delete expandedNodesRefs[node.id])"
        :key="node.id"
        v-contextmenu="
          (context: PopoverContext): PopoverInfo => {
            doFocus(node);
            context = { ...context, triggerNode: node };
            return {
              kind: 'menu',
              placement: 'bottom-right',
              items: menuActionsLike(
                [
                  'common.sense.*',
                  'common.edit.morph',
                  'common.edit.move',
                  'common.edit.duplicate',
                  'common.edit.archive',
                  'common.edit.delete',
                ],
                { context },
              ),
              context,
            };
          }
        "
        class="group relative mx-1 mt-[1px] flex flex-row items-center rounded border py-0.5 hover:cursor-pointer hover:bg-gray-100 hover:text-primary-900 data-[dragging=true]:opacity-50"
        :class="[
          focusedNode?.id == node.id && isFocusAbsolute ? 'border-orange-900' : 'border-transparent',
          isFocusedAbsolute(node) ? 'bg-gray-100' : '',
          activeDropZone?.targetId == node.id && activeDropZone?.anchor == 'center'
            ? 'border-primary-400 bg-primary-200'
            : '',
        ]"
        :style="{ paddingLeft: 8 + depth * DEPTH_OFFSET + 'px', paddingRight: 4 + 'px' }"
        role="treeitem"
        :draggable="true"
        @click.stop="fire(node)"
        @dragstart.stop="(e: DragEvent) => startDragging(e, pkgGraph, node)"
      >
        <!-- Drop indicator -->
        <div
          v-if="activeDropZone?.targetId == node.id && activeDropZone?.anchor != 'center'"
          class="absolute z-10 h-1 rounded-sm bg-primary-400"
          :class="[activeDropZone?.anchor == 'start' ? (i == 0 ? 'top-0' : '-top-[4px]') : '-bottom-[3px]']"
          :style="{ left: 8 + depth * DEPTH_OFFSET + 'px', width: 'calc(100% - ' + (8 + depth * DEPTH_OFFSET) + 'px)' }"
        />
        <!-- Expand button (or placeholder) -->
        <button
          v-if="hasChildren"
          class="group mr-1 w-5 rounded enabled:hover:text-primary-900"
          :class="focusedNode?.id == node.id ? '' : 'text-gray-400'"
          :disabled="props.type == ViewType.OUTLINE"
          @click.stop="toggleExpanded(node), doFocus(node)"
        >
          <i
            class="fas fa-chevron-right dxuration-75 transition-transform"
            :class="[isExpanded(node) ? 'rotate-90' : 'rotate-0']"
          />
        </button>
        <!-- Icon / title -->
        <IconInline
          v-bind="getNodeIcon(node)"
          class="mr-1.5 w-5"
          :class="[
            isFocusedAbsolute(node) ? 'text-primary-900' : 'text-gray-700 group-hover:text-primary-900',
            hasChildren ? '' : 'ml-6',
          ]"
        />
        <span
          class="select-none truncate group-hover:text-primary-900"
          :class="isFocusedAbsolute(node) ? 'text-primary-900' : ''"
          v-html="nodeTitlesMarked[i] ?? (node as any).name ?? node.id"
        />
        <!-- Meta -->
        <div class="ml-auto flex flex-row gap-x-1 pl-3">
          <!-- Create inside -->
          <button
            v-if="type == ViewType.EXPLORE && isNode(node, NodeType.BLOCK)"
            role="button"
            class="text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
            @click.stop="
              () => {
                const block = createBlock(pkgConnection.tx, pkgGraph, { type: BlockType.PAGE }, 'inside', node);
                canvas.goToNode(block, { where: 'nextFrameRoot', ifPresent: 'upsertAndFocus' });
                if (!isExpanded(node)) toggleExpanded(node);
              }
            "
          >
            <i class="fas fa-plus" />
          </button>
        </div>
        <!-- ... -->
      </li>
    </ul>
    <div v-else-if="type == ViewType.EXPLORE" class="flex h-full w-full flex-col justify-center bg-white text-center">
      <!-- Empty state -->
      <button
        class="mx-auto flex flex-row items-center rounded px-2 py-0.5 text-gray-700 hover:text-primary-900"
        @click="
          () => {
            // NOTE: we assume that pkg == pkgGraph root here (may be incorrect later)
            if (pkg == null) return;
            const block = createBlock(pkgConnection.tx, pkgGraph, { type: BlockType.PAGE }, 'inside', pkg);
            canvas.goToNode(block, { where: 'nextFrameRoot', ifPresent: 'upsertAndFocus' });
          }
        "
      >
        <i class="fas fa-plus" />
        <span class="ml-2.5">Page</span>
      </button>
    </div>
    <div v-else class="flex h-full w-full flex-col justify-center bg-white text-center">
      <!-- Missing state -->
      <span>
        <i class="fas fa-empty-set text-gray-500" />
        <span class="ml-1.5 text-gray-600">Select Node to Inspect</span>
      </span>
    </div>
  </Scroll>
</template>
