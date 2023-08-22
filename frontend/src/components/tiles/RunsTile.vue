<script lang="ts" setup>
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { useElementRefs } from "@/composables/useGrid";
import { formatDuration, useTimeFromNow } from "@/composables/useNow";
import { type Run, TriggerType } from "@/gql/graphql";
import { useRuns, getRunStatusColor, getRunStatusIconSolid } from "@/state/session";
import { useCurrentModule, TypeFlag, useNavigation } from "@/state/module";
import { useElementSize, useKeyModifier } from "@vueuse/core";
import { computed, ref, toRef, type Ref } from "vue";
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
const runRefs = useElementRefs<HTMLDivElement>();
const statement = computed(() => module.statementOf(props.runnableId));
const inputFields = computed(() => statement.value?.fields?.filter((t) => !(t.flags & TypeFlag.IsOutput)) ?? []);
const outputFields = computed(() => statement.value?.fields?.filter((t) => t.flags & TypeFlag.IsOutput) ?? []);

// navigation

const altKey = useKeyModifier("Alt");

// display

const containerRef: Ref<HTMLDivElement | null> = ref(null);
const containerSize = useElementSize(containerRef);
</script>
<template>
  <div ref="containerRef" class="flex flex-col">
    <!-- Runs -->
    <div v-if="totalCount == 0" class="flex h-full w-full items-center justify-center text-gray-400">No runs</div>
    <div v-else-if="loading" class="flex h-full w-full items-center justify-center">
      <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
    </div>
    <div v-else class="relative flex flex-col">
      <div v-if="(runs?.length ?? 0) == 0" class="w-full text-center"><span class="text-gray-400">No runs</span></div>
      <!-- Each run -->
      <div
        v-for="(run, y) in runs"
        :key="run.id"
        :ref="(el: any) => runRefs.registerRef(run.id, el)"
        tabindex="-1"
        class="group/run flex flex-row justify-between gap-5 rounded-sm px-1 py-2"
        :class="[y > 0 ? 'border-t- border-orange-900 border-opacity-[12%]' : '']"
      >
        <!-- Metadata -->
        <div class="flex w-48 flex-shrink-0 flex-col self-start">
          <!-- Status & timing -->
          <span class="transtion flex max-w-full flex-row items-center" :class="getRunStatusColor(run.status)">
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
                {{
                  run.duration != null ? formatDuration(run.duration * 1000) : now.getTimeFromNowString(run.startedAt)
                }}
              </span>
              <RunCacheInfo :run="(run as Run)" />
            </span>
          </span>
          <!-- Trigger -->
          <span class="flex w-full flex-row items-center gap-1">
            <!-- Type -->
            <component :is="TRIGGER_ICONS_SOLID[run.triggerType ?? TriggerType.Time]" class="h-4 w-4 text-gray-400" />
            <!-- From -->
            <span class="text-gray-400">{{ now.getTimeFromNowString(run.startedAt ?? run.createdAt) }}</span>
            <!-- Detail -->
          </span>
        </div>
        <!-- Selected fields as a preview: inputs top, outputs bottom -->
        <div class="flex flex-1 flex-col">
          <div
            v-for="k in ['inputs', 'outputs']"
            :key="k"
            class="relative flex w-full max-w-full flex-row justify-normal gap-x-3 overflow-hidden truncate"
          >
            <span v-if="(k == 'inputs' ? inputFields : outputFields).length == 0" class="text-sm text-gray-400">
              No {{ k }}
            </span>
            <ValueInterface
              v-for="field in k == 'inputs' ? inputFields : outputFields"
              :key="field.id"
              :type="module.effectiveTypeOf(field)"
              readonly
              active
              :wrap="false"
              :model-value="run[k as keyof typeof run]?.[module.getTypedKey(field) as string]"
              class="overflow-hidden"
            />
          </div>
        </div>
        <!-- Metadata & id -->
        <div class="flex flex-1 flex-col">
          <span class="text-sm text-gray-400">No metadata</span>
          <span class="text-sm text-gray-400">#{{ getUUIDFromGlobalID(run.id) }}</span>
        </div>
      </div>
    </div>
    <!-- Load more -->
  </div>
</template>
