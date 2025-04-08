<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { getProcessDurationString, isProcessActive, isProcessInterrupted } from "@/language/runtime/process";
import { ColorShade, Orientation, RunData, ProcessStatus, ProcessStatusOptionInfo } from "@/proto/wire";
import { IconInline, makeIcon } from "@/ui/icon";
import { getRunColorHex } from "@/ui/style";

const props = defineProps<{
  run: RunData;
  orientation?: Orientation;
  icon: "dot" | "rich" | "hide";
}>();
</script>
<template>
  <div
    class="flex items-center gap-x-2"
    :class="[orientation == Orientation.HORIZONTAL_REVERSED ? 'flex-row-reverse' : 'flex-row']"
  >
    <!-- Dot -->
    <template v-if="icon == 'dot'">
      <span v-if="isProcessInterrupted(run)" class="relative flex h-[8px] w-[8px]">
        <span
          class="absolute inline-flex h-full w-full animate-ping rounded-full opacity-75 transition-colors duration-150"
          :style="{ backgroundColor: getRunColorHex(run.status, ColorShade.S400) }"
        />
        <span
          class="relative inline-flex h-[8px] w-[8px] rounded-full transition-colors duration-150"
          :style="{ backgroundColor: getRunColorHex(run.status, ColorShade.S400) }"
        />
      </span>
      <span
        v-else
        class="h-[8px] w-[8px] rounded-full transition-colors duration-150"
        :class="[isProcessActive(run) ? 'animate-pulse' : '']"
        :style="{ backgroundColor: getRunColorHex(run.status) }"
      />
    </template>
    <template v-else-if="icon != 'hide'">
      <IconInline
        v-bind="makeIcon(ProcessStatusOptionInfo[run.status]!.icon!)"
        :class="[isProcessActive(run) ? 'animate-spin' : '']"
        :style="{ color: getRunColorHex(run.status) }"
        class=""
      />
    </template>
    <!-- Duration -->
    <span class="text-gray-400">{{ getProcessDurationString(run, { minUnit: "s" }) }}</span>
    <!-- Highlight -->
    <IconInline
      v-if="isProcessInterrupted(run)"
      v-tooltip="{ title: toCamelName(ProcessStatus, run.status), small: true, group: 'run.status' }"
      class="text-pink-500 transition-colors duration-150"
      v-bind="makeIcon(ProcessStatusOptionInfo[run.status]!.icon!)"
    />
  </div>
</template>
