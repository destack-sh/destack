<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { getRunDurationString, isRunActive, isRunInterrupted } from "@/language/session";
import { ColorShade, Orientation, RunData, RunStatus } from "@/proto/wire";
import { ICON_BY_RUN_STATUS, IconInline } from "@/ui/icon";
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
    <span v-if="isRunInterrupted(run)" class="relative flex h-[8px] w-[8px]">
      <span
        class="absolute inline-flex h-full w-full animate-ping rounded-full opacity-75 transition-colors duration-150"
        :style="{ backgroundColor: getRunColorHex(run.status, ColorShade.S400) }"
      />
      <span
        class="relative inline-flex h-[8px] w-[8px] rounded-full transition-colors duration-150"
        :style="{ backgroundColor: getRunColorHex(run.status, ColorShade.S500) }"
      />
    </span>
    <span
      v-else
      class="h-[8px] w-[8px] rounded-full transition-colors duration-150"
      :class="[isRunActive(run) ? 'animate-pulse' : '']"
      :style="{ backgroundColor: getRunColorHex(run.status) }"
    />
    <!-- Duration -->
    <span class="text-gray-400">{{ getRunDurationString(run, { minUnit: "s" }) }}</span>
    <!-- Highlight -->
    <IconInline
      v-if="isRunInterrupted(run)"
      v-tooltip="{ title: toCamelName(RunStatus, run.status), small: true, group: 'run.status' }"
      class="text-pink-500 transition-colors duration-150"
      v-bind="ICON_BY_RUN_STATUS[run.status]"
    />
  </div>
</template>
