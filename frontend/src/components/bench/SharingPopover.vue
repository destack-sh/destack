<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import Switch from "@/components/basic/Switch.vue";
import { useAppearance } from "@/state/appearance";
import { encodeSharingToken } from "@/state/auth";
import { useBenchState, type ProjectHeader } from "@/state/bench";
import { useOperations } from "@/state/operations";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { ShareIcon } from "@heroicons/vue/24/outline";

const props = defineProps<{
  project: ProjectHeader;
}>();

const appearance = useAppearance();
const bench = useBenchState();
const ops = useOperations();
</script>

<template>
  <Popover v-slot="{ open }" class="relative">
    <PopoverButton
      class="flex flex-row items-center rounded-sm px-1 py-1 text-sm focus:outline-none"
      :class="{
        'hover:bg-orange-100': true,
        'bg-orange-100': open,
      }"
    >
      <ShareIcon class="h-5 w-5 text-orange-600" />
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-30 flex w-96 flex-col gap-2 rounded-sm bg-white px-2 py-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
        unmount
      >
        <span class="px-2 font-bold">Share</span>

        <!-- You -->
        <div class="mx-2 w-full rounded-sm bg-gray-100 px-3 py-2 text-xs text-gray-700">
          <span
            >You have <span class="font-semibold">{{ project.accessLevel.toLocaleLowerCase() }}</span> access.</span
          >
        </div>

        <!-- Settings -->
        <div class="flex flex-col">
          <!-- Owner -->
          <div v-if="project.owner.__typename == 'User'" class="px-2">
            <span>{{ project.owner?.slug }}</span>
          </div>
          <!-- Org members -->
          <div v-else class="px-2">
            <span>{{ project.owner?.slug }} members</span>
          </div>
          <!-- Sharing link -->
          <div class="flex flex-row justify-between px-2">
            <span>Everyone with the link</span>
          </div>
          <!-- Project members (soon) -->
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
