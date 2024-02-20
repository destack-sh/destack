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

// some tailwind colors from default palette
const PALETTE_COLOR_BITS = 4; // 2^4 = 16 colors
const predefinedColors = [
  // 16 colors (4 bits)
  "#ef4444", // red-500
  "#f59e0b", // amber-500
  "#eab308", // yellow-500
  "#84cc16", // lime-500

  "#0ea5e9", // sky-500
  "#3b82f6", // blue-500
  "#6366f1", // indigo-500
  "#8b5cf6", // violet-500

  "#a855f7", // purple-500
  "#d946ef", // fuscia-500
  "#ec4899", // pink-500
  "#f43f5e", // rose-500

  "#22c55e", // green-500
  "#10b981", // emerald-500
  "#14b8a6", // teal-500
  "#06b6d4", // cyan-500
];

const instanceId = ref(Math.random().toString(36).substring(2)); // for scoping svg defs

// TODO @UX: figure out better vector color vignette/signature

// compute color stops based on vector sub samples
const colorStops = computed(() => {
  if (props.modelValue == null) return [];
  const numColorStops = Math.max(2, Math.floor(Math.sqrt(Math.sqrt(props.modelValue.length))));
  const numProbes = 8;
  const stops = [];
  const step = Math.floor(props.modelValue.length / numColorStops);
  for (let i = 0; i < numColorStops; i++) {
    let bucketBits = 0;
    for (let b = 0; b < PALETTE_COLOR_BITS; b++) {
      // set bit if more than half of probes for this bit are above zero
      let sum = 0;
      for (let j = 0; j < numProbes; j++) {
        const idx = i * step + b * PALETTE_COLOR_BITS + j;
        sum += props.modelValue[idx] ?? 0;
      }
      if (sum > 0) {
        bucketBits |= 1 << b;
      }
    }
    stops.push(predefinedColors[bucketBits]);
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
