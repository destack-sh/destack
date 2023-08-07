<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { RunStatus } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useCurrentModule, useNavigation } from "@/state/module";
import { getRunStatusIconSolid, getRunStatusColor, useCurrentSessions } from "@/state/session";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { PlayIcon, StopIcon } from "@heroicons/vue/24/outline";
import { Bars4Icon } from "@heroicons/vue/24/solid";
import { useKeyModifier } from "@vueuse/core";
import { computed } from "vue";

const appearance = useAppearance();
const module = useCurrentModule();
const sessions = useCurrentSessions();
const nav = useNavigation();

const altKey = useKeyModifier("Alt");
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
      <Bars4Icon class="h-5 w-5" />
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
