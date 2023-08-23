<script lang="ts" setup>
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import { TriggerType, type SearchQuery, type SearchSort } from "@/gql/graphql";
import { useRuns, getRunStatusColor, getRunStatusIconSolid } from "@/state/session";
import { useCurrentModule, useNavigation, type NodeBase, type InterpStatement } from "@/state/module";
import { useKeyModifier } from "@vueuse/core";
import { computed, ref, toRef, type Ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { TRIGGER_ICONS_SOLID } from "@/state/trigger";
import { getUUIDFromGlobalID } from "@/utils/functools";
import { IS_DEBUG } from "@/utils/globals";
import { useBenchState } from "@/state/bench";
import { getStatementIconSolid } from "@/state/statement";

const props = defineProps<{
  projectId: string;
  projectVersionId?: string;
  rootOnly?: boolean;
  runnableIds?: string[];
  query?: SearchQuery;
  sort?: [SearchSort];
  live?: boolean;
  after?: string;
  limit?: number;
  hideHeader?: boolean;
}>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

// state

const bench = useBenchState();
const module = useCurrentModule();
const now = useTimeFromNow(100);
const nav = useNavigation();

const { runs, loading, totalCount, pageInfo } = useRuns(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    runnableIds: toRef(props, "runnableIds"),
    sessionId: ref(null),
    runId: ref(null),
    rootOnly: toRef(props, "rootOnly"),
    query: toRef(props, "query"),
    sort: toRef(props, "sort"),
    after: toRef(props, "after"),
  },
  { live: props.live, limit: props.limit, count: true }
);

const statements: Ref<Record<string, InterpStatement | null>> = computed(() => {
  const statementsById: Record<string, InterpStatement | null> = {};

  for (const run of runs.value ?? []) {
    if (statementsById[run.runnable?.id] != null) continue;
    const statement = module.statementOf(run.runnable?.id);
    if (statement != null) {
      statementsById[run.runnable?.id] = statement;
    }
  }

  return statementsById;
});

// navigation

const altKey = useKeyModifier("Alt");

// display

defineExpose({ runs, loading, totalCount, pageInfo });
</script>
<template>
  <!-- Runs -->
  <div v-if="totalCount == 0" class="flex h-full w-full items-center justify-center text-gray-400">No runs</div>
  <div v-else-if="loading" class="flex h-full w-full items-center justify-center">
    <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
  </div>
  <table v-else class="min-w-full divide-y divide-orange-900 divide-opacity-[12%]">
    <!-- Header -->
    <thead v-if="!hideHeader">
      <tr>
        <th scope="col" class="px-2.5 py-1 text-left text-sm font-semibold text-gray-900">Statement</th>
        <th scope="col" class="px-2.5 py-1 text-left text-sm font-semibold text-gray-900">Run</th>
        <th scope="col" class="px-2.5 py-1 text-left text-sm font-semibold text-gray-900">Status</th>
        <th scope="col" class="px-2.5 py-1 text-left text-sm font-semibold text-gray-900">Trigger</th>
      </tr>
    </thead>
    <!-- Runs -->
    <tbody class="divide-y divide-orange-900 divide-opacity-[12%]">
      <tr v-for="run in runs" :key="run.id" class="group/run divide-orange-900 divide-opacity-[12%]">
        <!-- Statement -->
        <td class="whitespace-nowrap px-2.5 py-1.5">
          <button
            v-if="statements[run.runnable?.id] != null"
            class="flex flex-row items-center underline-offset-2 hover:underline"
            @click="nav.focusStatement(run.runnable as NodeBase)"
          >
            <component
              :is="getStatementIconSolid((statements[run.runnable?.id] as InterpStatement).type)"
              class="mr-1 h-4 w-4 text-gray-400"
            />
            <span>{{ (statements[run.runnable?.id] as InterpStatement).name }}</span>
          </button>
        </td>
        <!-- ID (to copy) -->
        <td class="whitespace-nowrap px-2.5 py-1.5">
          <button
            class="inline-flex flex-row items-center text-gray-400 hover:underline"
            @click="bench.openRun(run, { focus: true })"
          >
            <span class="font-mono underline-offset-2">#{{ getUUIDFromGlobalID(run.id).slice(-7, -1) }}</span>
          </button>
        </td>
        <!-- Status -->
        <td class="whitespace-nowrap px-2.5 py-1.5">
          <span class="inline-flex flex-row items-center gap-1" :class="[getRunStatusColor(run.status)]">
            <component :is="getRunStatusIconSolid(run.status)" class="h-4 w-4" />
            <span>{{ run.status }}</span>
            <!-- Duration -->
            <span v-if="run.startedAt != null">
              {{ run.terminatedAt != null ? "in" : "for" }}
              {{
                run.duration != null
                  ? formatDuration(run.duration * 1000)
                  : now.getTimeFromNowString(run.startedAt, { useNow: false })
              }}
            </span>
          </span>
        </td>
        <!-- Trigger -->
        <td class="whitespace-nowrap px-2.5 py-1.5">
          <div class="flex flex-row items-center gap-1">
            <!-- Type -->
            <component :is="TRIGGER_ICONS_SOLID[run.triggerType ?? TriggerType.Time]" class="h-4 w-4 text-gray-400" />
            <!-- From -->
            <span class="text-gray-900">{{ now.getTimeFromNowString(run.startedAt ?? run.createdAt) }}</span>
            <!-- Detail -->
            <span class="text-gray-900">
              <span v-if="run.triggerUser != null">by {{ run.triggerUser.username }}</span>
              <span v-else-if="run.triggerAccessToken != null">via API</span>
              <span v-else-if="run.parent != null">
                in
                <button class="underline-offset-2 hover:underline" @click="bench.openRun(run.parent)">
                  #{{ getUUIDFromGlobalID(run.parent.id).slice(-7, -1) }}
                </button>
              </span>
              <span v-else-if="run.trigger != null">by {{ run.trigger.type.toLowerCase() }} trigger</span>
              <span v-else-if="IS_DEBUG" class="text-red-600">???</span>
            </span>
          </div>
        </td>
      </tr>
    </tbody>
  </table>
</template>
