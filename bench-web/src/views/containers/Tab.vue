<script lang="ts" setup>
import { cloneNode } from "@/language/core/node";
import { newChangeId } from "@/language/runtime/transaction";
import { NodeType, Orientation, RectangleData, ViewData, ViewType } from "@/proto/wire";
import { isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/globals";
import { canvas, spaceGraph } from "@/system/space";
import { type Action, type ActionContext, type ActionMapImplementation } from "@/ui/action";
import { startDraggingIfAllowed, useMultiDropZone, useSplitDropZone, type SplitAnchor } from "@/ui/drag";
import { getNodeName, ICON_BY_NODE_TYPE, ICON_BY_VIEW_TYPE, IconInline } from "@/ui/icon";
import { ScrollbarWidth } from "@/ui/layout";
import { IS_DEV, IS_DEVELOPER_MODE } from "@/utils/globals";
import Empty from "@/views/builtins/Empty.vue";
import { type ViewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { getViewBinding, getViewComponent } from "@/views/registry";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const HEADER_HEIGHT = 32; // NOTE :UX: should tab header height == default header height? (weirdly big but consistent)

const props = defineProps<
  {
    self: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    size: Required<Pick<RectangleData, "width" | "height">>;
  } & Pick<ViewData, "focus">
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

// focus
const tabs = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });
const tabsNodes = supergraph.getManyRef(computed(() => tabs.value.map((t) => t.nodePtr).filter((n) => n != null)));
const tabsNames = computed(() => {
  const tabsNames: string[] = [];
  for (let tabIdx = 0; tabIdx < tabs.value.length; tabIdx++) {
    const tab = tabs.value[tabIdx];
    const base = tabsNodes.value.find((n) => n.id == tab.nodePtr?.id);
    const name = getNodeName(tab, { base }) ?? "???";
    tabsNames.push(name);
  }
  return tabsNames;
});
const focusedTabIdx: Ref<number | null> = computed(() => {
  if (tabs.value.length == 0) {
    return null;
  } else if ((props.focus?.nodesPtr.length ?? 0) > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    const focusedTabIdx = tabs.value.findIndex((tab) => tab.id == focusedId);
    return focusedTabIdx >= 0 ? focusedTabIdx : 0;
  } else {
    return 0;
  }
});
const innerSize = computed(() => ({ width: props.size.width, height: props.size.height - HEADER_HEIGHT }));

function focus(tab: ViewData) {
  canvas.focus({ node: tab });
  // ensure tab is visible in header
  nextTick(() => {
    tabsRef.value[tab.id]!.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
}
const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);

function remove(tab: ViewData) {
  canvas.removeView(spaceGraph, tab);
}

// dragging into header
const headerRef: Ref<HTMLElement | null> = ref(null);
const tabsRef: Ref<Record<string, HTMLElement>> = ref({});
const { activeDropZone: activeHeaderDropZone } = useMultiDropZone({
  name: "tab.header",
  container: headerRef,
  targetsById: tabsRef,
  kinds: ["node", "selection"],
  metatypes: [NodeType.VIEW],
  orientation: Orientation.HORIZONTAL,
  fallbackToClosest: true,
  onDrop: (dragged, anchor, targetId) => {
    if (dragged.kind != "node" && dragged.kind != "selection") return;
    const tx = canvas.tx().with({ change: { key: newChangeId(), title: "Move" } });
    for (let node of dragged.nodes) {
      node = spaceGraph.getOrError(node);
      if (!isNode(node, NodeType.VIEW)) continue;
      const self = spaceGraph.get(props.self) as ViewData;
      canvas.moveView(
        spaceGraph,
        { self, child: node, anchor: anchor as "start" | "end", referenceId: targetId },
        { tx },
      );
      focus(node);
    }
  },
});

// splitting body
const bodyRef: Ref<HTMLElement | null> = ref(null);
const { activeDropZone: activeBodyDropZone } = useSplitDropZone({
  name: "tab.body",
  container: bodyRef,
  kinds: ["node"],
  metatypes: [NodeType.VIEW],
  onDrop: (dragged, anchor) => {
    if (dragged.kind != "node") return;
    const draggedNode = spaceGraph.get(dragged.node) as ViewData;
    if (draggedNode != null) {
      const self = spaceGraph.get(props.self) as ViewData;
      if (anchor == "center") {
        canvas.moveView(spaceGraph, { self, child: draggedNode, anchor: "end" });
        focus(draggedNode);
      } else {
        canvas.splitView(spaceGraph, { parent: self, child: draggedNode, anchor });
      }
    }
  },
});

// actions
const hasMultipleTabs = computed(() => tabs.value.length > 1);
const splitAction = (anchor: SplitAnchor) => ({
  action: (action: Action, ctx: ActionContext) => {
    const tab = ctx.nodes?.[0];
    if (!isNode(tab, NodeType.VIEW)) return false;
    const selfData = spaceGraph.get(props.self) as ViewData;
    canvas.splitView(spaceGraph, { parent: selfData, child: tab, anchor });
    return true;
  },
});
const actions: Partial<ActionMapImplementation<"view">> = {
  "view.navigate.duplicateTab": {
    isEnabled: computed(() => focusedTabIdx.value != null),
    action: (action, ctx) => {
      const tab = ctx.nodes?.[0];
      if (!isNode(tab, NodeType.VIEW)) return false;
      const clonedTab = cloneNode(canvas.tx(), spaceGraph, tab);
      focus(clonedTab);
    },
  },
  "view.navigate.closeTab": {
    isEnabled: computed(() => focusedTabIdx.value != null),
    action: (action, ctx) => {
      const tab = ctx.nodes?.[0];
      if (!isNode(tab, NodeType.VIEW)) return false;
      remove(tab);
    },
  },
  "view.navigate.closeOtherTabs": {
    isEnabled: hasMultipleTabs,
    action: (action, ctx) => {
      const tab = ctx.nodes?.[0];
      if (!isNode(tab, NodeType.VIEW)) return false;
      for (const t of tabs.value) {
        if (t != tab) canvas.removeView(spaceGraph, t);
      }
    },
  },
  "view.navigate.focusPreviousTab": {
    isEnabled: hasMultipleTabs,
    action: (action, ctx) => {
      const tab = ctx.nodes?.[0];
      if (!isNode(tab, NodeType.VIEW)) return false;
      const newIdx = (tabs.value.indexOf(tab) - 1 + tabs.value.length) % tabs.value.length;
      focus(tabs.value[newIdx]);
    },
  },
  "view.navigate.focusNextTab": {
    isEnabled: hasMultipleTabs,
    action: (action, ctx) => {
      const tab = ctx.nodes?.[0];
      if (!isNode(tab, NodeType.VIEW)) return false;
      const newIdx = (tabs.value.indexOf(tab) + 1) % tabs.value.length;
      focus(tabs.value[newIdx]);
    },
  },
  "view.layout.splitLeft": splitAction("left"),
  "view.layout.splitRight": splitAction("right"),
  "view.layout.splitUp": splitAction("top"),
  "view.layout.splitDown": splitAction("bottom"),
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, actions });
</script>
<template>
  <div class="relative" :style="{ width: size.width + 'px', height: size.height + 'px' }">
    <!-- Tabs header -->
    <Scroll
      id="tabHeader"
      ref="columnHeaderRef"
      data-contextmenu-items="view.navigate*close*frame*,view.layout*"
      class="scrollbar-none relative flex w-full select-none flex-row bg-gray-100"
      :orientation="Orientation.HORIZONTAL"
      :track-width="ScrollbarWidth.sm"
      track-is-overlay
      :size="{ width: innerSize.width }"
      :style="{ height: HEADER_HEIGHT + 'px' }"
    >
      <!-- Tab button -->
      <div
        v-for="(tab, i) in tabs"
        :ref="(ref) => (ref != null ? (tabsRef[tab.id] = ref as HTMLElement) : delete tabsRef[tab.id])"
        :key="tab.id"
        class="group relative flex h-full max-w-52 select-none flex-row items-center whitespace-nowrap border-r border-gray-200 px-2.5 hover:cursor-pointer"
        :class="[
          // we grow a single tab to the full width of the tabbed view
          i == focusedTabIdx ? 'bg-white text-gray-900' : 'border-b',
          i != focusedTabIdx ? 'text-gray-600' : '',
        ]"
        data-contextmenu-items="view.navigate*close*tab*,view.navigate.duplicateTab,view.layout*"
        data-suppress-drag="select"
        :data-node-type="tab.metatype"
        :data-node-id="tab.id"
        :data-node-ck="tab.id"
        :draggable="true"
        @click="focus(tab)"
        @dragstart.stop="(e: DragEvent) => startDraggingIfAllowed(e, tab)"
      >
        <!-- Tab header  -->
        <IconInline
          v-bind="tab.icon ?? ICON_BY_VIEW_TYPE[tab.type] ?? ICON_BY_NODE_TYPE[NodeType.VIEW]"
          class="mr-1.5 w-5"
        />
        <span class="truncate" :class="[tabsNames[i] == tab.name ? 'italic' : '']">{{ tabsNames[i] }}</span>
        <!-- Close tab -->
        <button
          class="ml-1.5 group-hover:text-gray-400"
          :class="i == focusedTabIdx && isFocusAbsolute ? 'text-gray-400' : 'text-transparent'"
          @click.stop="remove(tab)"
        >
          <i class="fas fa-xmark hover:text-gray-700" />
        </button>
        <!-- Drop indicator -->
        <div
          v-if="activeHeaderDropZone?.targetId == tab.id"
          class="absolute z-10 h-full w-1 bg-primary-500"
          :class="[activeHeaderDropZone.anchor == 'start' ? (i == 0 ? 'left-0' : '-left-[3px]') : '-right-[3px]']"
        />
      </div>
      <!-- Remaining space -->
      <div class="flex-1 border-b border-gray-200" />
      <!-- Drop indicator if no tab -->
      <div
        v-if="activeHeaderDropZone != null && activeHeaderDropZone.targetId == null"
        class="absolute left-0 z-10 h-full w-1 bg-primary-700"
      />
    </Scroll>
    <!-- Tab body -->
    <div
      ref="bodyRef"
      class="absolute bg-white"
      :style="{
        left: '0px',
        top: `${HEADER_HEIGHT}px`,
        width: innerSize.width + 'px',
        height: innerSize.height + 'px',
      }"
      data-root-element="true"
    >
      <!-- Content -->
      <component
        :is="getViewComponent(tabs[focusedTabIdx].type)"
        v-if="focusedTabIdx != null && getViewComponent(tabs[focusedTabIdx].type) != null"
        :self="toNodeRef(tabs[focusedTabIdx])"
        v-bind="getViewBinding(tabs[focusedTabIdx], innerSize)"
      />
      <div
        v-else-if="focusedTabIdx != null"
        class="flex h-full w-full flex-col justify-center bg-danger-300 text-center"
      >
        <!-- missing view -->
        <span v-if="IS_DEV || IS_DEVELOPER_MODE" class="font-mono font-semibold">
          {{ ViewType[tabs[focusedTabIdx].type] }}
        </span>
      </div>
      <Empty v-else :type="ViewType.TAB" class="flex h-full w-full flex-col items-center justify-center" />
    </div>
    <!-- Tab body split drop overlay -->
    <Transition
      appear
      enter-active-class="transition-opacity ease-in duration-150"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-opacity ease-out duration-100"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="activeBodyDropZone != null"
        class="pointer-events-none absolute"
        :style="{ left: '0px', top: '30px', width: innerSize.width + 'px', height: innerSize.height + 'px' }"
      >
        <div class="relative h-full w-full">
          <div
            class="absolute z-20 transform bg-gray-400 opacity-40 transition-all duration-100"
            :class="activeBodyDropZone.splitClass"
          />
        </div>
      </div>
    </Transition>
  </div>
</template>
