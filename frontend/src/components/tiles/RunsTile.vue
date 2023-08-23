<script lang="ts" setup>
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import { type Run, TriggerType, type SearchQuery } from "@/gql/graphql";
import { useRuns, getRunStatusColor, getRunStatusIconSolid } from "@/state/session";
import { useCurrentModule, useNavigation } from "@/state/module";
import { useKeyModifier } from "@vueuse/core";
import { ref, toRef } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";
import { TRIGGER_ICONS_SOLID } from "@/state/trigger";
import { getUUIDFromGlobalID } from "@/utils/functools";
import { IS_DEBUG } from "@/utils/globals";

const props = defineProps<{
  runnableIds: string[];
  projectId: string;
  projectVersionId?: string;
  rootOnly?: boolean;
  query?: SearchQuery;
  live?: boolean;
  limit?: number;
  view: "list" | "grid";
}>();
const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
}>();

// state

const module = useCurrentModule();
const now = useTimeFromNow(100);
const nav = useNavigation();

const { runs, loading, totalCount } = useRuns(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    runnableIds: toRef(props, "runnableIds"),
    sessionId: ref(null),
    runId: ref(null),
    rootOnly: toRef(props, "rootOnly"),
    query: toRef(props, "query"),
  },
  { live: props.live, limit: props.limit, count: true }
);

// navigation

const altKey = useKeyModifier("Alt");

// display
</script>
<template>
  <!-- Runs -->
  <div v-if="totalCount == 0" class="flex h-full w-full items-center justify-center text-gray-400">No runs</div>
  <div v-else-if="loading" class="flex h-full w-full items-center justify-center">
    <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
  </div>
  <div v-else-if="view == 'list'" class="relative flex flex-col gap-y-2">
    <div v-if="(runs?.length ?? 0) == 0" class="w-full text-center"><span class="text-gray-400">No runs</span></div>
    <!-- Each run -->
    <div
      v-for="(run, i) in runs"
      :key="run.id"
      class="group/run flex flex-row items-baseline gap-2.5 px-0.5 py-0.5 hover:cursor-pointer hover:bg-orange-100 focus:border-orange-900 focus:border-opacity-40 focus:bg-orange-100"
    >
      <!-- Status & timing -->
      <span class="transtion inline-flex max-w-full flex-row items-center" :class="[getRunStatusColor(run.status)]">
        <!-- Status -->
        <component
          :is="getRunStatusIconSolid(run.status)"
          class="h-4 w-4"
          :class="[getRunStatusIconSolid(run.status) == BusySpinnerIcon ? 'animate-spin' : '']"
        />
        <!-- Runnable -->
        <span
          class="ml-1 max-w-full truncate font-semibold underline-offset-4"
          :class="[altKey ? 'cursor-pointer hover:underline' : '']"
          @click="
            (e) =>
              altKey && run.runnable != null
                ? (nav.focusStatement(run.runnable), e.stopPropagation(), e.preventDefault())
                : undefined
          "
          >{{ run.runnable != null ? module.statementOf(run.runnable?.id)?.name : "???" }}</span
        >
        <!-- Duration -->
        <span class="group/cache ml-1 flex flex-row flex-nowrap items-center" v-if="run.startedAt != null">
          <span class="font-semibold">
            {{ run.duration != null ? formatDuration(run.duration * 1000) : now.getTimeFromNowString(run.startedAt) }}
          </span>
          <RunCacheInfo :run="(run as Run)" />
        </span>
      </span>
      <!-- Trigger -->
      <span class="inline-flex flex-row items-center gap-1">
        <!-- Type -->
        <component :is="TRIGGER_ICONS_SOLID[run.triggerType ?? TriggerType.Time]" class="h-4 w-4 text-gray-400" />
        <!-- From -->
        <span class="text-gray-400">{{ now.getTimeFromNowString(run.startedAt ?? run.createdAt) }}</span>
        <!-- Detail -->
        <span class="text-gray-400">
          <span v-if="run.triggerUser != null">by {{ run.triggerUser.username }}</span>
          <span v-else-if="run.triggerAccessToken != null">via API</span>
          <span v-else-if="run.parent != null">in {{ module.nodeOf(run.parent.runnable?.id)?.name }}</span>
          <span v-else-if="run.trigger != null">by {{ run.trigger.type.toLowerCase() }} trigger</span>
          <span v-else-if="IS_DEBUG" class="text-red-600">???</span>
        </span>
      </span>
      <!-- ID -->
      <span class="text-sm text-gray-400 group-hover/run:underline"
        >#{{ "..." + getUUIDFromGlobalID(run.id).slice(-6, -1) }}</span
      >
    </div>
  </div>
  <div v-else-if="view == 'grid'">
    <table class="mx-1 min-w-full divide-y divide-orange-900 divide-opacity-[12%]">
      <!-- Header -->
      <thead>
        <tr>
          <th scope="col" class="py-1 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 sm:pl-0">ID</th>
          <th scope="col" class="py-1 pl-4 pr-3 text-left text-sm font-semibold text-gray-900 sm:pl-0">Status</th>
          <th scope="col" class="px-3 py-1 text-left text-sm font-semibold text-gray-900 sm:pl-0">Runnable</th>
          <th scope="col" class="px-3 py-1 text-left text-sm font-semibold text-gray-900 sm:pl-0">Trigger</th>
          <th scope="col" class="px-3 py-1 text-left text-sm font-semibold text-gray-900 sm:pl-0">Inputs</th>
          <th scope="col" class="px-3 py-1 text-left text-sm font-semibold text-gray-900 sm:pl-0">Outputs</th>
        </tr>
      </thead>
      <!-- Runs -->
      <tbody class="divide-y divide-orange-900 divide-opacity-[12%]">
        <tr v-for="run in runs" :key="run.id" class="divde-x divide-orange-900 divide-opacity-[12%]">
          <!-- ID (to copy) -->
          <td class="whitespace-nowrap py-2 pl-4 pr-3">
            <span class="inline-flex flex-row items-center text-gray-400">
              <span>#{{ "..." + getUUIDFromGlobalID(run.id).slice(-6, -1) }}</span>
            </span>
          </td>
          <!-- Status -->
          <td class="whitespace-nowrap px-3 py-2">
            <span class="inline-flex flex-row items-center gap-1" :class="[getRunStatusColor(run.status)]">
              <component :is="getRunStatusIconSolid(run.status)" class="h-4 w-4" />
              <span>{{ run.status }}</span>
            </span>
          </td>
          <!-- Runnable -->
        </tr>
      </tbody>
    </table>
  </div>
</template>
