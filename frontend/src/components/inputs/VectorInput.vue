<script lang="ts" setup>
import { computed, ref } from "vue";

const props = defineProps<{
  modelValue: number[] | undefined;
  readonly: boolean;
  preview: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: number[]): void;
}>();

// tailwind -500 pastelle colors from default palette
const predefinedColors = [
  "#f59e0b",
  "#84cc16",
  "#22c55e",
  "#10b981",
  "#14b8a6",
  "#0ea5e9",
  "#3b82f6",
  "#6366f1",
  "#8b5cf6",
  "#ec4899",
  "#f43f5e",
];
const numColorStops = 3;

const instanceId = ref(Math.random().toString(36).substring(2)); // for scoping svg defs

// compute color stops based on vector sub samples
const colorStops = computed(() => {
  if (props.modelValue == null) return [];
  const stops = [];
  const step = Math.floor(props.modelValue.length / numColorStops);
  for (let i = 0; i < numColorStops; i++) {
    const value = props.modelValue[i * step];
    const bucket = Math.round(value * predefinedColors.length) % predefinedColors.length;
    stops.push(predefinedColors[bucket]);
  }
  return stops;
});

defineExpose({
  click: () => {
    /* noop */
  },
  focus: () => {
    /* noop */
  },
  blur: () => {
    /* noop */
  },
});
</script>
<template>
  <div class="h-full w-full">
    <svg v-if="(modelValue?.length ?? 0) > 0" class="h-6 w-full rounded-sm opacity-50 blur-xs">
      <defs>
        <linearGradient :id="'vector-gradient-' + instanceId" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop
            v-for="(color, i) in colorStops"
            :key="i"
            :offset="(i / (colorStops.length - 1)) * 100 + '%'"
            :stop-color="color"
          />
        </linearGradient>
      </defs>
      <rect width="100%" height="100%" :fill="'url(#vector-gradient-' + instanceId + ')'" />
    </svg>
    &nbsp;
  </div>
</template>
