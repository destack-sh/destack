<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire";
import type { ActionMapImplementation } from "@/system/action";
import { useLoadedGraph } from "@/system/connection";
import { canvas } from "@/system/space";
import { DEFAULT_ORIENTATION, MIN_WINDOW_SIZE, useSplitView, type SplitLayout } from "@/utils/layout";
import { getViewBinding, getViewComponent } from "@/views";
import { type ViewExposed, viewEmits } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self: NodeReferenceData;
    size: Required<Pick<BoxData, "width" | "height">>;
  } & Pick<ViewData, "name" | "title" | "text" | "icon" | "orientation" | "focus">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(toRef(props, "self"));
const windows = spaceGraph.getChildrenRef(toRef(props, "self"), NodeType.VIEW);
const focusedWindowIdx: Ref<number | null> = computed(() => {
  if (windows.value.length == 0) return null;
  if (props.focus?.nodesPtr.length ?? 0 > 0) {
    const focusedId = props.focus!.nodesPtr[0].id;
    const focusedWindowIdx = windows.value.findIndex((window) => window.id == focusedId);
    return focusedWindowIdx >= 0 ? focusedWindowIdx : 0;
  } else {
    return 0;
  }
});
const orientation = computed(() => props.orientation ?? DEFAULT_ORIENTATION);
const isHorizontal = computed(() => orientation.value == Orientation.HORIZONTAL);

const BORDER_SIZE = 2;
const splitLayout: Ref<SplitLayout> = computed(() => ({
  orientation: orientation.value,
  minPx: MIN_WINDOW_SIZE,
  dividerSize: BORDER_SIZE,
}));
const containerRef: Ref<HTMLElement | null> = ref(null);
const { sizedViews, draggingIdx } = useSplitView(
  windows,
  toRef(props, "size"),
  containerRef,
  splitLayout,
  spaceConnection,
);

// actions
const actions: Partial<ActionMapImplementation<"view">> = {
  "view.navigate.closeWindow": {
    enabled: computed(() => focusedWindowIdx.value != null),
    action: () => {
      canvas.removeView(spaceConnection.sideTx, spaceGraph, windows.value[focusedWindowIdx.value!]);
    },
  },
  "view.navigate.focusPreviousWindow": {
    action: () => {
      const allWindows = canvas.currentWindows;
      const currentIdx = allWindows.findIndex((window) => window.id == windows.value[focusedWindowIdx.value!].id);
      const prevIdx = ((currentIdx ?? 0) - 1 + allWindows.length) % allWindows.length;
      canvas.focus(spaceConnection.sideTx, { view: allWindows[prevIdx] });
    },
  },
  "view.navigate.focusNextWindow": {
    action: () => {
      const allWindows = canvas.currentWindows;
      const currentIdx = allWindows.findIndex((window) => window.id == windows.value[focusedWindowIdx.value!].id);
      const nextIdx = ((currentIdx ?? 0) + 1) % canvas.currentWindows.length;
      canvas.focus(spaceConnection.sideTx, { view: allWindows[nextIdx] });
    },
  },
};

canvas.registerSelf(self);
defineExpose<ViewExposed>({ self, actions });
</script>
<template>
  <!-- Container -->
  <div
    ref="containerRef"
    class="relative bg-gray-100"
    :style="{ width: size.width + 'px', height: size.height + 'px' }"
    :class="[draggingIdx != null ? (isHorizontal ? 'cursor-ew-resize' : 'cursor-ns-resize') : '']"
  >
    <!-- Frames -->
    <template v-for="({ left, top, width, height, view }, viewIdx) in sizedViews" :key="view.id">
      <!-- Frame -->
      <div
        class="absolute border-gray-300 bg-gray-100"
        :style="{
          borderLeftWidth: viewIdx > 0 && isHorizontal ? BORDER_SIZE + 'px' : '0',
          borderTopWidth: viewIdx > 0 && !isHorizontal ? BORDER_SIZE + 'px' : '0',
          left: left + 'px',
          top: top + 'px',
          width: width + 'px',
          height: height + 'px',
        }"
        data-root-element="true"
      >
        <!-- Frame content -->
        <component
          v-if="getViewComponent(view.type) != null"
          :is="getViewComponent(view.type)"
          v-bind="
            getViewBinding(view, {
              width: isHorizontal && viewIdx > 0 ? width - BORDER_SIZE : width,
              height: !isHorizontal && viewIdx > 0 ? height - BORDER_SIZE : height,
            })
          "
        />
        <div v-else class="bg-red-100 text-center">{{ view.type }}</div>
      </div>
      <!-- Frame divider (draggable) -->
      <div
        v-if="viewIdx > 0"
        class="pointer-events-auto absolute transition-colors duration-300"
        :class="[
          isHorizontal ? 'w-1 cursor-ew-resize' : 'h-1 cursor-ns-resize',
          draggingIdx == viewIdx - 1 ? 'bg-primary-400' : 'bg-transparent hover:bg-primary-300',
        ]"
        :style="
          isHorizontal
            ? { left: left - 2 + 'px', top: top + 'px', height: height + 'px' }
            : { left: left + 'px', top: top - 2 + 'px', width: width + 'px' }
        "
        @mousedown="draggingIdx = viewIdx - 1"
        data-outside-view="true"
      />
    </template>
    <!-- No frames -->
    <div v-if="sizedViews.length === 0" class="h-full w-full">
      <!-- empty state -->
    </div>
  </div>
</template>
