<script lang="ts" setup>
import { createBlock, useHierarchicalNodeMoveActions } from "@/language/block";
import { NAME_CONSTRAINT, PAGE_BLOCK_TYPES } from "@/language/const";
import { isDescendantOf, walkDescendantsRef, type NodeTreeItem } from "@/language/graph";
import { cloneNode, moveNode } from "@/language/node";
import {
  BlockData,
  BlockType,
  BoxData,
  CHILD_NODE_TYPES,
  NodeReferenceData,
  NodeType,
  ObjectType,
  Orientation,
  TreeViewPreset,
  ViewData,
  type AnyNodeData,
} from "@/proto/wire";
import { isNode, toNodeRef, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { packagePtr } from "@/system/client";
import { useExistingConnection, type Connection } from "@/system/connection";
import { bench, canvas, inspectionBasePtr, inspectionPtr, pkg } from "@/system/space";
import type { ActionContext, ActionMapImplementation } from "@/ui/action";
import {
  DEFAULT_HEADER_HEIGHT,
  DEFAULT_MAX_WIDTH,
  DEFAULT_MIN_WIDTH,
  makeSelection,
  useExpansion,
  useViewState,
} from "@/ui/canvas";
import { startDragging, useMultiDropZone } from "@/ui/drag";
import { DEFAULT_BENCH_ICON, IconInline, getNodeIcon } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { menuActionsLike, type PopoverContext, type PopoverInfo } from "@/ui/popover";
import { highlightMatches } from "@/ui/search";
import { getNativeConstraintProps, guardNativeInput } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodePath from "@/views/builtins/NodePath.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { computed, nextTick, ref, toRef, watch, type Ref } from "vue";

const DEPTH_OFFSET = 12;
const HEADER_HEIGHT = DEFAULT_HEADER_HEIGHT;
const MIN_WIDTH = DEFAULT_MIN_WIDTH;
const MAX_WIDTH = DEFAULT_MAX_WIDTH;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "type" | "name" | "title" | "text" | "icon" | "nodePtr" | "focus" | "selection" | "expansion" | "valuePacked"
  >
>();

const containerRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const query: Ref<string> = ref("");
const queryRef: Ref<HTMLInputElement | null> = ref(null);
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph } = useExistingConnection(self);
const { state, useStateProp } = useViewState({
  selfPtr: self,
  graph: spaceGraph,
  stateType: ObjectType.TREE_VIEW_STATE,
  props,
  emit,
});
const preset = useStateProp("preset");
const filterIsPage = computed(() => {
  if (preset.value == TreeViewPreset.EXPLORE) return true;
  else if (preset.value == TreeViewPreset.OUTLINE) return false;
  else return state.value.filterIsPage;
});
const isDefaultExpanded = computed(() => preset.value == TreeViewPreset.OUTLINE || state.value.isDefaultExpanded);
const inspectedNodeTypes = computed(() => {
  if (preset.value == TreeViewPreset.EXPLORE) return [NodeType.BLOCK];
  else if (preset.value == TreeViewPreset.OUTLINE)
    return [NodeType.BLOCK, NodeType.FIELD, NodeType.VIEW, NodeType.STEP];
  else return state.value.nodeTypes;
});
const rootPtr = computedValue(() => {
  if (props.nodePtr?.oneofKind != null) return unwrapProtoOneOf(props.nodePtr);
  else if (preset.value == TreeViewPreset.EXPLORE) return packagePtr.value;
  else if (preset.value == TreeViewPreset.OUTLINE) return inspectionBasePtr.value;
  else return null;
});
const focusPtr = computedValue(() => {
  if (preset.value == TreeViewPreset.EXPLORE) return inspectionBasePtr.value;
  else if (preset.value == TreeViewPreset.OUTLINE) return inspectionPtr.value;
  else return null;
});

const { graph: pkgGraph, connection: pkgConnection } = useExistingConnection(rootPtr, {
  match: {
    predicate: (c) => {
      // NOTE: hack to exclude Bench connection (which also contains package) :ConnectionMatching
      return !(c as Connection<"get", NodeType>).params.roots.some((r) => r.type == NodeType.BENCH);
    },
  },
});

//
// Visible subtree
//

const { toggleExpanded, isExpanded } = useExpansion({
  graph: spaceGraph,
  tx: canvas.tx,
  self,
  props,
  emit,
  isDefaultExpanded,
});

function isIncludedSelf(node: AnyNodeData) {
  if (filterIsPage.value) {
    if (node.metatype == ObjectType.BLOCK) return PAGE_BLOCK_TYPES.includes((node as BlockData).type);
    else return true;
  } else {
    return true; // include everything
  }
}
function isIncludedChildren(node: AnyNodeData) {
  if (filterIsPage.value) {
    return true;
  } else if (filterIsPage.value === false) {
    // don't descend into pages for outline
    if (node.metatype == ObjectType.BLOCK) return !PAGE_BLOCK_TYPES.includes((node as BlockData).type);
    else return true;
  } else {
    return true; // include everything
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

const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);
const editingNodePtr: Ref<NodeReferenceData | null> = ref(null);
const editingNameRef: Ref<HTMLInputElement[]> = ref([]);

function cancelRename() {
  editingNodePtr.value = null;
  queryRef.value?.focus();
}

watch(isFocusAbsolute, (isFocused) => {
  if (!isFocused) {
    cancelRename();
  }
});

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
    const selfNode = spaceGraph.getOrError(self.value) as ViewData;
    canvas.tx().update(selfNode, { focus: makeSelection([node]) }, { debounce: "tick" });
  }
  queryRef.value?.focus();
  expandedNodesRefs.value[node.id!]?.scrollIntoView({ block: "center", behavior: "instant" });
}

function clear() {
  query.value = "";
}

function fire(node: AnyNodeData) {
  canvas.goToNode(node, { where: "bestFrame", skipSelf: preset.value == TreeViewPreset.OUTLINE });
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
    const target = targetId != null ? pkgGraph.get({ id: targetId }) : null;
    if (target == null || isDescendantOf(pkgGraph, target, dragged.node)) return false;
    const targetParentType = anchor == "center" ? (target.metatype as unknown as NodeType) : target.parentPtr!.type;
    if (!CHILD_NODE_TYPES[targetParentType].includes(dragged.node.type)) return false;
    return true;
  },
  onDrop: (dragged, anchor, targetId) => {
    if (targetId != null && dragged.kind == "node") {
      const target = pkgGraph.getOrError({ id: targetId });
      moveNode(pkgConnection.tx, pkgGraph, dragged.node, { anchor, target });
    }
  },
});

// actions
const getItemFromContext = (ctx: ActionContext | undefined): { item: NodeTreeItem<any> | null; idx: number } => {
  let item = expandedItems.value.find((item) => item.node.id == ctx?.triggerNode?.id);
  if (!item) item = expandedItems.value.find((item) => item.node.id == focusedItem.value?.node.id);
  if (!item) return { item: null, idx: -1 };
  const idx = expandedItems.value.indexOf(item);
  return { item, idx };
};
const actions: Partial<ActionMapImplementation<"common">> = {
  "common.sense.focus": {
    action: (action, ctx) => {
      const node = getItemFromContext(ctx).item?.node;
      if (node == null) return false;
      canvas.goToNode(node, { where: "bestFrame", skipSelf: preset.value == TreeViewPreset.OUTLINE });
    },
  },
  "common.edit.rename": {
    action: (action, ctx) => {
      const node = getItemFromContext(ctx).item?.node;
      if (node == null) return false;
      editingNodePtr.value = toNodeRef(node);
      nextTick(() => {
        editingNameRef.value?.[0]?.focus?.();
        editingNameRef.value?.[0]?.select?.();
      });
    },
  },
  "common.edit.duplicate": {
    action: (action, ctx) => {
      const node = getItemFromContext(ctx).item?.node;
      if (node == null) return false;
      const duplicate = cloneNode(pkgConnection.tx, pkgGraph, node, { includeChildren: true });
      nextTick(() => focus(duplicate));
    },
  },
  "common.edit.archive": {
    action: (action, ctx) => {
      const node = getItemFromContext(ctx).item?.node;
      if (node == null) return false;
      pkgConnection.tx.archive(node);
    },
  },
  "common.edit.delete": {
    action: (action, ctx) => {
      const node = getItemFromContext(ctx).item?.node;
      if (node == null) return false;
      pkgConnection.tx.delete(node);
    },
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
  <div class="h-full w-full">
    <!-- Header -->
    <div class="group w-full" :style="{ height: DEFAULT_HEADER_HEIGHT + 'px' }">
      <div
        class="mx-auto flex h-full max-w-full flex-row items-center pl-1.5 pr-3"
        :style="{ minWidth: DEFAULT_MIN_WIDTH + 'px', maxWidth: DEFAULT_MAX_WIDTH + 'px' }"
      >
        <!-- Location -->
        <!-- NOTE :UX: should probably be only node crumb in explorer header? -->
        <div v-if="preset == TreeViewPreset.EXPLORE" class="flex flex-row items-center px-1">
          <IconInline
            v-bind="bench != null ? getNodeIcon(bench) : DEFAULT_BENCH_ICON"
            class="mr-1.5 w-5 text-gray-600"
          />
          <span class="text-gray-900">{{ bench?.name ?? "???" }}</span>
        </div>
        <NodePath v-else :focus="rootPtr" :graph="pkgGraph" class="px-0.5" />
        <!-- Controls -->
        <div class="ml-auto flex flex-row items-center pl-1.5">
          <!-- Create -->
          <button
            v-if="pkg != null"
            class="rounded py-0.5 text-gray-400 hover:text-primary-900"
            @click="
              () => {
                // NOTE: we assume that pkg == pkgGraph root here (may be incorrect later)
                if (pkg == null) return;
                const block = createBlock(pkgConnection.tx, pkgGraph, {
                  anchor: 'inside',
                  target: pkg,
                  block: { type: BlockType.PAGE },
                });
                canvas.goToNode(block, { where: 'bestFrame', ifPresent: 'upsertAndFocus' });
              }
            "
          >
            <i class="fas fa-plus" />
          </button>
        </div>
      </div>
    </div>
    <!-- Magic floating query -->
    <!-- Captures focus for navigation & typing for search/highlight -->
    <div class="relative">
      <div class="absolute -top-4 left-0 px-2 pl-4">
        <input
          ref="queryRef"
          v-model="query"
          class="max-w-60 cursor-default rounded border-0 bg-transparent font-semibold text-gray-900 decoration-2 underline-offset-3 caret-transparent outline-none ring-0 focus:underline focus:ring-0"
          spellcheck="false"
          :data-suppress-actions="'common.navigate' /* allow select & move */"
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
    <Scroll
      :size="{ width: size.width, height: size.height - HEADER_HEIGHT }"
      :orientation="Orientation.VERTICAL"
      :track-width="ScrollbarWidth.md"
      track-is-overlay
      class=""
      @click.stop="queryRef?.focus()"
    >
      <!-- Nodes -->
      <ul v-if="expandedItems.length > 0" ref="containerRef" class="group/list mb-1 flex flex-col text-gray-900">
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
                    'common.sense.focus*',
                    'common.edit.rename',
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
          class="group relative mx-1 flex flex-row items-center rounded border py-[3px] hover:cursor-pointer hover:bg-gray-100 hover:text-primary-900 data-[dragging=true]:opacity-50"
          :class="[
            (focusedNode?.id == node.id && isFocusAbsolute) ||
            (activeDropZone?.targetId == node.id && activeDropZone?.anchor == 'center')
              ? 'border-primary-900'
              : 'border-transparent',
            isFocused(node) ? 'bg-gray-100' : '',
          ]"
          :style="{ paddingLeft: 4 + depth * DEPTH_OFFSET + 'px', paddingRight: 4 + 'px' }"
          role="treeitem"
          :draggable="true"
          @click.stop="fire(node)"
          @dragstart.stop="(e: DragEvent) => startDragging(e, pkgGraph, node)"
        >
          <!-- Drop indicator -->
          <div
            v-if="activeDropZone?.targetId == node.id && activeDropZone?.anchor != 'center'"
            class="absolute z-10 h-1 rounded-sm bg-primary-900"
            :class="[activeDropZone?.anchor == 'start' ? (i == 0 ? 'top-0' : '-top-[4px]') : '-bottom-[3px]']"
            :style="{
              left: 8 + depth * DEPTH_OFFSET + 'px',
              width: 'calc(100% - ' + (8 + depth * DEPTH_OFFSET) + 'px)',
            }"
          />
          <!-- Expand button (or placeholder) -->
          <button
            v-if="hasChildren"
            class="group mr-1 w-5 rounded enabled:hover:text-primary-900"
            :class="focusedNode?.id == node.id ? '' : 'text-gray-400'"
            :disabled="isDefaultExpanded"
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
            class="mr-1.5 w-5 flex-shrink-0"
            :class="[
              isFocused(node) ? 'text-primary-900' : 'text-gray-700 group-hover:text-primary-900',
              hasChildren ? '' : 'ml-6',
            ]"
          />
          <!-- Name (editable) if editing -->
          <input
            v-if="node.id == editingNodePtr?.id"
            ref="editingNameRef"
            v-outside.mousedown.stop="cancelRename"
            class="flex-1 rounded border-0 bg-transparent outline-none ring-0 hover:bg-gray-100 focus:ring-0"
            spellcheck="false"
            :value="(node as any).name"
            v-bind="getNativeConstraintProps(NAME_CONSTRAINT)"
            @click.stop
            @keydown.enter.stop.prevent="cancelRename"
            @keydown.escape.stop.prevent="cancelRename"
            @input="
              guardNativeInput(NAME_CONSTRAINT, $event, (node as any).name, (newValue) => {
                pkgConnection.tx.update(node!, { name: newValue }, { debounce: 'long' });
              })
            "
          />
          <!-- Name otherwise -->
          <span
            v-else
            class="select-none truncate group-hover:text-primary-900"
            :class="isFocused(node) ? 'text-primary-900' : ''"
            v-html="nodeTitlesMarked[i] ?? (node as any).name ?? node.id"
          />
          <!-- Meta -->
          <div class="ml-auto flex flex-row gap-x-1 pl-3 pr-[3px]">
            <!-- Create inside -->
            <button
              v-if="isNode(node, NodeType.BLOCK)"
              role="button"
              class="text-gray-400 opacity-0 hover:text-primary-900 group-hover:opacity-100"
              @click.stop="
                () => {
                  const block = createBlock(pkgConnection.tx, pkgGraph, {
                    anchor: 'inside',
                    target: node,
                    block: { type: BlockType.PAGE },
                  });
                  canvas.goToNode(block, { where: 'bestFrame', ifPresent: 'upsertAndFocus' });
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
      <div v-else class="flex h-full w-full flex-col justify-center text-center">
        <!-- Missing state -->
        <span>
          <i class="fas fa-empty-set text-gray-500" />
          <span class="ml-1.5 text-gray-600">{{
            rootPtr != null ? "Nothing Here Yet" : "Select Node to Inspect"
          }}</span>
        </span>
      </div>
    </Scroll>
  </div>
</template>
