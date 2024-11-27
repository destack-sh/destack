<script lang="ts" setup>
import { getRunDurationString, isRunActive, isRunInterrupted } from "@/language/session";
import { ColorShade, Orientation, RunData } from "@/proto/wire";
import { getRunColorHex } from "@/ui/style";

const props = defineProps<{
  run: RunData;
  orientation?: Orientation;
}>();
</script>
<template>
  <div
    class="flex items-center gap-x-2"
    :class="[orientation == Orientation.HORIZONTAL_REVERSED ? 'flex-row-reverse' : 'flex-row']"
  >
    <!-- Dot -->
    <span v-if="isRunInterrupted(run)" class="relative flex h-[10px] w-[10px]">
      <span
        class="absolute inline-flex h-full w-full animate-ping rounded-full opacity-75"
        :style="{ backgroundColor: getRunColorHex(run.status, ColorShade.S400) }"
      />
      <span
        class="relative inline-flex h-[10px] w-[10px] rounded-full"
        :style="{ backgroundColor: getRunColorHex(run.status, ColorShade.S500) }"
      />
    </span>
    <span
      v-else
      class="h-[10px] w-[10px] rounded-full"
      :class="[isRunActive(run) ? 'animate-pulse' : '']"
      :style="{ backgroundColor: getRunColorHex(run.status) }"
    />
    <!-- Duration -->
    <span class="text-gray-400">{{ getRunDurationString(run, { minUnit: "s" }) }}</span>
    <!-- Highlight -->
    <i
      v-if="isRunInterrupted(run)"
      v-tooltip="{ title: 'Interrupted', small: true, group: 'run.status' }"
      class="fas fa-hand rounded text-pink-500"
    />
  </div>
</template>
