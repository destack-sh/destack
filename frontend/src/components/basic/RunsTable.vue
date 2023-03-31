<script lang="ts" setup>
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import { ANY_TYPE_NODE } from "@/components/statement";
import { formatDiffSeconds, useTimeFromNow } from "@/composables/useNow";
import { graphql } from "@/gql";
import { ExecutionStatus, ExecutionTriggerType, type SimpleType } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { SYMBOL_TYPE_KEYWORD } from "@/state/editor";
import { useExecutions } from "@/state/executions";
import { symbolOf } from "@/state/runtime";
import {
  ArrowPathRoundedSquareIcon,
  LinkIcon,
  PlayIcon,
  QuestionMarkCircleIcon,
  UserIcon,
  WrenchIcon,
  XMarkIcon,
} from "@heroicons/vue/20/solid";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, watchEffect } from "vue";

type Column = {
  name: string;
  type: SimpleType;
};

const props = defineProps<{
  projectId: string;
  projectVersionId: string;
  includeAncestorVersions?: boolean;
  buildIds?: string[];
  taskIds?: string[];
  codeIds?: string[];
  inputColumns?: Column[];
  outputColumns?: Column[];
  overrideFromProps?: boolean;
}>();

const projectId = computed(() => props.projectId);
const projectVersionId = computed(() => props.projectVersionId);
const includeAncestorVersions = ref(props.includeAncestorVersions);
const buildIds = ref(props.buildIds ?? []);
const taskIds = ref(props.taskIds ?? []);
const codeIds = ref(props.codeIds ?? []);
const symbolIds = computed(() => [...buildIds.value, ...taskIds.value, ...codeIds.value]);
const symbols = computed(() => symbolIds.value.map((id) => symbolOf(id)).filter((s) => s != undefined));

// default input columns to any-typed catch-all columns
const inputColumns = computed(() => props.inputColumns ?? [{ name: "Input", type: ANY_TYPE_NODE }]);
const outputColumns = computed(() => props.outputColumns ?? [{ name: "Output", type: ANY_TYPE_NODE }]);

const showSymbols = computed(
  () =>
    !(
      (buildIds.value.length == 1 && taskIds.value.length == 1) ||
      (buildIds.value.length <= 1 && codeIds.value.length == 1)
    )
);

// watch and sync fields from props if enabled
if (props.overrideFromProps) {
  watchEffect(() => (buildIds.value = props.buildIds ?? []));
  watchEffect(() => (taskIds.value = props.taskIds ?? []));
  watchEffect(() => (codeIds.value = props.codeIds ?? []));
  watchEffect(() => (includeAncestorVersions.value = props.includeAncestorVersions));
}

const { result: runsInfoResult } = useQuery(
  graphql(/* GraphQL */ `
    query runInfo($projectId: GlobalID!, $projectVersionId: GlobalID!) {
      project(id: $projectId) {
        id
        path
        name
        slug
      }
      projectVersion(id: $projectVersionId) {
        id
        name
        tag
        committed
        createdAt
        committedAt
      }
    }
  `),
  { projectId, projectVersionId }
);
const project = computed(() => runsInfoResult.value?.project);
const projectVersion = computed(() => runsInfoResult.value?.projectVersion);

const { executions, totalCount } = useExecutions(
  {
    projectId,
    projectVersionId,
    includeAncestorVersions,
    buildIds,
    taskIds,
    codeIds,
  },
  { root: true, live: true }
);
const { getTimeFromNowString, now } = useTimeFromNow(33);

function getTriggerIcon(type: ExecutionTriggerType) {
  const icons: Record<ExecutionTriggerType, any> = {
    [ExecutionTriggerType.UiInteractive]: UserIcon,
    [ExecutionTriggerType.RestApi]: LinkIcon,
    [ExecutionTriggerType.Job]: ArrowPathRoundedSquareIcon,
    [ExecutionTriggerType.Manual]: QuestionMarkCircleIcon,
  };
  return icons[type] ?? QuestionMarkCircleIcon;
}

const appearance = useAppearance();
defineExpose({
  totalCount,
});
</script>
<template>
  <div
    class="flex flex-col items-baseline gap-2"
    :class="{ 'font-mono': appearance.fontMono, 'text-sm': appearance.textSmall, 'text-md': !appearance.textSmall }"
  >
    <!-- Selection: filters & view -->
    <div class="flex flex-row flex-wrap items-center gap-2">
      <span class="rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 px-2.5 py-1">
        <span class="text-gray-600">Bench:</span>
        <span class="pl-1 text-gray-900">{{ project?.path.replace(".", "/") }}</span>
      </span>
      <span class="rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 px-2.5 py-1">
        <span class="text-gray-600">Version:</span>
        <span class="pl-1 text-gray-900">
          {{ projectVersion?.tag ?? projectVersion?.name ?? (projectVersion?.committed ? "Autosave" : "(Working)") }}
        </span>
      </span>
      <span
        class="flex flex-row items-center rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 px-2.5 py-1"
        v-for="symbol in symbols"
        :key="symbol?.id"
      >
        <span class="text-gray-600">{{ SYMBOL_TYPE_KEYWORD[symbol?.symbolType] }}:</span>
        <span class="pl-1 text-gray-900">{{ symbol?.name }}</span>
        <button class="ml-0.5 mt-0.5">
          <XMarkIcon class="h-4 w-4 text-gray-400 hover:text-red-600" />
        </button>
      </span>
      <!-- TODO @Incomplete: project version, build, code, task, etc filter pills -->
    </div>

    <!-- Content -->
    <table
      class="divice-opacity-[12%] mt-1 w-full items-baseline rounded-sm border border-orange-900 border-opacity-[15%]"
    >
      <!-- Selected column headers -->
      <thead class="border-b border-orange-900 border-opacity-[12%]">
        <tr class="text-center">
          <th class="px-3 py-2 font-semibold text-gray-700">Status</th>
          <th v-if="showSymbols" class="px-3 py-2 font-semibold text-gray-700">Symbols</th>
          <th v-for="column in inputColumns" :key="column.type.id" class="px-3 py-2 font-semibold text-gray-700">
            {{ column.name }}
          </th>
          <th v-for="column in outputColumns" :key="column.type.id" class="px-3 py-2 font-semibold text-gray-700">
            {{ column.name }}
          </th>
          <!-- <th class="px-3"><span class="sr-only">Action</span></th> -->
        </tr>
      </thead>
      <!-- Values  -->
      <tbody class="divide-y divide-orange-900 divide-opacity-[12%] align-top">
        <tr v-for="execution in executions" :key="execution.id">
          <!-- Metadata: status, trigger, version, etc. -->
          <td class="px-3 py-3">
            <div class="flex flex-col items-center gap-1.5">
              <!-- Status -->
              <div class="flex flex-row items-center justify-between gap-1.5 transition-all">
                <svg
                  viewBox="0 0 10 10"
                  class="h-2 w-2"
                  :class="{
                    'text-green-600': execution.status == ExecutionStatus.Completed,
                    'text-red-600':
                      execution.status == ExecutionStatus.Failed || execution.status == ExecutionStatus.Aborted,
                    'text-gray-500':
                      execution.status == ExecutionStatus.Created ||
                      execution.status == ExecutionStatus.Scheduled ||
                      execution.status == ExecutionStatus.Queued ||
                      execution.status == ExecutionStatus.Running,
                    'animate-spin': execution.status == ExecutionStatus.Running,
                  }"
                >
                  <rect width="10" height="10" rx="2" ry="2" fill="currentColor" />
                </svg>
                <span class="text-gray-700" v-if="execution.status == ExecutionStatus.Queued">queue</span>
                <span class="text-gray-900" v-else-if="execution.terminatedAt != null">
                  {{ formatDiffSeconds(execution.startedAt, execution.terminatedAt) }}
                </span>
                <span class="text-gray-900" v-else-if="execution.startedAt != null">
                  {{ formatDiffSeconds(execution.startedAt, now) }}
                </span>
              </div>
              <!-- Trigger -->
              <span class="flex flex-row items-center gap-1">
                <!-- Trigger icon -->
                <component :is="getTriggerIcon(execution.triggerType)" class="mb-[1px] h-4 w-4 text-gray-400" />
                <!-- Triggered time -->
                <span class="text-gray-700">{{ getTimeFromNowString(execution.createdAt) }}</span>
              </span>
              <!-- Version -->
              <!-- <span class="flex flex-row items-center gap-1">
                <BookmarkIcon class="w-4 h-4 text-gray-400" />
                <span class="text-gray-700">{{ execution.projectVersion.tag }}</span>
              </span> -->
            </div>
          </td>
          <!-- Symbols: build/task/code -->
          <td v-if="showSymbols" class="px-3 py-3">
            <div class="flex flex-col gap-1.5">
              <span v-if="execution.build" class="flex flex-row items-start gap-1 text-gray-700">
                <WrenchIcon class="mt-0.5 h-4 w-4 text-gray-400" />
                {{ execution.build.name }}
              </span>
              <span v-if="execution.task || execution.code" class="flex flex-row items-start gap-1 text-gray-700">
                <PlayIcon class="mt-0.5 h-5 w-5 text-gray-400" />
                {{ execution.task?.name ?? execution.code?.name }}
              </span>
            </div>
          </td>
          <!-- Inputs -->
          <td v-for="column in inputColumns" :key="column.type.id" class="px-3 py-3">
            <InlineValueCell
              :type="column.type"
              :model-value="column.type.name == null ? execution.inputs : execution.inputs?.[column.type.name]"
              :readonly="true"
              :immediate="false"
            />
          </td>
          <!-- Outputs -->
          <td v-for="column in outputColumns" :key="column.type.id" class="px-3 py-3">
            <InlineValueCell
              v-if="execution.status != ExecutionStatus.Failed"
              :type="column.type"
              :model-value="column.type.name == null ? execution.outputs : execution.outputs?.[column.type.name]"
              :readonly="true"
              :immediate="false"
            />
            <div v-else class="truncate text-red-600">
              {{ execution.error["type"] }}
            </div>
          </td>
          <!-- Actions? -->
          <!-- <td class="w-full px-1 py-2.5">.</td> -->
        </tr>
      </tbody>
    </table>
  </div>
</template>
