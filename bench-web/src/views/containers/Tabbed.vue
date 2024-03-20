<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire";
import { toNodeReference } from "@/proto/wiring";
import { type ActionMapImplementation } from "@/system/action";
import { useLoadedGraph } from "@/system/connection";
import { IconInline } from "@/system/icon";
import { addView, removeView, spaceRegistry, splitView } from "@/system/space";
import { setDragData, useMultiDropZone, useSplitDropZone } from "@/utils/drag";
import { ScrollbarWidth } from "@/utils/layout";
import { getViewBinding, getViewComponent } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, nextTick, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<ViewData, "focus">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

// focus
const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(self);
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
  spaceRegistry.focus(spaceConnection.sideTx, self.value, { view: tab });
  // ensure tab is visible in header
  nextTick(() => {
    tabsRef.value[tab.id]!.scrollIntoView({ block: "nearest", inline: "nearest" });
  });
}
const isFocusAbsolute = spaceRegistry.isFocusedAbsoluteRef(self);

function remove(tab: ViewData) {
  removeView(spaceConnection.sideTx, spaceGraph, tab);
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
      addView(spaceConnection.sideTx, spaceGraph, self, draggedNode, anchor, targetId);
      focus(draggedNode);
    }
  },
});

// splitting body
// TODO :Architecture: shouldn't window splitting be implemented in Windowed?
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
        addView(spaceConnection.sideTx, spaceGraph, self, draggedNode, "end", null);
        focus(draggedNode);
      } else {
        splitView(spaceConnection.sideTx, spaceGraph, self, draggedNode, anchor);
      }
    }
  },
});

// actions
const actions: Partial<ActionMapImplementation<"view">> = {
  "view.navigate.closeTab": {
    enabled: computed(() => focusedTabIdx.value != null),
    action: () => remove(tabs.value[focusedTabIdx.value!]),
  },
  "view.navigate.closeOtherTabs": {
    enabled: computed(() => tabs.value.length > 1),
    action: () => {
      const focusedTab = tabs.value[focusedTabIdx.value!];
      for (const tab of tabs.value) {
        if (tab != focusedTab) removeView(spaceConnection.sideTx, spaceGraph, tab);
      }
    },
  },
  "view.navigate.focusPreviousTab": {
    enabled: computed(() => tabs.value.length > 1),
    action: () => {
      const newIdx = focusedTabIdx.value == 0 ? tabs.value.length - 1 : (focusedTabIdx.value ?? 1 ) - 1;
      focus(tabs.value[newIdx]);
    },
  },
  "view.navigate.focusNextTab": {
    enabled: computed(() => tabs.value.length > 1),
    action: () => {
      const newIdx = focusedTabIdx.value == tabs.value.length - 1 ? 0 : (focusedTabIdx.value ?? -1) + 1;
      focus(tabs.value[newIdx]);
    },
  },
  "view.layout.splitHorizontal": {
    enabled: computed(() => focusedTabIdx.value != null),
    action: () => {
      const selfData = spaceGraph.get(props.self) as ViewData;
      const focusedTab = tabs.value[focusedTabIdx.value!];
      splitView(spaceConnection.sideTx, spaceGraph, selfData, focusedTab, "right");
    },
  },
  "view.layout.splitVertical": {
    enabled: computed(() => focusedTabIdx.value != null),
    action: () => {
      const selfData = spaceGraph.get(props.self) as ViewData;
      const focusedTab = tabs.value[focusedTabIdx.value!];
      splitView(spaceConnection.sideTx, spaceGraph, selfData, focusedTab, "bottom");
    },
  },
};

spaceRegistry.registerCurrent(self);
defineExpose<ViewExposed>({ self, actions });
</script>
<template>
  <div class="relative select-none" :style="{ width: size.width + 'px', height: size.height + 'px' }">
    <!-- Tab header -->
    <Scroll
      ref="headerRef"
      class="scrollbar-none relative flex w-full flex-row border-b border-gray-300"
      :class="[activeHeaderDropZone != null ? 'bg-gray-100' : 'bg-gray-200']"
      :orientation="Orientation.HORIZONTAL"
      :track-width="ScrollbarWidth.sm"
      track-is-overlay
      :size="{ width: innerSize.width, height: 30 }"
    >
      <!-- Tab button -->
      <button
        :ref="(ref) => (ref != null ? (tabsRef[tab.id] = ref as HTMLElement) : delete tabsRef[tab.id])"
        v-for="(tab, i) in tabs"
        :key="tab.id"
        class="group relative flex h-full max-w-52 select-none flex-row items-center justify-center whitespace-nowrap border-r border-gray-300 bg-gray-100 px-2.5 hover:cursor-pointer"
        :class="[
          i == focusedTabIdx ? 'text-primary-900  shadow-primary-900' : ' hover:text-primary-900',
          i == focusedTabIdx ? (isFocusAbsolute ? 'shadow-inset-md' : 'shadow-inset-sm') : '',
          i != focusedTabIdx ? (isFocusAbsolute ? 'text-gray-700' : 'text-gray-500') : '',
        ]"
        @mousedown="focus(tab)"
        :draggable="true"
        @dragstart="
          (e: DragEvent) => {
            setDragData(e, { kind: 'node', node: toNodeReference(tab) });
            e.dataTransfer?.setDragImage(tabsRef[tab.id]!, 0, 0)
          }
        "
      >
        <IconInline
          v-if="tab.icon"
          v-bind="tab.icon"
          class="mr-1.5"
          :class="i == focusedTabIdx ? '' : ' group-hover:text-primary-900'"
        />
        <span class="truncate" :class="[tab.title ? '' : 'italic', i == focusedTabIdx ? '' : '']">
          {{ tab.title ?? `Tab ${i + 1}` }}
        </span>
        <!-- Close tab button -->
        <button
          class="ml-1.5 group-hover:text-gray-400"
          :class="[i == focusedTabIdx ? 'text-gray-400' : 'text-transparent']"
          @mousedown.stop="remove(tab)"
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
      enter-active-class="transition-opacity duration-300"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition-opacity duration-300"
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
