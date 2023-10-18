<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { useViewSectionGroup } from "@/components/views/sections";
import { useActiveScroll } from "@/composables/useScroll";
import { useAppearance } from "@/state/appearance";
import type { Action } from "@/state/bench";
import { toValueRef } from "@/utils/functools";
import { useElementBounding } from "@vueuse/core";
import { computed, ref } from "vue";
const props = defineProps<{
  index: number;
  title: string;
  loading?: boolean;
  actions?: Action<unknown>[];
  minHeight?: number;
}>();

const appearance = useAppearance();
const api = useViewSectionGroup();
const sectionRef = ref<HTMLDivElement | undefined>(undefined);
const sectionContainerRef = ref<HTMLDivElement | undefined>(undefined);
const sectionBodyRef = ref<HTMLDivElement | undefined>(undefined);
const sectionSlotRef = ref<InstanceType<any> | undefined>(undefined);
const sectionBodyBounding = useElementBounding(sectionBodyRef);

useActiveScroll(sectionContainerRef);
api.registerSection(props.index, {
  sectionRef,
  sectionBodyRef,
  sectionSlotRef,
  sectionBodySize: toValueRef(
    computed(() => ({
      width: sectionBodyBounding.width.value,
      height: sectionBodyBounding.height.value,
    }))
  ),
  minHeight: props.minHeight,
});

defineExpose({ api });
</script>
<template>
  <div ref="sectionRef">
    <!-- Header -->
    <div
      class="flex flex-shrink-0 flex-row items-center justify-between px-3"
      :style="{ height: appearance.panelHeaderHeight + 'px' }"
    >
      <!-- Title -->
      <h3 class="select-none text-xs font-semibold tracking-wide text-gray-500">{{ title }}</h3>
      <!-- Actions -->
      <div v-if="loading">
        <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
      </div>
      <span v-else class="inline-flex flex-row gap-1">
        <button
          v-for="action in (actions ?? []).filter((action) => !action.disabled)"
          :key="action.label"
          class="inline-flex flex-row rounded-sm p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
          @click.prevent="action.action"
        >
          <component :is="action.icon" class="h-4 w-4" />
          <span class="sr-only pl-0.5 text-xs text-gray-700">{{ action.label }}</span>
        </button>
      </span>
    </div>
    <!-- Body -->
    <div
      v-if="!loading"
      ref="sectionContainerRef"
      class="scroll-hidden w-full overflow-y-scroll"
      :style="{
        maxHeight: api.heights.value[props.index] != null ? api.heights.value[props.index] + 'px' : 'none',
      }"
    >
      <!-- Inner body -->
      <div ref="sectionBodyRef" :class="sectionRef?.classList">
        <slot
          ref="sectionSlotRef"
          :navigateUp="api.navigateUp(index)"
          :navigateDown="api.navigateDown(index)"
          @navigate-up="api.navigateUp(index)"
          @navigate-down="api.navigateDown(index)"
        />
      </div>
    </div>
  </div>
</template>
