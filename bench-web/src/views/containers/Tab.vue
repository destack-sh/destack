<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { type ActionMapImplementation } from "@/system/action";
import { useExistingConnection } from "@/system/connection";
import { IconInline } from "@/system/icon";
import { ICON_BY_NODE_TYPE } from "@/system/lang";
import { canvas } from "@/system/space";
import { setDragData, useMultiDropZone, useSplitDropZone, type SplitAnchor } from "@/utils/drag";
import { ScrollbarWidth } from "@/utils/layout";
import { getViewBinding, getViewComponent } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { type ContextMenuInfo, type MenuContext, menuActionsLike } from "@/utils/menu";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<ViewData, "focus">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

// focus
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const tabs = spaceGraph.getChildrenRef(self, NodeType.VIEW);

const focusedTabIdx: Ref<number | null> = computed(() => {
  if (tabs.value.length == 0) return null;
  if ((props.focus?.nodesPtr.length ?? 0) > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    const focusedTabIdx = tabs.value.findIndex((tab) => tab.id == focusedId);
    return focusedTabIdx >= 0 ? focusedTabIdx : 0;
  } else {
    return 0;
  }
});
const innerSize = computed(() => ({
  width: props.size.width,
  height: props.size.height - 30,
}));

function focus(tab: ViewData) {
  canvas.focus(spaceConnection.tx, { view: tab, parent: self.value });
  // ensure tab is visible in header
  nextTick(() => {
    tabsRef.value[tab.id]!.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
}
const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);

function remove(tab: ViewData) {
  canvas.removeView(spaceConnection.tx, spaceGraph, tab);
}

// dragging into header
const headerRef: Ref<HTMLElement | null> = ref(null);
const tabsRef: Ref<Record<string, HTMLElement>> = ref({});
const { activeDropZone: activeHeaderDropZone } = useMultiDropZone({
  container: headerRef,
  targets: tabsRef,
  kinds: ["node"],
  metatypes: [NodeType.VIEW],
  orientation: Orientation.HORIZONTAL,
  onDrop: (dragged, anchor, targetId) => {
    if (dragged.kind != "node") return;
    const draggedNode = spaceGraph.get(dragged.node) as ViewData;
    if (draggedNode != null) {
      const self = spaceGraph.get(props.self) as ViewData;
      canvas.moveView(spaceConnection.tx, spaceGraph, self, draggedNode, anchor, targetId);
      focus(draggedNode);
    }
  },
});

// splitting body
const bodyRef: Ref<HTMLElement | null> = ref(null);
const { activeDropZone: activeBodyDropZone } = useSplitDropZone({
  container: bodyRef,
  kinds: ["node"],
  metatypes: [NodeType.VIEW],
  onDrop: (dragged, anchor) => {
    if (dragged.kind != "node") return;
    const draggedNode = spaceGraph.get(dragged.node) as ViewData;
    if (draggedNode != null) {
      const self = spaceGraph.get(props.self) as ViewData;
      if (anchor == "center") {
        canvas.moveView(spaceConnection.tx, spaceGraph, self, draggedNode, "end", null);
        focus(draggedNode);
      } else {
        canvas.splitView(spaceConnection.tx, spaceGraph, self, draggedNode, anchor);
      }
    }
  },
});

// actions
const hasMultipleTabs = computed(() => tabs.value.length > 1);
const splitAction = (anchor: SplitAnchor) => ({
  action: () => {
    const focusedTab = tabs.value[focusedTabIdx.value!];
    if (focusedTab == null) return false;
    const selfData = spaceGraph.get(props.self) as ViewData;
    canvas.splitView(spaceConnection.tx, spaceGraph, selfData, focusedTab, anchor);
    return true;
  },
});
const actions: Partial<ActionMapImplementation<"view">> = {
  "view.navigate.closeTab": {
    enabled: computed(() => focusedTabIdx.value != null),
    action: () => remove(tabs.value[focusedTabIdx.value!]),
  },
  "view.navigate.closeOtherTabs": {
    enabled: hasMultipleTabs,
    action: () => {
      const focusedTab = tabs.value[focusedTabIdx.value!];
      for (const tab of tabs.value) {
        if (tab != focusedTab) canvas.removeView(spaceConnection.tx, spaceGraph, tab);
      }
    },
  },
  "view.navigate.focusPreviousTab": {
    enabled: hasMultipleTabs,
    action: () => {
      const newIdx = focusedTabIdx.value == 0 ? tabs.value.length - 1 : (focusedTabIdx.value ?? 1) - 1;
      focus(tabs.value[newIdx]);
    },
  },
  "view.navigate.focusNextTab": {
    enabled: hasMultipleTabs,
    action: () => {
      const newIdx = focusedTabIdx.value == tabs.value.length - 1 ? 0 : (focusedTabIdx.value ?? -1) + 1;
      focus(tabs.value[newIdx]);
    },
  },
  "view.layout.splitLeft": splitAction("left"),
  "view.layout.splitRight": splitAction("right"),
  "view.layout.splitUp": splitAction("top"),
  "view.layout.splitDown": splitAction("bottom"),
};

canvas.registerView(self);
defineExpose<ViewExposed>({ self, actions });
</script>
<template>
  <div class="relative select-none" :style="{ width: size.width + 'px', height: size.height + 'px' }">
    <!-- Tabs header -->
    <Scroll
      ref="headerRef"
      class="scrollbar-none relative flex w-full flex-row border-b border-gray-300"
      :class="[activeHeaderDropZone != null ? 'bg-gray-100' : 'bg-gray-200']"
      :orientation="Orientation.HORIZONTAL"
      :track-width="ScrollbarWidth.sm"
      track-is-overlay
      :size="{ width: innerSize.width, height: 30 }"
      v-contextmenu="(context: MenuContext) => ({items: menuActionsLike({wildcard: ['view.navigate*frame*', 'view.layout*']}, {context})} as ContextMenuInfo) "
    >
      <!-- Tab button -->
      <button
        :ref="(ref) => (ref != null ? (tabsRef[tab.id] = ref as HTMLElement) : delete tabsRef[tab.id])"
        v-for="(tab, i) in tabs"
        :key="tab.id"
        class="group relative flex h-full max-w-52 select-none flex-row items-center justify-center whitespace-nowrap border-r border-gray-300 bg-gray-100 px-2.5 transition-colors duration-75 hover:cursor-pointer"
        :class="[
          i == focusedTabIdx ? 'text-primary-900  shadow-primary-900' : ' hover:text-primary-900',
          i == focusedTabIdx ? (isFocusAbsolute ? 'shadow-inset-md' : '') : '',
          i != focusedTabIdx ? (isFocusAbsolute ? 'text-gray-700' : 'text-gray-500') : '',
        ]"
        @click="focus(tab)"
        :draggable="true"
        @dragstart="
          (e: DragEvent) => {
            setDragData(e, { kind: 'node', node: toNodeReference(tab) });
            e.dataTransfer?.setDragImage(tabsRef[tab.id]!, 0, 0)
          }
        "
        v-contextmenu="(context: MenuContext) => ({items: menuActionsLike({wildcard: ['view.navigate*tab*', 'view.layout*']}, {context})} as ContextMenuInfo)"
      >
        <!-- Tab header  -->
        <IconInline
          v-bind="tab.icon ?? ICON_BY_NODE_TYPE[NodeType.VIEW]"
          class="mr-1.5"
          :class="i == focusedTabIdx ? '' : ' group-hover:text-primary-900'"
        />
        <span class="truncate" :class="[tab.title ? '' : 'italic']">
          {{ tab.title ?? `Tab ${i + 1}` }}
        </span>
        <!-- Close tab -->
        <button
          class="ml-1.5 transition-colors duration-75 group-hover:text-gray-400"
          :class="i == focusedTabIdx && isFocusAbsolute ? 'text-gray-400' : 'text-transparent'"
          @click.stop="remove(tab)"
        >
          <i class="fas fa-xmark hover:text-primary-900" />
        </button>
        <!-- Drop indicator -->
        <div
          v-if="activeHeaderDropZone?.targetId == tab.id"
          class="absolute z-10 h-full w-1 bg-primary-400"
          :class="[activeHeaderDropZone.anchor == 'start' ? (i == 0 ? 'left-0' : '-left-[3px]') : '-right-[3px]']"
        />
      </button>
      <!-- Drop indicator if no tab -->
      <div
        v-if="activeHeaderDropZone != null && activeHeaderDropZone.targetId == null"
        class="absolute left-0 z-10 h-full w-1 bg-primary-400"
      />
    </Scroll>
    <!-- Tab body -->
    <div
      ref="bodyRef"
      class="absolute"
      :style="{ left: '0px', top: '30px', width: innerSize.width + 'px', height: innerSize.height + 'px' }"
      data-root-element="true"
    >
      <!-- Content -->
      <component
        v-if="focusedTabIdx != null && getViewComponent(tabs[focusedTabIdx].type) != null"
        :is="getViewComponent(tabs[focusedTabIdx].type)"
        :self="toNodeReference(tabs[focusedTabIdx])"
        v-bind="getViewBinding(tabs[focusedTabIdx], innerSize)"
      />
      <div v-else-if="focusedTabIdx != null" class="h-full w-full">
        <!-- missing view -->
      </div>
      <div v-else class="h-full w-full">
        <!-- empty state -->
      </div>
    </div>
    <!-- Tab body split drop overlay -->
    <Transition
      appear
      enter-active-class="transition-opacity ease-in duration-300"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-opacity ease-out duration-300"
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
            class="absolute z-20 transform bg-primary-400 opacity-40 transition-all duration-300"
            :class="activeBodyDropZone.splitClass"
          />
        </div>
      </div>
    </Transition>
  </div>
</template>
