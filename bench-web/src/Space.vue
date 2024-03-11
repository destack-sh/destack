<script lang="ts" setup>
import { useLoadedGraph } from "@/system/connection";
import { LOCAL_SPACE_PTR, spacePtr } from "@/system/local";
import { space } from "@/system/space";
import Windowed from "@/views/containers/Windowed.vue";
import Bar from "@/views/system/Bar.vue";
import { useWindowSize } from "@vueuse/core";
import { computed, ref } from "vue";

const BAR_HEIGHT = 40;
const BAR_OFFSET = 0;
const spaceRef = ref<HTMLElement | null>(null);
const barRef = ref<InstanceType<typeof Bar> | null>(null);
const windowRef = ref<InstanceType<typeof Windowed> | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(computed(() => spacePtr.value ?? LOCAL_SPACE_PTR));
</script>

<template>
  <div ref="spaceRef" class="max-h-screen w-full overflow-hidden bg-gray-100">
    <Bar
      ref="barRef"
      class="w-full border-b border-gray-400 shadow-sm shadow-gray-400"
      :style="{ height: BAR_HEIGHT + 'px' }"
      :space-graph="spaceGraph"
      :space-connection="spaceConnection"
    />
    <Windowed
      ref="windowRef"
      v-if="spacePtr && space"
      :self="spacePtr"
      :size="{ width: spaceWidth, height: spaceHeight - BAR_HEIGHT - BAR_OFFSET }"
      :style="{ marginTop: BAR_OFFSET + 'px' }"
    />
  </div>
</template>

<style>
body {
  /* stop overscrolling */
  overscroll-behavior: none;
}
</style>
