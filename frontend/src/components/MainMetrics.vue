<script lang="ts" setup>
import { computed, type Ref } from "vue";

type Metric = {
  label: string;
  description: string;
  value: number;
  unit?: string;
};

// global to scope
const globalMetrics: Ref<Metric[]> = computed(() => [
  {
    label: "Clarity",
    description: "How comprehensible the instruction is.",
    value: 78,
    unit: "%",
  },
  {
    label: "Difficulty",
    description: "How complex the instruction is.",
    value: 15,
    unit: "%",
  },
]);

// local to a build for scope
const localMetrics: Ref<Metric[]> = computed(() => [
  {
    label: "Performance",
    description: "How well the AI performs.",
    value: 90,
    unit: "%",
  },
  {
    label: "Speed",
    description: "How fast the AI is.",
    value: 3,
    unit: "/min",
  },
]);

const metricSets = computed(() => [
  {
    // obviously <main symbol> will be replaced
    label: "General",
    description: "Global metrics for <main symbol>.",
    metrics: globalMetrics.value,
  },
  {
    label: "Main",
    description: "Build-specific metrics for <main symbol>.",
    metrics: localMetrics.value,
  },
]);
</script>
<template>
  <div class="flex flex-row gap-3">
    <div v-for="metricSet in metricSets" :key="metricSet.label" class="group relative rounded-sm">
      <!-- Metric set itself -->
      <div
        class="relative flex flex-row gap-3 rounded-sm border border-amber-900 border-opacity-[12%] bg-amber-100 px-2"
      >
        <span
          v-if="metricSet.label != 'general' && metricSets.length > 2"
          class="absolute left-0 -top-2 z-[5] mx-auto w-full text-center text-xs text-amber-900"
        >
          <!-- TODO @UX: clean up multi-build metrics -->
          <span
            class="rounded-sm border border-b-0 border-l border-amber-900 border-opacity-[12%] bg-amber-100 p-0.5 py-0 text-xs"
            >{{ metricSet.label }}
          </span>
        </span>
        <!-- Metric set -->
        <button
          v-for="metric in metricSet.metrics"
          :key="metric.label"
          class="relative flex flex-row items-baseline gap-0.5 p-1.5 text-center hover:cursor-pointer hover:bg-amber-200"
        >
          <!-- Label -->
          <span class="text-sm text-gray-500">{{ metric.label.slice(0, 1) }}</span>
          <!-- Metric -->
          <span class="text-sm font-bold text-gray-900">{{ metric.value }} </span>
          <!-- Unit -->
        </button>
      </div>
      <!-- Popover details if hovered -->
      <div
        class="invisible absolute top-9 z-20 w-80 bg-white px-3 py-2 shadow-sm ring-1 ring-amber-900 ring-opacity-[12%] group-hover:visible"
      >
        <h3 class="text-sm font-bold">{{ metricSet.label }} metrics</h3>
        <p class="text-sm text-gray-500">{{ metricSet.description }}</p>
        <!-- Metric breakdown -->
        <ul class="mt-4 flex flex-col gap-3">
          <li v-for="metric in metricSet.metrics" :key="metric.label" class="flex flex-row justify-between">
            <!-- Label & description -->
            <span class="flex flex-col gap-0.5">
              <span class="text-sm font-bold text-gray-900">{{ metric.label }}</span>
              <span class="text-sm text-gray-500">
                {{ metric.description }}
              </span>
            </span>
            <!-- Value -->
            <span class="flex flex-col text-right text-sm">
              <span class="font-bold text-gray-900">{{ metric.value }}</span>
              <span class="text-sm text-gray-500" v-if="metric.unit">{{ metric.unit }}</span>
            </span>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>
