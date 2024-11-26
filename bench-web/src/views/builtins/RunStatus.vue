<script lang="ts" setup>
import { getRunDurationString, isRunActive } from "@/language/session";
import { Orientation, RunData } from "@/proto/wire";
import { getRunColorHex } from "@/ui/style";

const props = defineProps<{
  run: RunData;
  orientation?: Orientation;
}>();
</script>
<template>
  <div
    class="flex items-center gap-x-1"
    :class="[orientation == Orientation.HORIZONTAL_REVERSED ? 'flex-row-reverse' : 'flex-row']"
  >
    <span
      class="fas fa-circle-small w-5 text-center"
      :class="[isRunActive(run) ? 'animate-pulse' : '']"
      :style="{ color: getRunColorHex(run.status) }"
    />
    <span class="text-gray-400">{{ getRunDurationString(run, { minUnit: "s" }) }}</span>
  </div>
</template>
