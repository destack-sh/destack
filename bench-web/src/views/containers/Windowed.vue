<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire";
import { useLoadedGraph } from "@/system/connection";
import { DEFAULT_ORIENTATION, MIN_WINDOW_SIZE, useSplitView, type SplitLayout } from "@/utils/positioning";
import { getViewBinding, getViewComponent } from "@/views";
import { viewEmits } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  {
    self: NodeReferenceData;
    size: Required<Pick<BoxData, "width" | "height">>;
  } & Pick<ViewData, "name" | "title" | "text" | "icon" | "orientation">
>();
const emit = defineEmits(viewEmits());

const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(toRef(props, "self"));
const windows = spaceGraph.getChildrenRef(toRef(props, "self"), NodeType.VIEW);
const orientation = computed(() => props.orientation ?? DEFAULT_ORIENTATION);
const splitLayout: Ref<SplitLayout> = computed(() => ({
  orientation: props.orientation ?? Orientation.HORIZONTAL,
  defaultRelativeUnits: 1,
  minPx: MIN_WINDOW_SIZE,
  dividerSize: 2,
}));
const containerRef: Ref<HTMLElement | null> = ref(null);
const { sizedViews, draggingIdx } = useSplitView(
  windows,
  toRef(props, "size"),
  containerRef,
  splitLayout,
  spaceConnection,
);

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
    <!-- Frames -->
    <template v-for="({ left, top, width, height, view }, viewIdx) in sizedViews" :key="view.id">
      <!-- Frame -->
      <div
        class="absolute border-gray-300 bg-gray-100"
        :class="[viewIdx > 0 ? (orientation == Orientation.HORIZONTAL ? 'border-l-2' : 'border-t-2') : '']"
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
    <!-- No frames -->
    <div v-if="sizedViews.length === 0" class="h-full w-full">
      <!-- empty state -->
    </div>
  </div>
</template>
