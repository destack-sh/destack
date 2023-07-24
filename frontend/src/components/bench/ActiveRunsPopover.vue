<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { RunStatus } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useCurrentModule } from "@/state/module";
import { getStatusIconSolid, useCurrentSessions } from "@/state/session";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { StopIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const appearance = useAppearance();
const module = useCurrentModule();
const sessions = useCurrentSessions();

const activeRuns = sessions.activeRoots;
const activeRunsAsc = computed(() => activeRuns.value.slice().sort((a, b) => b.createdAt.compareTo(a.createdAt)));
const activeRunsDesc = computed(() => activeRuns.value.slice().sort((a, b) => a.createdAt.compareTo(b.createdAt)));
</script>
<template>
  <Popover v-slot="{ open }" class="relative">
    <PopoverButton
      v-if="activeRuns.length > 0"
      ref="deployButtonRef"
      class="relative flex flex-row gap-1 rounded-sm p-1 text-sm focus:outline-none"
      :class="{
        'text-orange-600 hover:bg-orange-100': true,
        'bg-orange-100': open,
      }"
    >
      <BusySpinnerIcon class="h-4 w-4 animate-spin" />
      <span class="text-gray-900" v-if="activeRuns.length > 0">
        {{ module.statementOf(activeRunsAsc[0].runnable?.id)?.name }}
      </span>
      <span v-if="activeRuns.length > 1">+{{ activeRuns.length - 1 }}</span>
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-30 mt-0 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 pb-4 pt-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <h2 class="font-bold text-gray-900">Active runs</h2>
        <div v-if="activeRuns.length > 0" class="mt-1 flex flex-col gap-0.5">
          <div v-for="run in activeRunsDesc" :key="run.id" class="flex flex-row justify-between gap-1 px-3 py-0.5">
            <span>
              <component
                :is="getStatusIconSolid(run.status)"
                class="h-4 w-4"
                :class="[run.status == RunStatus.Running || run.status == RunStatus.Queued ? 'animate-spin' : '']"
              />
              <span class="text-gray-900"> {{ module.statementOf(run.runnable?.id)?.name ?? "untitled" }}</span>
            </span>
            <button
              class="p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
              @click="sessions.cancel(run)"
            >
              <StopIcon class="h-4 w-4 text-gray-900" />
            </button>
          </div>
        </div>
        <div v-if="activeRuns.length == 0" class="mt-1 text-center">
          <span class="px-3 text-gray-400">No active runs.</span>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
