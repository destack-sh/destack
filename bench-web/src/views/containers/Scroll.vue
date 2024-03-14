<script lang="tsx" setup>
import { ViewData, NodeReferenceData, Orientation, BoxData } from "@/proto/wire/";
import { useScrollArea, ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import { useMousePressed } from "@vueuse/core";
import { ref, toRef, watch } from "vue";

const props = defineProps<
  {
    self?: NodeReferenceData | undefined;
    trackWidth: ScrollbarWidth;
    size: Required<Pick<BoxData, "width" | "height">>;
  } & Pick<ViewData, "orientation">
>();
const emit = defineEmits(viewEmits());

const areaRef = ref<HTMLElement | null>(null);
const isScrolling = ref(false);
const { thumb, isScrolling: isNativeScrolling, isOverflown } = useScrollArea({
  container: areaRef,
  orientation: toRef(props, "orientation"),
  trackWidth: toRef(props, "trackWidth"),
});
const { pressed } = useMousePressed();
watch(pressed, () => {
  if (!pressed.value) {
    isScrolling.value = false;
  }
});

const self = toRef(props, "self");
defineExpose({ self });
</script>
<template>
  <div class="relative">
    <!-- Scroll area -->
    <div
      ref="areaRef"
      class="scrollbar-none relative"
      :class="[orientation == Orientation.HORIZONTAL ? 'overflow-x-auto' : 'overflow-y-auto', $attrs.class]"
      :style="{
        width: (orientation == Orientation.HORIZONTAL ? size.width : size.width - trackWidth) + 'px',
        height: (orientation == Orientation.HORIZONTAL ? size.height - trackWidth : size.height) + 'px',
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
        class="absolute z-40 transition-colors duration-300"
        :class="[
          isScrolling || isNativeScrolling
            ? 'bg-primary-400 opacity-100'
            : 'bg-primary-300 opacity-0 group-hover:opacity-80',
        ]"
        :style="{
          left: thumb.left + 'px',
          top: thumb.top + 'px',
          width: thumb.width + 'px',
          height: thumb.height + 'px',
        }"
        @mousedown="isScrolling = true"
      />
    </div>
  </div>
</template>
