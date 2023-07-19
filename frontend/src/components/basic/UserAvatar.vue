<script lang="ts" setup>
import { computed } from "vue";

const props = defineProps<{
  clientId: string;
  user: { username: string; name: string };
}>();

const getClientColors = (clientId: string) => {
  const colorMap = [
    "#f56565", // red-500
    "#ecc94b", // yellow-500
    "#48bb78", // green-500
    "#4299e1", // blue-500
    "#667eea", // indigo-500
    "#9f7aea", // purple-500
    "#ed64a6", // pink-500
    "#6b7280", // gray-500
  ];
  let seed = 0;
  for (let i = 0; i < clientId.length; i++) {
    seed += clientId.charCodeAt(i);
  }
  const colors = Array(9)
    .fill("")
    .map((_, i) => colorMap[(seed + i) % colorMap.length]);
  return colors;
};

const gridColors = computed(() => getClientColors(props.clientId));
</script>

<template>
  <div :class="['rounded-sm border-orange-900 border-opacity-[15%]']">
    <svg viewBox="0 0 30 30">
      <rect
        v-for="(color, index) in gridColors"
        :key="index"
        :fill="color"
        :x="(index % 3) * 10"
        :y="Math.floor(index / 3) * 10"
        width="10"
        height="10"
        rx="1"
        ry="1"
      />
    </svg>
  </div>
</template>
