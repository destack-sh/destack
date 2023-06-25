<script lang="ts" setup>
import { computed } from "vue";

const props = defineProps<{
  modelValue: number[];
  readonly: boolean;
  preview: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: number[]): void;
}>();

const predefinedColors = [
  "#FFB3BA", // Light Pink
  "#FFDFBA", // Light Peach
  "#FFFFBA", // Light Yellow
  "#BAFFC9", // Light Mint
  "#BAE1FF", // Light Sky Blue
  "#A2C3FF", // Light Blue
  "#B9A3FF", // Light Lavender
  "#FFB3E6", // Light Pinkish Purple
  "#FFDFB3", // Light Orange
  "#E6B3FF", // Light Purple
];
const numColorStops = 2;

// compute color stops based on vector sub samples
const colorStops = computed(() => {
  const stops = [];
  const step = Math.floor(props.modelValue.length / numColorStops);
  for (let i = 0; i < numColorStops; i++) {
    // bucket value into pastelColors
    const value = props.modelValue[i * step];
    const bucket = Math.round(value * predefinedColors.length);
    console.log(value, i, step, bucket);
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
    <svg v-if="modelValue.length > 0" class="h-6 w-full rounded-sm">
      <defs>
        <linearGradient id="gradient" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop
            v-for="(color, i) in colorStops"
            :key="i"
            :offset="(i / colorStops.length) * 100 + '%'"
            :stop-color="color"
          />
        </linearGradient>
      </defs>
      <rect width="100%" height="100%" fill="url(#gradient)" />
    </svg>
    &nbsp;
  </div>
</template>
