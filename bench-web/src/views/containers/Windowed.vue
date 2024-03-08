<script lang="tsx" setup>
import { BoxData, NodeType, Orientation } from "@/proto/wire";
import { getChildrenRef } from "@/system/graph";
import { DEFAULT_ORIENTATION, MIN_WINDOW_SIZE, splitView } from "@/utils/positioning";
import { type ViewEmits, type ViewPropsAllAnchored } from "@/views/common";
import { useMouseInElement, useMousePressed } from "@vueuse/core";
import { computed, ref, toRef, watch } from "vue";

const props = defineProps<
  Pick<ViewPropsAllAnchored, "self" | "name" | "title" | "text" | "icon" | "orientation"> & {
    size: Required<Pick<BoxData, "width" | "height">>;
  }
>();
const emits = defineEmits<ViewEmits>();

const { children: windows } = getChildrenRef(toRef(props, "self"), NodeType.VIEW);

// positioning
const orientation = computed(() => props.orientation ?? DEFAULT_ORIENTATION);
const splitLayout = computed(() => ({
  orientation: props.orientation ?? Orientation.HORIZONTAL,
  defaultRelativeUnits: 1,
  minPx: MIN_WINDOW_SIZE,
}));
const { sizedViews, updateSeparator } = splitView(windows, toRef(props, "size"), splitLayout);

// dragging
const containerRef = ref<HTMLElement | null>(null);
const { pressed } = useMousePressed();
const { elementX: mouseRelativeX, elementY: mouseRelativeY } = useMouseInElement(containerRef);
const draggingSepIdx = ref<number | null>(null);
watch([pressed, mouseRelativeX, mouseRelativeY], () => {
  if (draggingSepIdx.value == null) return;
  if (!pressed.value) {
    draggingSepIdx.value = null;
    return;
  }
  const draggedToPx = orientation.value == Orientation.HORIZONTAL ? mouseRelativeX.value : mouseRelativeY.value;
  const [aUpdate, bUpdate] = updateSeparator(draggingSepIdx.value, draggedToPx);

  // nocheckin: apply through tx
  windows.value[draggingSepIdx.value].size = aUpdate.size;
  windows.value[draggingSepIdx.value + 1].size = bUpdate.size;
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
      draggingSepIdx != null ? (orientation == Orientation.HORIZONTAL ? 'cursor-ew-resize' : 'cursor-ns-resize') : '',
    ]"
  >
    <!-- Windowed -->
    <template v-for="({ left, top, width, height, view }, viewIdx) in sizedViews" :key="view.id">
      <!-- Window -->
      <div
        class="absolute border-gray-300 bg-gray-100"
        :class="[
          orientation == Orientation.HORIZONTAL
            ? viewIdx == 0
              ? 'border-x-2'
              : 'border-r-2'
            : viewIdx == 0
              ? 'border-y-2'
              : 'border-b-2',
        ]"
        :style="{ left: left + 'px', top: top + 'px', width: width + 'px', height: height + 'px' }"
      >
        <!-- Content -->
        <!-- <div class="flex w-fit flex-col bg-red-700 font-mono text-xs text-white">
          <div>{{ ViewType[view.type] }} left:{{ left }} top:{{ top }} width:{{ width }} height:{{ height }}</div>
        </div> -->
      </div>
      <!-- Draggable separator -->
      <div
        v-if="viewIdx > 0"
        class="absolute transition-colors duration-500"
        :class="[
          orientation == Orientation.HORIZONTAL ? 'w-1 cursor-ew-resize' : 'h-1 cursor-ns-resize',
          draggingSepIdx == viewIdx - 1 ? 'bg-primary-400' : 'bg-transparent hover:bg-primary-300',
        ]"
        :style="
          orientation == Orientation.HORIZONTAL
            ? { left: left - 2 + 'px', top: top + 'px', height: height + 'px' }
            : { left: left + 'px', top: top - 2 + 'px', width: width + 'px' }
        "
        @mousedown="draggingSepIdx = viewIdx - 1"
      />
    </template>
  </div>
</template>
