<script lang="ts" setup>
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import { TriggerType, type Conditional, type Sort, type Run } from "@/gql/graphql";
import { useRuns, getRunStatusColor, getRunStatusIconSolid, RUN_STATUS_NAME } from "@/state/session";
import { useCurrentModule, useNavigation, type NodeBase, type InterpStatement } from "@/state/module";
import { useKeyModifier } from "@vueuse/core";
import { computed, ref, toRef, type Ref } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { TRIGGER_ICONS_SOLID } from "@/state/trigger";
import { getUUIDFromGlobalID } from "@/utils/functools";
import { IS_DEBUG } from "@/utils/globals";
import { useBenchState } from "@/state/bench";
import { getStatementIconSolid } from "@/state/statement";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";
import { DateTime } from "luxon";

const props = defineProps<{
  projectId: string;
  projectVersionId?: string;
  rootOnly?: boolean;
  statementCks?: string[];
  query?: Conditional;
  sort?: [Sort];
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
    statementCks: toRef(props, "statementCks"),
    sessionId: ref(null),
    runId: ref(null),
    rootOnly: toRef(props, "rootOnly"),
    query: toRef(props, "query"),
    sort: toRef(props, "sort"),
    after: toRef(props, "after"),
  },
  { live: props.live, limit: props.limit, count: true }
);

const statementsByCk: Ref<Record<string, InterpStatement | null>> = computed(() => {
  const statementsByCk: Record<string, InterpStatement | null> = {};

  for (const run of runs.value ?? []) {
    if (statementsByCk[run.statementCk] != null) continue;
    const statement = module.statementOf(run.statementCk);
    if (statement != null) {
      statementsByCk[run.statementCk] = statement;
    }
  }

  return statementsByCk;
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
    <tbody class="divide-y divide-orange-900 divide-opacity-[12%] text-gray-900">
      <tr v-for="run in runs" :key="run.id" class="group/run divide-orange-900 divide-opacity-[12%]">
        <!-- ID (to copy) -->
        <td class="whitespace-nowrap px-2.5 py-1.5">
          <button
            class="flex flex-row items-center hover:underline"
            @click.stop="bench.openViewRun(run, { focus: true })"
          >
            <span class="font-mono underline-offset-2">#{{ getUUIDFromGlobalID(run.id).slice(-7, -1) }}</span>
          </button>
        </td>
        <!-- Statement -->
        <td class="whitespace-nowrap px-2.5 py-1.5">
          <button
            v-if="statementsByCk[run.statementCk] != null"
            class="flex flex-row items-center underline-offset-2 hover:underline"
            @click.stop.prevent="nav.focusStatement(statementsByCk[run.statementCk] as NodeBase)"
          >
            <component
              :is="getStatementIconSolid((statementsByCk[run.statementCk] as InterpStatement).type)"
              class="mr-1 h-4 w-4 text-gray-400"
            />
            <span>{{ (statementsByCk[run.statementCk] as InterpStatement).name }}</span>
          </button>
          <span v-else class="text-gray-400">(not found)</span>
        </td>
        <!-- Status -->
        <td class="whitespace-nowrap px-2.5 py-1.5">
          <div class="flex flex-row items-center gap-1" :class="[getRunStatusColor(run.status)]">
            <component
              :is="getRunStatusIconSolid(run.status)"
              class="h-4 w-4"
              :class="[getRunStatusIconSolid(run.status) == BusySpinnerIcon ? 'animate-spin' : '']"
            />
            <span>{{ RUN_STATUS_NAME[run.status] }}</span>
            <!-- Duration -->
            <span v-if="run.startedAt != null">
              {{ run.terminatedAt != null ? "in" : "for" }}
              {{
                run.duration != null
                  ? formatDuration(run.duration * 1000)
                  : now.getTimeFromNowString(run.startedAt, { useNow: false })
              }}
            </span>
            <RunCacheInfo :run="(run as Run)" class="" />
          </div>
        </td>
        <!-- Trigger -->
        <td class="whitespace-nowrap px-2.5 py-1.5">
          <div class="group relative flex flex-row items-center gap-1">
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
                <button
                  class="underline-offset-2 hover:underline"
                  @click.stop="bench.openViewRun(run.parent, { focus: true })"
                >
                  #{{ getUUIDFromGlobalID(run.parent.id).slice(-7, -1) }}
                </button>
              </span>
              <span v-else-if="run.trigger != null">by {{ run.trigger.type.toLowerCase() }}</span>
              <span v-else-if="IS_DEBUG" class="text-red-600">???</span>
            </span>
            <!-- Trigger time popover -->
            <!-- Label (yeah these should be refactored) -->
            <span
              class="pointer-events-none absolute right-5 top-5 z-10 whitespace-nowrap rounded-sm border border-orange-900 border-opacity-[15%] bg-white px-2 py-0.5 text-center text-xs text-gray-500 opacity-0 transition duration-150 group-hover:opacity-100"
            >
              {{ DateTime.fromISO(run.startedAt ?? run.createdAt).toFormat("yyyy-MM-dd HH:mm:ss.SSS") }}
            </span>
          </div>
        </td>
      </tr>
    </tbody>
  </table>
</template>
