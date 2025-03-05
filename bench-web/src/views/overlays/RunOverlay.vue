<script lang="ts" setup>
import { getRunDurationString, isRunActive } from "@/language/runtime/run";
import { RunStatusOptionInfo } from "@/proto/wire";
import { CLEAR_RUN_ACTION, getRunActions, runtime } from "@/runtime/runtime";
import { IconInline, makeIcon } from "@/ui/icon";
import { getRunColorHex } from "@/ui/style";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { computed } from "vue";

const runTree = runtime.focusedRunTree;
const run = runTree.runRef;
const base = runTree.base;

const actions = computed(() => {
  if (run.value == null) return [];
  const actions = getRunActions(run.value);
  actions.push(CLEAR_RUN_ACTION);
  return actions;
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
      <div
        v-if="run && base"
        class="relative z-50 flex h-[50px] w-[500px] items-center justify-between rounded-lg rounded-b-none border border-b-0 border-gray-400 bg-white shadow-lg transition-colors duration-150"
      >
        <!-- Node -->
        <div class="absolute left-4 flex flex-row items-center gap-x-2">
          <!-- Status -->
          <IconInline
            v-bind="makeIcon(RunStatusOptionInfo[run.status]!.icon!)"
            :class="[isRunActive(run) ? 'animate-spin' : '']"
            :style="{ color: getRunColorHex(run.status) }"
            class="text-base"
          />
          <!-- Duration -->
          <span class="text-base text-gray-900">{{ getRunDurationString(run, { minUnit: "s" }) }}</span>
        </div>
        <!-- Run -->
        <div class="absolute left-1/2 flex -translate-x-1/2 flex-row items-center gap-x-1">
          <NodeReference :node="base" size="large" />
        </div>
        <!-- Meta -->
        <div class="absolute right-4 flex flex-row items-center gap-x-1">
          <!-- Actions -->
          <button
            v-for="action in actions"
            :key="action.title"
            class="rounded-md p-1 text-gray-700 hover:bg-gray-200"
            @click="action.action"
          >
            <IconInline v-bind="action.icon" class="text-base" />
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>
