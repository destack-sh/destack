<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { StatementType } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { useCurrentModule, useNavigation, type InterpStatement, TypeFlag } from "@/state/module";
import { getRunStatusIconSolid, getRunStatusColor, useCurrentSessions } from "@/state/session";
import { getStatementIconSolid } from "@/state/statement";
import { getUUIDFromGlobalID } from "@/utils/functools";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import {
  PlayIcon as PlayIconSolid,
  WindowIcon as WindowIconSolid,
  StopIcon as StopIconSolid,
} from "@heroicons/vue/24/solid";
import { computed, ref } from "vue";

const bench = useBenchState();
const module = useCurrentModule();
const sessions = useCurrentSessions();
const nav = useNavigation();

const activeRuns = sessions.activeRoots;
const activeRunsAsc = computed(() => activeRuns.value.slice().sort((a, b) => b.createdAt.localeCompare(a.createdAt)));
const activeRunsDesc = computed(() => activeRuns.value.slice().sort((a, b) => a.createdAt.localeCompare(b.createdAt)));

const statements = module.statementsLike({
  types: [StatementType.Code, StatementType.Task, StatementType.Flow],
});
const suggestedPreviewLength = ref(5);
const suggestedRunnables = computed(() => {
  // sort by most recently edited but put those in the current focused file first
  let candidates = statements.value
    .filter((n) => (n.name ?? "").trim().length > 0)
    .sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  if (bench.focusedFileCk != null) {
    // shift focused file's statements to top
    candidates = candidates
      .filter((r) => r.file.id == bench.focusedFileId)
      .concat(candidates.filter((r) => r.file.id != bench.focusedFileId));
  }
  return candidates;
});

function hasInputs(statement: InterpStatement) {
  return statement.fields.filter((f) => f.deletedAt == null && !(f.flags & TypeFlag.IS_OUTPUT)).length > 0;
}

function run(statement: InterpStatement) {
  sessions.run(statement);
}

function launch(statement: InterpStatement) {
  bench.openLaunchRun(statement, { focus: true });
}

function runOrLaunch(statement: InterpStatement) {
  if (!hasInputs(statement) && bench.canUse) {
    run(statement);
  } else {
    launch(statement);
  }
}
</script>
<template>
  <Popover v-slot="{ open }" class="relative">
    <PopoverButton
      class="relative flex flex-row items-center rounded-sm px-1 py-1 text-sm focus:outline-none"
      :class="{
        'hover:bg-orange-100': true,
        'bg-orange-100': open,
      }"
    >
      <PlayIconSolid class="h-5 w-5 text-orange-600" />
      <!-- little number with current runs -->
      <span
        class="absolute -bottom-1 -right-1 flex h-4 w-4 items-center justify-center rounded-full text-xs font-semibold text-gray-900 transition-opacity duration-150"
        :class="[activeRuns.length > 0 ? 'opacity-100' : 'opacity-0']"
      >
        {{ activeRuns.length }}
      </span>
    </PopoverButton>

    <FadeTransition>
      <PopoverPanel
        class="absolute right-0 top-10 z-30 mt-0 flex max-h-96 w-96 flex-col overflow-y-scroll rounded-sm bg-white px-4 pb-4 pt-2 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <h2
          class="font-bold text-gray-900 underline-offset-4 hover:cursor-pointer hover:underline"
          @click="bench.openViewRuns(undefined, { focus: true })"
        >
          Runs
        </h2>
        <!-- Active runs -->
        <div v-if="activeRuns.length > 0" class="mt-2 flex flex-col gap-0.5">
          <!-- Run -->
          <div
            v-for="run in activeRunsDesc"
            :key="run.id"
            class="relative flex max-w-full flex-row justify-between gap-1 py-0.5"
          >
            <!-- Run preview -->
            <div class="flex max-w-full flex-row items-center">
              <!-- Status -->
              <component
                :is="getRunStatusIconSolid(run.status)"
                class="h-4 w-4"
                :class="[getRunStatusIconSolid(run.status) == BusySpinnerIcon ? 'animate-spin' : '']"
              />
              <span v-if="run.startedAt" class="ml-1.5" :class="[getRunStatusColor(run.status)]">
                {{ sessions.getDurationFormatted(run) }}
              </span>
              <!-- Statement -->
              <span
                class="ml-1 cursor-pointer truncate text-gray-900 decoration-gray-700 underline-offset-2 hover:underline"
                @click="nav.focusStatement(run.statementCk)"
              >
                {{ module.statementOf(run.statementCk)?.name ?? "unnamed" }}
              </span>
              <!-- Run id -->
              <span
                class="ml-1 cursor-pointer text-gray-400 underline-offset-2 transition duration-150 hover:text-gray-700 hover:underline"
                @click="bench.openViewRun(run, { focus: true })"
              >
                #{{ getUUIDFromGlobalID(run.id).slice(-7, -1) }}
              </span>
            </div>
            <!-- Controls -->
            <button
              class="p-0.5 text-gray-400 transition duration-150 hover:bg-orange-100 hover:text-gray-700"
              @click="sessions.kill(run)"
            >
              <StopIconSolid class="h-4 w-4 text-orange-600" />
            </button>
          </div>
        </div>
        <div v-else class="mt-2">
          <span class="text-gray-400">No active runs.</span>
        </div>
        <!-- Suggested statements -->
        <div class="mt-2 text-xs font-semibold text-gray-500">Suggested</div>
        <div class="flex flex-col gap-0.5" v-if="suggestedRunnables.length > 0">
          <div
            v-for="statement in suggestedRunnables.slice(0, suggestedPreviewLength)"
            :key="statement.id"
            class="group flex flex-row justify-between gap-1 rounded-sm py-0.5 transition duration-150 hover:cursor-pointer hover:bg-orange-100"
            @click="() => runOrLaunch(statement)"
          >
            <!-- Statement -->
            <div class="flex flex-row items-center gap-1">
              <component :is="getStatementIconSolid(statement.type)" class="h-4 w-4 text-gray-500" />
              <a class="cursor-pointer text-gray-900" @click="nav.focusStatement(statement)">
                {{ statement.name ?? "(unnamed)" }}
              </a>
            </div>
            <!-- Controls -->
            <div class="flex flex-row">
              <button
                v-if="!hasInputs(statement) && bench.canUse"
                class="p-0.5 text-orange-600 hover:text-orange-500 group-hover:text-orange-500"
                @click="() => run(statement)"
              >
                <PlayIconSolid class="h-4 w-4" />
              </button>
              <button
                class="p-0.5 text-orange-600 hover:text-orange-500"
                :class="[hasInputs(statement) ? 'group-hover:text-orange-500' : '']"
                @click="() => launch(statement)"
              >
                <WindowIconSolid class="h-4 w-4" />
              </button>
            </div>
          </div>
          <!-- Show more -->
          <div v-if="suggestedRunnables.length > suggestedPreviewLength" class="mt-1 flex flex-row justify-center">
            <button
              class="text-xs text-gray-400 hover:text-gray-700 hover:underline"
              @click="suggestedPreviewLength += 5"
            >
              Show more
            </button>
          </div>
        </div>
        <div v-else>
          <span class="text-gray-400">No statement statements yet.</span>
        </div>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
