<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { getProcessDurationString, isProcessActive, isProcessInterrupted } from "@/language/runtime/process";
import {
  ColorShade,
  Orientation,
  RunData,
  ProcessStatus,
  ProcessStatusOptionInfo,
  ProcessableNodeData,
} from "@/proto/wire";
import { IconInline, makeIcon } from "@/ui/icon";
import { getProcessColorHex } from "@/ui/style";

const props = defineProps<{
  node: ProcessableNodeData;
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
      <span v-if="isProcessInterrupted(node)" class="relative flex h-[8px] w-[8px]">
        <span
          class="absolute inline-flex h-full w-full animate-ping rounded-full opacity-75 transition-colors duration-75"
          :style="{ backgroundColor: getProcessColorHex(node.status, ColorShade.S400) }"
        />
        <span
          class="relative inline-flex h-[8px] w-[8px] rounded-full transition-colors duration-75"
          :style="{ backgroundColor: getProcessColorHex(node.status, ColorShade.S400) }"
        />
      </span>
      <span
        v-else
        class="h-[8px] w-[8px] rounded-full transition-colors duration-75"
        :class="[isProcessActive(node) ? 'animate-pulse' : '']"
        :style="{ backgroundColor: getProcessColorHex(node.status) }"
      />
    </template>
    <template v-else-if="icon != 'hide'">
      <IconInline
        v-bind="makeIcon(ProcessStatusOptionInfo[node.status]!.icon!)"
        :class="[isProcessActive(node) ? 'animate-spin' : '']"
        :style="{ color: getProcessColorHex(node.status) }"
        class=""
      />
    </template>
    <!-- Duration -->
    <span class="text-gray-400">{{ getProcessDurationString(node, { minUnit: "s" }) }}</span>
    <!-- Highlight -->
    <IconInline
      v-if="isProcessInterrupted(node)"
      v-tooltip="{ title: toCamelName(ProcessStatus, node.status), small: true, group: 'run.status' }"
      class="text-pink-500 transition-colors duration-75"
      v-bind="makeIcon(ProcessStatusOptionInfo[node.status]!.icon!)"
    />
  </div>
</template>
