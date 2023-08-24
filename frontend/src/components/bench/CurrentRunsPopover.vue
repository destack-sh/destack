<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useBenchState } from "@/state/bench";
import { useCurrentModule, useNavigation } from "@/state/module";
import { getRunStatusIconSolid, getRunStatusColor, useCurrentSessions } from "@/state/session";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { PlayIcon, StopIcon } from "@heroicons/vue/24/outline";
import { useKeyModifier } from "@vueuse/core";
import { computed } from "vue";

const bench = useBenchState();
const module = useCurrentModule();
const sessions = useCurrentSessions();
const nav = useNavigation();

const altKey = useKeyModifier("Alt");

const activeRuns = sessions.activeRoots;
const activeRunsAsc = computed(() => activeRuns.value.slice().sort((a, b) => b.createdAt.localeCompare(a.createdAt)));
const activeRunsDesc = computed(() => activeRuns.value.slice().sort((a, b) => a.createdAt.localeCompare(b.createdAt)));
</script>
<template>
  <Popover v-slot="{ open }" class="relative">
    <PopoverButton
      ref="deployButtonRef"
      class="relative flex flex-row items-center rounded-sm px-1 py-1 text-sm focus:outline-none"
      :class="{
        'hover:bg-orange-100': true,
        'bg-orange-100': open,
      }"
    >
      <BusySpinnerIcon v-if="activeRuns.length > 0" class="h-5 w-5 animate-spin text-gray-700" />
      <PlayIcon v-else class="h-5 w-5 text-orange-600" />
      <span class="ml-1 text-gray-900" v-if="activeRuns.length > 0">
        {{ module.statementOf(activeRunsAsc[0].runnable?.id)?.name }}
      </span>
      <span v-if="activeRuns.length > 1" class="ml-1.5 text-gray-400">+{{ activeRuns.length - 1 }}</span>
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-30 mt-0 flex w-96 flex-col gap-2 rounded-sm bg-white px-4 pb-4 pt-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <h2
          class="font-bold text-gray-900 underline-offset-4 hover:cursor-pointer hover:underline"
          @click="bench.openRuns(undefined, { focus: true })"
        >
          Runs
        </h2>
        <!-- Active runs -->
        <div v-if="activeRuns.length > 0" class="mt-1 flex flex-col gap-0.5">
          <!-- Run -->
          <div v-for="run in activeRunsDesc" :key="run.id" class="relative flex flex-row justify-between gap-1 py-0.5">
            <!-- Run preview -->
            <span class="flex flex-row items-center">
              <component
                :is="getRunStatusIconSolid(run.status)"
                class="h-4 w-4"
                :class="[getRunStatusIconSolid(run.status) == BusySpinnerIcon ? 'animate-spin' : '']"
              />
              <span class="ml-1.5" :class="[getRunStatusColor(run.status)]">
                {{ run.startedAt == null ? "..." : sessions.getDurationFormatted(run) }}
              </span>
              <span
                class="ml-1 text-gray-900 decoration-gray-700 underline-offset-4"
                :class="[altKey ? 'cursor-pointer hover:underline' : '']"
                @click="() => (altKey ? nav.focusStatement(run.runnable?.id) : null)"
              >
                {{ module.statementOf(run.runnable?.id)?.name ?? "unnamed" }}
              </span>
            </span>
            <!-- Controls -->
            <button
              class="p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
              @click="sessions.cancel(run)"
            >
              <StopIcon class="h-4 w-4 text-gray-900" />
            </button>
            <!-- Debug info -->
            <span
              v-if="bench.debug"
              class="absolute right-2 top-2 z-20 rounded-sm bg-red-200 bg-opacity-50 font-sans text-sm"
            >
              {{ run.id }}
            </span>
          </div>
        </div>
        <!-- Recent runs -->
        <!-- nocheckin -->
        <!-- Runnables -->
        <!-- nocheckin -->
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
