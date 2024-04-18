<script lang="ts" setup>
import { getFloatingPosition, type FloatingOptions } from "@/utils/floating";
import { Shortcut, activeTooltips, type TooltipInstance } from "@/utils/tooltip";
import { ref, type Ref } from "vue";

const tooltipRefs: Ref<Record<string, HTMLDivElement>> = ref({});

function positionTooltip(tooltip: TooltipInstance, el: HTMLDivElement) {
  // get bounding
  const tooltipRect = el.getBoundingClientRect();
  const referenceRect = tooltip.reference.getBoundingClientRect();
  const options: FloatingOptions = { placement: "top", referenceMargin: 4, containerMargin: 12, ...tooltip.info };
  const containerRect =
    tooltip.container != null
      ? tooltip.container.getBoundingClientRect()
      : { x: 0, y: 0, width: window.innerWidth, height: window.innerHeight };

  // position
  const { x, y } = getFloatingPosition({
    floating: { width: tooltipRect.width, height: tooltipRect.height },
    reference: referenceRect,
    container: containerRect,
    options,
  });
  el.style.position = "fixed";
  el.style.left = x + "px";
  el.style.top = y + "px";
}
</script>
<template>
  <TransitionGroup
    enter-active-class="transition-all ease-in duration-75"
    enter-from-class="opacity-0 scale-95"
    enter-to-class="opacity-100 scale-100"
    leave-active-class="transition-all ease-out duration-75"
    leave-from-class="opacity-100 scale-100"
    leave-to-class="opacity-0 scale-95"
  >
    <template v-for="tooltip in activeTooltips" :key="tooltip.id">
      <div
        :ref="(ref?: any) => (ref != null ? (tooltipRefs[tooltip.id] = ref, positionTooltip(tooltip, ref)) : (delete tooltipRefs[tooltip.id]))"
        class="absolute z-70 w-fit max-w-80 whitespace-nowrap rounded-md border border-gray-300 bg-white text-gray-700 shadow-sm shadow-gray-300"
        :class="[tooltip.info.small ? 'px-1.5 py-0.5' : 'px-2.5 py-1']"
        @mouseenter="tooltip.reference.tooltipOnMouseEnter"
        @mouseleave="tooltip.reference.tooltipOnMouseLeave"
      >
        <!-- Header -->
        <p v-if="tooltip.info.icon || tooltip.info.title" class="mb-0.5 flex flex-row items-center">
          <i v-if="tooltip.info.icon" class="mr-1.5 text-gray-600" :class="tooltip.info.icon" />
          <span v-if="tooltip.info.title" class="truncate font-semibold">
            {{ typeof tooltip.info.title == "function" ? tooltip.info.title() : tooltip.info.title }}
          </span>
          <span v-if="tooltip.info.shortcuts" class="ml-auto pl-4">
            <Shortcut class="text-gray-700" :shortcut="tooltip.info.shortcuts[0]" />
          </span>
        </p>
        <!-- Content -->
        <p class="max-h-20 max-w-full truncate whitespace-break-spaces">
          {{ typeof tooltip.info.text == "function" ? tooltip.info.text() : tooltip.info.text }}
        </p>
      </div>
    </template>
  </TransitionGroup>
</template>
