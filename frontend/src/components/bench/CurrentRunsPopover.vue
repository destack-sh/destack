<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { StatementType } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { useCurrentModule, useNavigation, type InterpStatement, TypeFlag } from "@/state/module";
import { getRunStatusIconSolid, getRunStatusColor, useCurrentSessions } from "@/state/session";
import { getStatementIconSolid } from "@/state/statement";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { PlayIcon, StopIcon } from "@heroicons/vue/24/outline";
import { useKeyModifier } from "@vueuse/core";
import { computed, ref } from "vue";

const bench = useBenchState();
const module = useCurrentModule();
const sessions = useCurrentSessions();
const nav = useNavigation();

const altKey = useKeyModifier("Alt");

const activeRuns = sessions.activeRoots;
const activeRunsAsc = computed(() => activeRuns.value.slice().sort((a, b) => b.createdAt.localeCompare(a.createdAt)));
const activeRunsDesc = computed(() => activeRuns.value.slice().sort((a, b) => a.createdAt.localeCompare(b.createdAt)));

const runnables = module.statementsLike({
  types: [StatementType.Code, StatementType.Task, StatementType.Flow],
});
const suggestedPreviewLength = ref(5);
const suggestedRunnables = computed(() => {
  let candidates = runnables.value.slice().sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  if (bench.focusedFileId != null) {
    // put those on top that are in the same file
    candidates = candidates
      .filter((r) => r.file.id == bench.focusedFileId)
      .concat(candidates.filter((r) => r.file.id != bench.focusedFileId));
  }
  return candidates;
});

function run(statement: InterpStatement) {
  bench.openLaunch(statement, { focus: true });
}
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
        class="absolute right-0 top-10 z-30 mt-0 flex w-96 flex-col rounded-sm bg-white px-4 pb-4 pt-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <h2
          class="font-bold text-gray-900 underline-offset-4 hover:cursor-pointer hover:underline"
          @click="bench.openRuns(undefined, { focus: true })"
        >
          Runs
        </h2>
        <!-- Active runs -->
        <div v-if="activeRuns.length > 0" class="mt-2 flex flex-col gap-0.5">
          <!-- Run -->
          <div v-for="run in activeRunsDesc" :key="run.id" class="relative flex flex-row justify-between gap-1 py-0.5">
            <!-- Run preview -->
            <div class="flex flex-row items-center">
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
            </div>
            <!-- Controls -->
            <button
              class="p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
              @click="sessions.cancel(run)"
            >
              <StopIcon class="h-4 w-4 text-gray-900" />
            </button>
          </div>
        </div>
        <div v-else class="mt-2">
          <span class="text-gray-400">No active runs.</span>
        </div>
        <!-- Suggested runnables -->
        <div class="mt-2 text-xs font-semibold text-gray-500">Suggested</div>
        <div class="flex flex-col gap-0.5" v-if="suggestedRunnables.length > 0">
          <div
            v-for="statement in suggestedRunnables.slice(0, suggestedPreviewLength)"
            :key="statement.id"
            class="flex flex-row justify-between gap-1 py-0.5"
          >
            <!-- Statement -->
            <div class="flex flex-row items-center gap-1">
              <component :is="getStatementIconSolid(statement.type)" class="h-4 w-4 text-gray-500" />
              <span class="text-gray-900">{{ statement.name ?? "(unnamed)" }}</span>
            </div>
            <!-- Controls -->
            <div class="flex flex-row">
              <button class="p-0.5 text-gray-400 hover:bg-orange-100 hover:text-gray-700" @click="() => run(statement)">
                <PlayIcon class="h-4 w-4" />
              </button>
            </div>
          </div>
          <!-- Show more -->
          <div v-if="suggestedRunnables.length > suggestedPreviewLength" class="flex flex-row justify-center">
            <button
              class="text-xs text-gray-400 hover:text-gray-700 hover:underline"
              @click="suggestedPreviewLength += 5"
            >
              Show more
            </button>
          </div>
        </div>
        <div v-else>
          <span class="text-gray-400">No runnable statements yet.</span>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
