<script lang="ts" setup>
import { IconInline, toIconMaybe } from "@/ui/icon";
import { Shortcut, activeTooltips, type TooltipInstance } from "@/ui/tooltip";
import { getFloatingPosition, type FloatingOptions } from "@/utils/floating";
import { ref, type Ref } from "vue";

const tooltipRefs: Ref<Record<string, HTMLDivElement>> = ref({});

function positionTooltip(tooltip: TooltipInstance, el: HTMLDivElement) {
  // get bounding
  const tooltipRect = el.getBoundingClientRect();
  const referenceRect = tooltip.reference.getBoundingClientRect();
  const options: FloatingOptions = { placement: "top", referenceMargin: 4, containerMargin: 12, ...tooltip };
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
      <!-- Tooltip -->
      <div
        :ref="
          (ref?: any) =>
            ref != null
              ? ((tooltipRefs[tooltip.id] = ref), positionTooltip(tooltip, ref))
              : delete tooltipRefs[tooltip.id]
        "
        class="absolute z-100 flex w-fit max-w-80 whitespace-nowrap rounded-sm border border-gray-200 bg-white text-gray-700"
        :class="[tooltip.small ? 'flex-row px-1.5 gap-x-1.5 py-0.5' : 'flex-col px-2.5 py-1']"
        @mouseenter="tooltip.reference.tooltipOnMouseEnter"
        @mouseleave="tooltip.reference.tooltipOnMouseLeave"
      >
        <!-- Header -->
        <p v-if="tooltip.icon || tooltip.title" class="mb-0.5 flex flex-row items-center gap-x-1.5">
          <IconInline v-if="tooltip.icon" class="w-5 text-gray-700" v-bind="toIconMaybe(tooltip.icon)" />
          <span v-if="tooltip.title" class="truncate" :class="tooltip.small ? 'font-medium' : 'font-semibold'">
            {{ typeof tooltip.title == "function" ? tooltip.title() : tooltip.title }}
          </span>
          <Shortcut v-for="shortcut in tooltip.shortcuts" class="text-gray-700" :shortcut="shortcut" />
        </p>
        <!-- Content -->
        <p class="max-h-20 max-w-full truncate whitespace-break-spaces">
          {{ typeof tooltip.text == "function" ? tooltip.text() : tooltip.text }}
        </p>
      </div>
    </template>
  </TransitionGroup>
</template>
