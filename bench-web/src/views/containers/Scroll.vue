<script lang="ts" setup>
import { BoxData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { ScrollbarWidth, useScrollArea } from "@/utils/layout";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { useMouseInElement } from "@vueuse/core";
import { computed, ref, toRef, watch, watchEffect, type Ref } from "vue";

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    trackWidth: ScrollbarWidth;
    trackIsOverlay?: boolean;
    trackIsAlwaysVisible?: boolean;
    size: Required<Pick<BoxData, "width" | "height">>;
    sizeIsDynamic?: boolean;
    stickToEnd?: boolean;
  } & Pick<ViewData, "orientation" | "variant">
>();
const emit = defineEmits({ ...viewEmits(), scroll: null });
const self = toRef(props, "self");

const containerRef = ref<HTMLElement | null>(null);
const innerRef = ref<HTMLElement | null>(null);
const areaMouse = useMouseInElement(containerRef);
const isMouseInArea = computed(() => !areaMouse.isOutside.value);
const { thumb, isThumbScrolling, isNativeScrolling, isOverflown, scroll, innerSize, isAtEnd } = useScrollArea({
  container: containerRef,
  inner: innerRef,
  orientation: toRef(props, "orientation"),
  trackWidth: toRef(props, "trackWidth"),
});

function scrollToEnd() {
  if (props.orientation == Orientation.HORIZONTAL) {
    scroll.x.value = innerSize.width.value;
  } else {
    scroll.y.value = innerSize.height.value;
  }
}

// auto-scroll to end if sticky
watch(
  () => [props.stickToEnd, innerSize.width.value, innerSize.height.value],
  () => {
    if (props.stickToEnd) scrollToEnd();
  },
  { immediate: true },
);

// show scrolling instantly, fade out once inactive
const isVisiblyScrolling = ref(false);
watch([isThumbScrolling, isNativeScrolling], () => {
  if (isThumbScrolling.value || isNativeScrolling.value) {
    isVisiblyScrolling.value = true;
  } else {
    setTimeout(() => {
      if (!isThumbScrolling.value && !isNativeScrolling.value) {
        isVisiblyScrolling.value = false;
      }
    }, 1000);
  }
});

const id = makeViewId(props);
canvas.registerView(self, id);
defineExpose<ViewExposed & { isScrolling: Ref<boolean>; isAtEnd: Ref<boolean>; scrollToEnd: () => void }>({
  self,
  id,
  isScrolling: computed(() => isThumbScrolling.value || isNativeScrolling.value),
  isAtEnd,
  scrollToEnd,
});
</script>
<template>
  <div class="relative">
    <!-- Scroll area -->
    <div
      ref="containerRef"
      class="scrollbar-none relative"
      :class="[orientation == Orientation.HORIZONTAL ? 'overflow-x-scroll' : 'overflow-y-scroll', $attrs.class]"
      :style="{
        [sizeIsDynamic ? 'maxWidth' : 'width']:
          (orientation == Orientation.HORIZONTAL || trackIsOverlay ? size.width : size.width - trackWidth) + 'px',
        [sizeIsDynamic ? 'maxHeight' : 'height']:
          (orientation == Orientation.VERTICAL || trackIsOverlay ? size.height : size.height - trackWidth) + 'px',
      }"
      @scroll="(e) => $emit('scroll', e)"
    >
      <!-- Inner wrapper -->
      <div ref="innerRef" class="w-full h-full" :class="$attrs.class">
        <slot />
      </div>
    </div>
    <!-- Scroll track  -->
    <div
      :class="[
        'group absolute z-30',
        props.orientation == Orientation.HORIZONTAL ? 'bottom-0 left-0 w-full' : 'right-0 top-0 h-full',
      ]"
      :style="
        props.orientation == Orientation.HORIZONTAL
          ? { height: props.trackWidth + 'px' }
          : { width: props.trackWidth + 'px' }
      "
    >
      <!-- Scroll thumb -->
      <div
        v-if="isOverflown"
        class="absolute z-40 rounded transition-colors duration-300"
        :class="[
          'hover:opacity-100 group-hover:opacity-80',
          trackIsAlwaysVisible || isVisiblyScrolling
            ? 'bg-primary-400 opacity-100'
            : isMouseInArea
              ? 'bg-primary-300 opacity-80'
              : 'bg-primary-300 opacity-0',
        ]"
        :style="{
          left: thumb.left + 'px',
          top: thumb.top + 'px',
          width: thumb.width + 'px',
          height: thumb.height + 'px',
        }"
        @mousedown="isThumbScrolling = true"
      />
    </div>
  </div>
</template>
