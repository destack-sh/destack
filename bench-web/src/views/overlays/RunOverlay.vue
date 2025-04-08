<script lang="ts" setup>
import { canvas } from "@/globals";
import { getProcessDurationString, isProcessActive } from "@/language/runtime/process";
import { ProcessStatusOptionInfo } from "@/proto/wire";
import { CLEAR_RUN_COMMAND, getRunCommands, runtime } from "@/runtime/runtime";
import { IconInline, makeIcon } from "@/ui/icon";
import { getRunColorHex } from "@/ui/style";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { computed } from "vue";

const runTree = runtime.focusedRunTree;
const run = runTree.runRef;
const base = runTree.runBaseRef;

const commands = computed(() => {
  if (run.value == null) return [];
  const commands = getRunCommands(run.value);
  commands.push(CLEAR_RUN_COMMAND);
  return commands;
});
</script>
<template>
  <div class="absolute bottom-0 flex w-full flex-row items-center justify-center">
    <Transition
      enter-active-class="transition-all ease-in duration-75"
      enter-from-class="opacity-0 translate-y-[4px]"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition-all ease-out duration-75"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 translate-y-[4px]"
      appear
    >
      <!-- NOTE :UX: maybe RunOverlay could become a general contextual overlay (also for reviewing Task queues and such?) -->
      <div
        v-if="run && base"
        class="relative z-50 flex h-[50px] w-[500px] items-center justify-between rounded-lg rounded-b-none border border-b-0 border-gray-400 bg-white shadow-lg transition-colors duration-150"
      >
        <!-- Node -->
        <div class="absolute left-5 flex flex-row items-center gap-x-2">
          <!-- Status -->
          <IconInline
            v-bind="makeIcon(ProcessStatusOptionInfo[run.status]!.icon!)"
            :class="[isProcessActive(run) ? 'animate-spin' : '']"
            :style="{ color: getRunColorHex(run.status) }"
            class="text-base"
          />
          <!-- Duration -->
          <span class="text-base text-gray-900">{{ getProcessDurationString(run, { minUnit: "s" }) }}</span>
        </div>
        <!-- Run -->
        <div class="absolute left-1/2 flex -translate-x-1/2 flex-row items-center gap-x-1">
          <NodeReference class="cursor-pointer" :node="base" size="base" @click="canvas.goToNode(base)" />
        </div>
        <!-- Meta -->
        <div class="absolute right-5 flex flex-row items-center gap-x-1">
          <!-- Commands -->
          <button
            v-for="command in commands"
            :key="command.title"
            class="rounded-md px-1.5 py-0.5 text-gray-700 transition-colors duration-150 hover:bg-gray-200"
            @click="command.command"
          >
            <IconInline v-bind="command.icon" class="text-base" />
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>
