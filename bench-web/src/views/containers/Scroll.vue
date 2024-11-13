<script lang="ts" setup>
import { RectangleData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { ScrollbarWidth, useScrollArea } from "@/ui/layout";
import { viewEmits, type ViewExposed } from "@/views/common";
import { useMouseInElement } from "@vueuse/core";
import { computed, ref, toRef, watch, watchEffect, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    trackWidth: ScrollbarWidth;
    trackIsOverlay?: boolean;
    trackIsAlwaysVisible?: boolean;
    size: Pick<RectangleData, "width" | "height">;
    sizeIsDynamic?: boolean;
    stickToEnd?: boolean;
  } & Pick<ViewData, "orientation" | "variant">
>();
const emit = defineEmits({ ...viewEmits(), scroll: null });
const self = toRef(props, "self");
const id = toRef(props, "id");

const containerRef = ref<HTMLElement | null>(null);
const innerRef = ref<HTMLElement | null>(null);
const areaMouse = useMouseInElement(containerRef);
const isMouseInArea = computed(() => !areaMouse.isOutside.value);
const horizontalScrollArea = useScrollArea({
  container: containerRef,
  inner: innerRef,
  orientation: Orientation.HORIZONTAL,
  trackWidth: toRef(props, "trackWidth"),
});
const verticalScrollArea = useScrollArea({
  container: containerRef,
  inner: innerRef,
  orientation: Orientation.VERTICAL,
  trackWidth: toRef(props, "trackWidth"),
});

function scrollToEnd() {
  if (props.orientation == null || props.orientation == Orientation.HORIZONTAL) {
    horizontalScrollArea.scroll.x.value = horizontalScrollArea.innerSize.width.value;
  }
  if (props.orientation == null || props.orientation == Orientation.VERTICAL) {
    verticalScrollArea.scroll.y.value = verticalScrollArea.innerSize.height.value;
  }
}

const sizeStyles = computed(() => {
  if (props.orientation == Orientation.HORIZONTAL) {
    const width = props.trackIsOverlay ? props.size.width : (props.size.width ?? 0) - props.trackWidth;
    return {
      [props.sizeIsDynamic ? "maxWidth" : "width"]: width + "px",
    };
  } else {
    const height = props.trackIsOverlay ? props.size.height : (props.size.height ?? 0) - props.trackWidth;
    return {
      [props.sizeIsDynamic ? "maxHeight" : "height"]: height + "px",
    };
  }
});

// auto-scroll to end if sticky
watch(
  () => [props.stickToEnd, horizontalScrollArea.innerSize.width.value, verticalScrollArea.innerSize.height.value],
  () => {
    if (props.stickToEnd) scrollToEnd();
  },
  { immediate: true },
);

// show scrolling instantly, fade out once inactive
const isSomeScrolling = computed(
  () =>
    horizontalScrollArea.isThumbScrolling.value ||
    horizontalScrollArea.isNativeScrolling.value ||
    verticalScrollArea.isThumbScrolling.value ||
    verticalScrollArea.isNativeScrolling.value,
);
const showScrolling = ref(false);
watch([isSomeScrolling], () => {
  if (isSomeScrolling.value) {
    showScrolling.value = true;
  } else {
    setTimeout(() => {
      if (!isSomeScrolling.value) {
        showScrolling.value = false;
      }
    }, 1000);
  }
});

canvas.registerView(self, id)
defineExpose<ViewExposed & { isScrolling: Ref<boolean>; isAtEnd: Ref<boolean>; scrollToEnd: () => void }>({
  self,
  id,
  isScrolling: isSomeScrolling,
  isAtEnd: computed(() => horizontalScrollArea.isAtEnd.value && verticalScrollArea.isAtEnd.value),
  scrollToEnd,
});
</script>
<template>
  <div class="relative">
    <!-- TODO :UX: Scroll view captures scroll in both directions, not just its own orientation -->
    <!-- (so if we have a horizontal Scroll, it will prevent vertical scrolling, sometimes annoying) -->
    <!-- Scroll area -->
    <div
      ref="containerRef"
      class="scrollbar-none relative"
      :class="[
        orientation == Orientation.HORIZONTAL ? 'touch-pan-x overflow-x-scroll' : '',
        orientation == Orientation.VERTICAL ? 'touch-pan-y overflow-y-scroll' : '',
        orientation == null ? 'touch-pan-xy overflow-scroll' : '',
        $attrs.class,
      ]"
      :style="{ ...sizeStyles }"
    >
      <!-- Inner wrapper -->
      <div ref="innerRef" class="" :class="$attrs.class">
        <slot />
      </div>
    </div>
    <!-- Scroll track  -->
    <div
      v-for="{ orientation, area } in props.orientation != null
        ? [
            {
              orientation: props.orientation,
              area: props.orientation == Orientation.HORIZONTAL ? horizontalScrollArea : verticalScrollArea,
            },
          ]
        : [
            { orientation: Orientation.HORIZONTAL, area: horizontalScrollArea },
            { orientation: Orientation.VERTICAL, area: verticalScrollArea },
          ]"
      :class="[
        'group absolute z-30',
        orientation == Orientation.HORIZONTAL ? 'bottom-0 left-0 w-full' : 'right-0 top-0 h-full',
      ]"
      :style="
        orientation == Orientation.HORIZONTAL ? { height: props.trackWidth + 'px' } : { width: props.trackWidth + 'px' }
      "
    >
      <div
        v-if="area.isOverflown.value"
        class="absolute z-40 rounded transition-colors duration-300"
        :class="[
          'hover:opacity-100 group-hover:opacity-80',
          showScrolling ? 'bg-gray-400 opacity-100' : '',
          !showScrolling && trackIsAlwaysVisible ? 'bg-gray-300 opacity-100' : '',
          !showScrolling && !trackIsAlwaysVisible
            ? isMouseInArea
              ? 'bg-gray-300 opacity-80'
              : 'bg-gray-300 opacity-0'
            : '',
        ]"
        :style="{
          left: area.thumb.value.left + 'px',
          top: area.thumb.value.top + 'px',
          width: area.thumb.value.width + 'px',
          height: area.thumb.value.height + 'px',
        }"
        @mousedown="area.isThumbScrolling.value = true"
      />
    </div>
    <!-- Scroll track (horizontal) -->
  </div>
</template>
