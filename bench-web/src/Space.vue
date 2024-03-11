<script lang="ts" setup>
import { spacePtr } from "@/system/local";
import { space } from "@/system/space";
import Windowed from "@/views/containers/Windowed.vue";
import Bar from "@/views/system/Bar.vue";
import { useWindowSize } from "@vueuse/core";
import { ref } from "vue";

const BAR_HEIGHT = 40;
const BAR_OFFSET = 1;
const spaceRef = ref<HTMLElement | null>(null);
const barRef = ref<InstanceType<typeof Bar> | null>(null);
const windowRef = ref<InstanceType<typeof Windowed> | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
</script>

<template>
  <div ref="spaceRef" class="w-full bg-gray-100">
    <Bar ref="barRef" class="w-full shadow-sm shadow-gray-300 border-b border-gray-300" :style="{ height: BAR_HEIGHT + 'px' }" />
    <Windowed
      ref="windowRef"
      v-if="spacePtr && space"
      :self="spacePtr"
      :size="{ width: spaceWidth, height: spaceHeight - BAR_HEIGHT - BAR_OFFSET }"
      :style="{ marginTop: BAR_OFFSET + 'px'}"
    />
  </div>
</template>

<style>
body {
  /* stop overscrolling */
  overscroll-behavior: none;
}
</style>
