<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
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
    description: "How comprehensible the instructions is.",
    value: 78,
    unit: "%",
  },
  {
    label: "Difficulty",
    description: "How complex the instruction is.",
    value: 15,
    unit: "x",
  },
]);

// local to a build for scope
const localMetrics: Ref<Metric[]> = computed(() => [
  {
    label: "Performance",
    description: "How well the AI does.",
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
    description: "Metrics for <main symbol> in build <build>.",
    metrics: localMetrics.value,
  },
]);
</script>
<template>
  <div class="flex flex-row gap-3">
    <div v-for="metricSet in metricSets" :key="metricSet.label" class="group relative rounded-sm">
      <!-- Metric set itself -->
      <button
        class="relative flex flex-row gap-3 rounded-sm border border-sky-900 border-opacity-[12%] bg-sky-100 px-2 hover:cursor-pointer hover:bg-sky-200"
      >
        <span
          v-if="metricSet.label != 'general' && metricSets.length > 2"
          class="absolute left-0 -top-2 z-[5] mx-auto w-full text-center text-xs text-sky-900"
        >
          <!-- TODO @UX: clean up multi-build metrics -->
          <span
            class="rounded-sm border border-b-0 border-l border-sky-900 border-opacity-[12%] bg-sky-100 p-0.5 py-0 text-xs"
            >{{ metricSet.label }}
          </span>
        </span>
        <!-- Metric set -->
        <div
          v-for="metric in metricSet.metrics"
          :key="metric.label"
          class="relative flex flex-row items-start gap-1 p-1.5 text-center"
        >
          <!-- Label -->
          <span class="text-xs font-bold text-gray-500">{{ metric.label.slice(0, 1) }}</span>
          <!-- Metric -->
          <span class="text-sm font-bold text-gray-900">{{ metric.value }} </span>
          <!-- Unit -->
        </div>
      </button>
      <!-- Popover details if hovered -->
      <div
        class="invisible absolute top-10 z-20 w-80 rounded-sm bg-white px-3 py-2 shadow-sm ring-1 ring-sky-900 ring-opacity-40 group-hover:visible"
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
