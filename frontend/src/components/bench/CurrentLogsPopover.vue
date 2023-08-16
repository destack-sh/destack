<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { Bars3BottomLeftIcon } from "@heroicons/vue/24/outline";
</script>
<template>
  <Popover v-slot="{ open }" class="relative">
    <PopoverButton
      ref="deployButtonRef"
      class="relative flex flex-row items-center rounded-sm px-1 py-1 text-sm focus:outline-none"
      disabled
      :class="{
        // 'hover:bg-orange-100 text-orange-600': true, disabled
        'text-gray-400': true,
        'bg-orange-100': open,
      }"
    >
      <Bars3BottomLeftIcon class="h-5 w-5" />
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-30 mt-0 flex w-[500px] flex-col gap-2 rounded-sm bg-white px-4 pb-4 pt-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <h2 class="font-bold text-gray-900">Logs</h2>
        <!-- TODO @Feature: global logs -->
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
