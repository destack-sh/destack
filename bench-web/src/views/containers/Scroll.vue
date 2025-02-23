<script lang="ts" setup>
import { NodeType, Orientation, RectangleData, ViewData } from "@/proto/wire/";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { ScrollbarWidth, useScrollArea } from "@/ui/layout";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { computed, ref, toRef, watch, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    trackWidth?: ScrollbarWidth;
    trackIsOverlay?: boolean;
    trackIsAlwaysVisible?: boolean;
    size: Pick<RectangleData, "width" | "height">;
    sizeIsDynamic?: boolean;
    stickToEnd?: boolean;
  } & Pick<ViewData, "orientation">
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

const containerRef = ref<HTMLElement | null>(null);
const innerRef = ref<HTMLElement | null>(null);
const trackWidth = computed(() => props.trackWidth ?? ScrollbarWidth.sm);
const horizontalScrollArea = useScrollArea({
  container: containerRef,
  inner: innerRef,
  orientation: Orientation.HORIZONTAL,
  trackWidth,
});
const verticalScrollArea = useScrollArea({
  container: containerRef,
  inner: innerRef,
  orientation: Orientation.VERTICAL,
  trackWidth,
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
    const width = props.trackIsOverlay ? props.size.width : (props.size.width ?? 0) - trackWidth.value;
    return {
      [props.sizeIsDynamic ? "maxWidth" : "width"]: width + "px",
    };
  } else if (props.orientation == Orientation.VERTICAL) {
    const height = props.trackIsOverlay ? props.size.height : (props.size.height ?? 0) - trackWidth.value;
    return {
      [props.sizeIsDynamic ? "maxHeight" : "height"]: height + "px",
    };
  } else {
    return {};
  }
});

// auto-scroll to end if sticky
watch(
  () => [props.stickToEnd, horizontalScrollArea.innerSize.width.value, verticalScrollArea.innerSize.height.value],
  () => {
    if (props.stickToEnd) {
      scrollToEnd();
    }
  },
  { immediate: true },
);

// show scrolling instantly, fade out once inactive
const isSomeScrolling = computed(() => {
  if (
    (props.orientation == null || props.orientation == Orientation.HORIZONTAL) &&
    (horizontalScrollArea.isThumbScrolling.value || horizontalScrollArea.isNativeScrolling.value)
  ) {
    return true;
  } else if (
    (props.orientation == null || props.orientation == Orientation.VERTICAL) &&
    (verticalScrollArea.isThumbScrolling.value || verticalScrollArea.isNativeScrolling.value)
  ) {
    return true;
  } else {
    return false;
  }
});
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

canvas.registerView(self, id);
defineExpose<
  ViewExpose & {
    isScrolling: Ref<boolean>;
    isAtStart: Ref<boolean>;
    isCloseToStart: Ref<boolean>;
    isAtEnd: Ref<boolean>;
    isCloseToEnd: Ref<boolean>;
    isHorizontalOverflown: Ref<boolean>;
    isVerticalOverflown: Ref<boolean>;
    isOverflown: Ref<boolean>;
    scrollToEnd: () => void;
  }
>({
  self,
  id,
  isScrolling: isSomeScrolling,
  isAtStart: computed(
    () =>
      ((props.orientation == null || props.orientation == Orientation.HORIZONTAL) &&
        horizontalScrollArea.isAtStart.value) ||
      ((props.orientation == null || props.orientation == Orientation.VERTICAL) && verticalScrollArea.isAtStart.value),
  ),
  isCloseToStart: computed(
    () =>
      ((props.orientation == null || props.orientation == Orientation.HORIZONTAL) &&
        horizontalScrollArea.isCloseToStart.value) ||
      ((props.orientation == null || props.orientation == Orientation.VERTICAL) &&
        verticalScrollArea.isCloseToStart.value),
  ),
  isAtEnd: computed(
    () =>
      ((props.orientation == null || props.orientation == Orientation.HORIZONTAL) &&
        horizontalScrollArea.isAtEnd.value) ||
      ((props.orientation == null || props.orientation == Orientation.VERTICAL) && verticalScrollArea.isAtEnd.value),
  ),
  isCloseToEnd: computed(
    () =>
      ((props.orientation == null || props.orientation == Orientation.HORIZONTAL) &&
        horizontalScrollArea.isCloseToEnd.value) ||
      ((props.orientation == null || props.orientation == Orientation.VERTICAL) &&
        verticalScrollArea.isCloseToEnd.value),
  ),
  isHorizontalOverflown: horizontalScrollArea.isOverflown,
  isVerticalOverflown: verticalScrollArea.isOverflown,
  isOverflown: computed(() => horizontalScrollArea.isOverflown.value || verticalScrollArea.isOverflown.value),
  scrollToEnd,
});
</script>
<template>
  <div class="group/scroll relative focus:outline-none">
    <!-- Scroll area -->
    <div
      ref="containerRef"
      class="scrollbar-none relative overscroll-auto focus:outline-none"
      :class="[
        orientation == Orientation.HORIZONTAL ? 'overflow-y-hidden overflow-x-scroll' : '',
        orientation == Orientation.VERTICAL ? 'overflow-x-hidden overflow-y-scroll' : '',
        orientation == null ? 'overflow-scroll' : '',
        $attrs.class,
      ]"
      :style="{ ...sizeStyles }"
      :tabindex="-1"
    >
      <!-- Inner wrapper -->
      <div ref="innerRef" class="min-h-fit min-w-fit" :class="$attrs.class">
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
        'group/track absolute z-30',
        orientation == Orientation.HORIZONTAL ? 'bottom-0 left-0 w-full' : 'right-0 top-0 h-full',
      ]"
      :style="
        orientation == Orientation.HORIZONTAL ? { height: props.trackWidth + 'px' } : { width: props.trackWidth + 'px' }
      "
      data-suppress-drag="both"
    >
      <div
        v-if="area.isOverflown.value"
        class="absolute z-40 rounded transition-colors duration-150"
        :class="[
          'hover:opacity-100 group-hover:opacity-80',
          trackIsAlwaysVisible
            ? 'bg-gray-300 opacity-100'
            : 'bg-gray-300 hover:opacity-100 group-hover/track:opacity-80',
          !trackIsAlwaysVisible && !showScrolling ? 'opacity-0' : '',
          !trackIsAlwaysVisible && showScrolling ? 'opacity-100' : '',
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
  </div>
</template>
