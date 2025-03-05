<script lang="ts" setup>
import { runtime } from "@/runtime/runtime";
import NodeReference from "@/views/builtins/NodeReference.vue";

const runTree = runtime.focusedRunTree;
const base = runTree.base;
</script>
<template>
  <div class="absolute bottom-0 flex w-full flex-row items-center justify-center">
    <!-- nocheckin: RunOverlay -->
    <Transition
      enter-active-class="transition-all ease-in duration-75"
      enter-from-class="opacity-0 translate-y-full"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition-all ease-out duration-75"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 -translate-y-full"
      appear
    >
      <div
        v-if="base"
        class="z-50 flex w-[500px] items-center justify-center rounded-lg rounded-b-none border border-gray-400 bg-white p-2 shadow-md"
      >
        <!-- Node -->
        <NodeReference :node="base" size="large" />
        <!-- Status -->
        <!-- ... -->
        <!-- Actions -->
        <button></button>
      </div>
    </Transition>
  </div>
</template>
