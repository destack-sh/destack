<script lang="ts" setup>
import { useBenchState } from "@/state/bench";
import { useRelativeDropZone } from "@/utils/drop";
import { ref } from "vue";

const props = defineProps<{ position: "start" | "end" }>();
const buttonRef = ref(null);
const { isOverDropZone } = useRelativeDropZone(buttonRef, ["BrowserFile"]);
const bench = useBenchState();
</script>
<template>
  <button ref="buttonRef" class="group relative flex cursor-default py-1 outline-none transition duration-150">
    <!-- Drag indicators (bottom if start, top if end) :DragStyle -->
    <div
      v-if="position == 'end'"
      class="absolute -top-0.5 left-0 z-[5] h-1 w-full bg-orange-200 transition duration-150"
      :class="[isOverDropZone && !bench.readonly ? 'opacity-100' : 'opacity-0']"
    />
    <div
      v-if="position == 'start'"
      class="absolute -bottom-0.5 left-0 z-[5] h-1 w-full bg-orange-200 transition duration-150"
      :class="[isOverDropZone && !bench.readonly ? 'opacity-100' : 'opacity-0']"
    />
  </button>
</template>
