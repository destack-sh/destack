<script lang="ts" setup>
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import { type Run, TriggerType } from "@/gql/graphql";
import { useRuns, getRunStatusColor, getRunStatusIconSolid } from "@/state/session";
import { useCurrentModule, TypeFlag, useNavigation } from "@/state/module";
import { useKeyModifier } from "@vueuse/core";
import { computed, ref, toRef } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import RunCacheInfo from "@/components/tiles/RunCacheInfo.vue";
import { TRIGGER_ICONS_SOLID } from "@/state/trigger";
import { getUUIDFromGlobalID } from "@/utils/functools";

const props = defineProps<{
  runnableId: string;
  projectId: string;
  projectVersionId?: string;
  rootOnly?: boolean;
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
    runnableIds: computed(() => [props.runnableId]),
    sessionId: ref(null),
    runId: ref(null),
    rootOnly: toRef(props, "rootOnly"),
  },
  { live: props.live, limit: props.limit, count: true }
);
const statement = computed(() => module.statementOf(props.runnableId));
const inputFields = computed(() => statement.value?.fields?.filter((t) => !(t.flags & TypeFlag.IsOutput)) ?? []);
const outputFields = computed(() => statement.value?.fields?.filter((t) => t.flags & TypeFlag.IsOutput) ?? []);

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
  <div v-else-if="view == 'list'" class="relative grid grid-cols-3 gap-x-3 gap-y-0.5 px-1 py-1">
    <div v-if="(runs?.length ?? 0) == 0" class="w-full text-center"><span class="text-gray-400">No runs</span></div>
    <!-- Each run -->
    <template v-for="(run, i) in runs" :key="run.id">
      <!-- Metadata row -->
      <!-- Status & timing -->
      <span
        class="transtion flex max-w-full flex-row items-center"
        :class="[getRunStatusColor(run.status), i != 0 ? 'mt-2' : '']"
      >
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
          >{{ statement?.name }}</span
        >
        <!-- Duration -->
        <span class="group/cache ml-1 flex flex-row flex-nowrap items-center" v-if="run.startedAt != null">
          <span class="font-semibold">
            {{ run.duration != null ? formatDuration(run.duration * 1000) : now.getTimeFromNowString(run.startedAt) }}
          </span>
          <RunCacheInfo :run="(run as Run)" />
        </span>
      </span>
      <!-- Metadata -->
      <span class="text-gray-400" :class="[i != 0 ? 'mt-2' : '']"> No metadata </span>
      <!-- ID (copy on click) -->
      <span class="truncate text-sm text-gray-400" :class="[i != 0 ? 'mt-2' : '']"
        >#{{ getUUIDFromGlobalID(run.id) }}</span
      >

      <!-- Detail row -->
      <!-- Trigger -->
      <span class="flex w-full flex-row items-center gap-1">
        <!-- Type -->
        <component :is="TRIGGER_ICONS_SOLID[run.triggerType ?? TriggerType.Time]" class="h-4 w-4 text-gray-400" />
        <!-- From -->
        <span class="text-gray-400">{{ now.getTimeFromNowString(run.startedAt ?? run.createdAt) }}</span>
        <!-- Detail -->
        <!-- TODO -->
      </span>
      <!-- Inputs/outputs -->
      <div
        v-for="kind in ['inputs', 'outputs']"
        :key="kind"
        class="relative flex w-full max-w-full flex-row justify-normal gap-x-1.5 overflow-hidden truncate"
      >
        <span v-if="(kind == 'inputs' ? inputFields : outputFields).length == 0" class="text-sm text-gray-400">
          No {{ kind }}
        </span>
        <div
          v-for="field in (kind == 'inputs' ? inputFields : outputFields).filter(f => run[kind as keyof typeof run]?.[module.getTypedKey(f) as string] != null)"
          :key="field.id"
          class="flex max-w-full flex-row rounded-2xl bg-orange-50 px-1.5 py-0.5 ring-1 ring-inset ring-orange-900 ring-opacity-[12%]"
          :class="[]"
        >
          <span class="mr-1 text-gray-500">{{ field.name }}</span>
          <ValueInterface
            :type="module.effectiveTypeOf(field)"
            readonly
            active
            :wrap="false"
            :model-value="run[kind as keyof typeof run]?.[module.getTypedKey(field) as string]"
            class="overflow-hidden truncate"
          />
        </div>
      </div>
    </template>
  </div>
  <div v-else>unknown view {{ view }}</div>
</template>
