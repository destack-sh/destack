<script lang="ts" setup>
import { useLoadedGraph } from "@/system/connection";
import { LOCAL_SPACE_PTR, spacePtr } from "@/system/local";
import { space } from "@/system/space";
import Windowed from "@/views/containers/Windowed.vue";
import Bar from "@/views/system/Bar.vue";
import { useWindowSize } from "@vueuse/core";
import { computed, ref } from "vue";

const BAR_HEIGHT = 42;
const BAR_OFFSET = 0;
const spaceRef = ref<HTMLElement | null>(null);
const barRef = ref<InstanceType<typeof Bar> | null>(null);
const windowRef = ref<InstanceType<typeof Windowed> | null>(null);
const { width: spaceWidth, height: spaceHeight } = useWindowSize(); // Space must be root element
const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(
  computed(() => spacePtr.value ?? LOCAL_SPACE_PTR),
);
</script>

<template>
  <div ref="spaceRef" class="scrollbar-none max-h-screen w-full overflow-hidden overscroll-none bg-gray-100 text-sm">
    <Bar
      ref="barRef"
      class="w-full border-b-2 border-gray-300"
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
* {
  /* stop overscrolling */
  overscroll-behavior: none;
  scrollbar-gutter: overlay;
}

.scrollbar-none {
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.scrollbar-none::-webkit-scrollbar {
  display: none;
}

::selection {
  background-color: #fbbf24;
}
</style>
