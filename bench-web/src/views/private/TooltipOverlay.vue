<script lang="tsx" setup>
import { getFloatingPosition, type FloatingOptions } from "@/utils/floating";
import { Shortcut, activeTooltips, type TooltipInstance } from "@/utils/tooltip";
import { ref, type Ref } from "vue";

const tooltipRefs: Ref<Record<string, HTMLDivElement>> = ref({});

function positionTooltip(tooltip: TooltipInstance, el: HTMLDivElement) {
  const tooltipRect = el.getBoundingClientRect();
  const referenceRect = tooltip.reference.getBoundingClientRect();
  const options: FloatingOptions = { placement: "top", referenceMargin: 4, containerMargin: 12, ...tooltip.info };
  const containerRect =
    tooltip.container != null
      ? tooltip.container.getBoundingClientRect()
      : { x: 0, y: 0, width: window.innerWidth, height: window.innerHeight };
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
    <div
      :ref="(ref?: any) => (ref != null ? (tooltipRefs[tooltip.id] = ref, positionTooltip(tooltip, ref)) : (delete tooltipRefs[tooltip.id]))"
      v-for="tooltip in activeTooltips"
      v-bind="tooltip.info"
      :key="tooltip.id"
      class="z-70 w-fit max-w-72 whitespace-nowrap rounded-md border border-gray-300 bg-white px-2.5 py-1 text-gray-700 shadow-md shadow-gray-300"
    >
      <!-- Header -->
      <p v-if="tooltip.info.icon || tooltip.info.title" class="mb-0.5 flex flex-row items-center">
        <i v-if="tooltip.info.icon" class="mr-1.5 text-gray-600" :class="tooltip.info.icon" />
        <span v-if="tooltip.info.title" class="font-semibold">{{ tooltip.info.title }}</span>
        <span v-if="tooltip.info.shortcuts" class="ml-auto pl-4">
          <Shortcut :shortcut="tooltip.info.shortcuts[0]" />
        </span>
      </p>
      <!-- Content -->
      <p>{{ tooltip.info.text }}</p>
    </div>
  </TransitionGroup>
</template>
