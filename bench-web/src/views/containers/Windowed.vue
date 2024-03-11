<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire";
import { useLoadedGraph } from "@/system/connection";
import { DEFAULT_ORIENTATION, MIN_WINDOW_SIZE, splitView, type SplitLayout } from "@/utils/positioning";
import { getViewBinding, getViewComponent } from "@/views";
import { viewEmits } from "@/views/common";
import { useMouseInElement, useMousePressed } from "@vueuse/core";
import { computed, ref, toRef, watch, type Ref } from "vue";

const props = defineProps<
  {
    self: NodeReferenceData;
    size: Required<Pick<BoxData, "width" | "height">>;
  } & Pick<ViewData, "name" | "title" | "text" | "icon" | "orientation">
>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(toRef(props, "self"));
const windows = spaceGraph.getChildrenRef(toRef(props, "self"), NodeType.VIEW);

// positioning
const orientation = computed(() => props.orientation ?? DEFAULT_ORIENTATION);
const splitLayout: Ref<SplitLayout> = computed(() => ({
  orientation: props.orientation ?? Orientation.HORIZONTAL,
  defaultRelativeUnits: 1,
  minPx: MIN_WINDOW_SIZE,
  dividerSize: 2,
}));
const { sizedViews, updateSeparator } = splitView(windows, toRef(props, "size"), splitLayout);

// dragging
// could probably reuse Windowed component for Split view?
const containerRef = ref<HTMLElement | null>(null);
const { pressed } = useMousePressed();
const { elementX: mouseRelativeX, elementY: mouseRelativeY } = useMouseInElement(containerRef);
const draggingIdx = ref<number | null>(null);
watch([pressed, mouseRelativeX, mouseRelativeY], () => {
  if (draggingIdx.value == null) return;
  if (!pressed.value) {
    draggingIdx.value = null;
    return;
  }
  const draggedToPx = orientation.value == Orientation.HORIZONTAL ? mouseRelativeX.value : mouseRelativeY.value;
  const [aUpdate, bUpdate] = updateSeparator(draggingIdx.value, draggedToPx);
  spaceConnection.sideTx.update({
    metatype: NodeType.VIEW,
    id: windows.value[draggingIdx.value].id,
    size: aUpdate.size,
    debounce: true,
  });
  spaceConnection.sideTx.update({
    metatype: NodeType.VIEW,
    id: windows.value[draggingIdx.value + 1].id,
    size: bUpdate.size,
    debounce: true,
  });
});

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <!-- Container -->
  <div
    ref="containerRef"
    class="relative bg-gray-100"
    :style="{ width: size.width + 'px', height: size.height + 'px' }"
    :class="[
      draggingIdx != null ? (orientation == Orientation.HORIZONTAL ? 'cursor-ew-resize' : 'cursor-ns-resize') : '',
      draggingIdx != null ? 'pointer-events-none select-none' : 'pointer-events-auto select-auto',
    ]"
  >
    <!-- Windowed -->
    <template v-for="({ left, top, width, height, view }, viewIdx) in sizedViews" :key="view.id">
      <!-- Window -->
      <div
        class="absolute border-gray-400"
        :class="[
          viewIdx > 0 ? (orientation == Orientation.HORIZONTAL ? 'border-l' : 'border-t') : '',
        ]"
        :style="{ left: left + 'px', top: top + 'px', width: width + 'px', height: height + 'px' }"
      >
        <!-- Content -->
        <component
          v-if="getViewComponent(view.type) != null"
          :is="getViewComponent(view.type)"
          v-bind="getViewBinding(view, { width, height })"
        />
        <div v-else class="bg-red-100 text-center">{{ view.type }}</div>
      </div>
      <!-- Draggable divider -->
      <div
        v-if="viewIdx > 0"
        class="pointer-events-auto absolute transition-colors duration-500"
        :class="[
          orientation == Orientation.HORIZONTAL ? 'w-1 cursor-ew-resize' : 'h-1 cursor-ns-resize',
          draggingIdx == viewIdx - 1 ? 'bg-primary-400' : 'bg-transparent hover:bg-primary-300',
        ]"
        :style="
          orientation == Orientation.HORIZONTAL
            ? { left: left - 2 + 'px', top: top + 'px', height: height + 'px' }
            : { left: left + 'px', top: top - 2 + 'px', width: width + 'px' }
        "
        @mousedown="draggingIdx = viewIdx - 1"
      />
    </template>
  </div>
</template>
