<script lang="tsx" setup>
import { BoxData, NodeReferenceData, Orientation, ViewData } from "@/proto/wire/";
import { ScrollbarWidth, useScrollArea } from "@/utils/layout";
import { makeViewId } from "@/views";
import { type ViewExposed, viewEmits } from "@/views/common";
import { useMouseInElement } from "@vueuse/core";
import { computed, ref, toRef, watch } from "vue";

const props = defineProps<
  {
    self?: NodeReferenceData;
    trackWidth: ScrollbarWidth;
    trackIsOverlay?: boolean;
    trackIsAlwaysVisible?: boolean;
    size: Required<Pick<BoxData, "width" | "height">>;
    sizeIsDynamic?: boolean;
  } & Pick<ViewData, "orientation" | "variant">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const areaRef = ref<HTMLElement | null>(null);
const areaMouse = useMouseInElement(areaRef);
const isMouseInArea = computed(() => !areaMouse.isOutside.value);
const { thumb, isManualScrolling, isNativeScrolling, isOverflown } = useScrollArea({
  container: areaRef,
  orientation: toRef(props, "orientation"),
  trackWidth: toRef(props, "trackWidth"),
});

// show scrolling instantly, fade out once inactive
const isVisiblyScrolling = ref(false);
watch([isManualScrolling, isNativeScrolling], () => {
  if (isManualScrolling.value || isNativeScrolling.value) {
    isVisiblyScrolling.value = true;
  } else {
    setTimeout(() => {
      if (!isManualScrolling.value && !isNativeScrolling.value) {
        isVisiblyScrolling.value = false;
      }
    }, 1000);
  }
});

defineExpose<ViewExposed>({ self, id: makeViewId(props) });
</script>
<template>
  <div class="relative">
    <!-- Scroll area -->
    <div
      ref="areaRef"
      class="scrollbar-none relative"
      :class="[orientation == Orientation.HORIZONTAL ? 'overflow-x-scroll' : 'overflow-y-scroll', $attrs.class]"
      :style="{
        [sizeIsDynamic ? 'maxWidth' : 'width']:
          (orientation == Orientation.HORIZONTAL || trackIsOverlay ? size.width : size.width - trackWidth) + 'px',
        [sizeIsDynamic ? 'maxHeight' : 'height']:
          (orientation == Orientation.VERTICAL || trackIsOverlay ? size.height : size.height - trackWidth) + 'px',
      }"
    >
      <slot />
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
        class="absolute z-40 rounded-md transition-colors duration-300"
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
        @mousedown="isManualScrolling = true"
      />
    </div>
  </div>
</template>
