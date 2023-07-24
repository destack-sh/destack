<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { useActiveScroll } from "@/composables/useScroll";
import { graphql } from "@/gql";
import { WorkerSetStatus } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useBenchState } from "@/state/bench";
import { useCurrentSessions, WORKER_RESOURCES_BY_PROFILE } from "@/state/session";
import { ArrowPathIcon, StopIcon } from "@heroicons/vue/24/outline";
import { CheckCircleIcon, PauseIcon, QuestionMarkCircleIcon, XCircleIcon } from "@heroicons/vue/24/solid";
import { CodeBracketSquareIcon, ServerStackIcon } from "@heroicons/vue/24/solid";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<{
  active: boolean;
  focused: boolean;
  containerSize: { width: Ref<number>; height: Ref<number> };
}>();
const emit = defineEmits<{ (e: "show"): void; (e: "blur"): void }>();

const bench = useBenchState();
const session = useCurrentSessions();
const workerSet = session.workerSet;
const appearance = useAppearance();

const packagesTableRef = ref<HTMLDivElement | null>(null);
useActiveScroll(packagesTableRef);

const { result: environmentQuery, loading } = useQuery(
  graphql(/* GraphQL */ `
    query workerEnvironment($projectId: GlobalID!) {
      environment(projectId: $projectId) {
        ... on Environment {
          language
          version
          platform
          packages {
            name
            version
          }
        }
      }
    }
  `),
  {
    projectId: toRef(bench, "projectId"),
  },
  {
    enabled: computed(() => props.active) as any,
  }
);
const environment = computed(() =>
  environmentQuery.value?.environment.__typename == "Environment" ? environmentQuery.value?.environment : undefined
);

const workerStatusColor = computed(() => ({
  [WorkerSetStatus.Pending]: "text-yellow-700",
  [WorkerSetStatus.Healthy]: "text-green-700",
  [WorkerSetStatus.Unhealthy]: "text-red-700",
  [WorkerSetStatus.Updating]: "text-gray-700",
  [WorkerSetStatus.Sleeping]: "text-gray-700",
  [WorkerSetStatus.Unknown]: "text-gray-700",
}));

const workerStatusIcon = computed(() => ({
  [WorkerSetStatus.Pending]: BusySpinnerIcon,
  [WorkerSetStatus.Healthy]: CheckCircleIcon,
  [WorkerSetStatus.Unhealthy]: XCircleIcon,
  [WorkerSetStatus.Updating]: BusySpinnerIcon,
  [WorkerSetStatus.Sleeping]: PauseIcon,
  [WorkerSetStatus.Unknown]: QuestionMarkCircleIcon,
}));
const workerStatusTitle = computed(() => ({
  [WorkerSetStatus.Pending]: "Pending",
  [WorkerSetStatus.Healthy]: "Healthy",
  [WorkerSetStatus.Unhealthy]: "Unhealthy",
  [WorkerSetStatus.Updating]: "Updating",
  [WorkerSetStatus.Sleeping]: "Sleeping",
  [WorkerSetStatus.Unknown]: "Unknown",
}));

const resourcesInfo = computed(() => {
  if (workerSet.value == null) return undefined;
  const profile = workerSet.value.profile;
  return WORKER_RESOURCES_BY_PROFILE[profile];
});
</script>
<template>
  <div class="flex flex-col">
    <!-- View header -->
    <div
      class="flex h-[31px] flex-row items-center justify-between px-3 py-2"
      :style="{
        height: appearance.editorHeaderHeight + 'px',
      }"
    >
      <span class="text-xs font-semibold tracking-wide text-gray-500">Environment</span>
      <div v-if="loading">
        <BusySpinnerIcon class="h-4 w-4 animate-spin text-gray-500" />
      </div>
    </div>
    <!-- Environment -->
    <div
      class="mt-0.5 flex flex-col gap-y-0.5 px-3 text-sm"
      v-if="environment != null && resourcesInfo != null && workerSet != null"
    >
      <!-- Language -->
      <div class="flex flex-row items-center">
        <span class=""><CodeBracketSquareIcon class="mr-2 h-4 w-4 text-gray-400" /></span>
        <span class="whitespace-nowrap font-semibold text-gray-900">Python {{ environment.version }}</span>
      </div>
      <!-- Profile -->
      <div class="flex flex-row items-center">
        <span class=""><ServerStackIcon class="mr-2 h-4 w-4 text-gray-400" /></span>
        <span class="whitespace-nowrap font-semibold text-gray-900">
          <span>{{ workerSet.desiredReplicas }}x {{ resourcesInfo.name }}</span>
          <div class="ml-1.5 inline font-normal text-gray-500">
            {{ resourcesInfo.cpu }}vCPU + {{ resourcesInfo.mem }}GB
          </div>
        </span>
      </div>
      <!-- Status/actions -->
      <div class="flex flex-row items-center">
        <component
          :is="workerStatusIcon[workerSet.status]"
          class="mr-2 h-4 w-4"
          :class="workerStatusColor[workerSet.status]"
        />
        <span class="mr-1.5 whitespace-nowrap font-semibold text-gray-900" :class="workerStatusColor[workerSet.status]">
          {{ workerStatusTitle[workerSet.status] }}
        </span>
        <!-- Actions -->
        <BusySpinnerIcon v-if="session.restarting.value || session.waking.value" class="h-4 w-4 animate-spin" />
        <button
          v-else-if="workerSet.status == WorkerSetStatus.Healthy || workerSet.status == WorkerSetStatus.Unhealthy"
          class="rounded-sm px-0.5 text-gray-500 hover:bg-orange-100 hover:text-gray-700"
          @click="session.restartWorkerSet()"
        >
          Restart
        </button>
        <button
          v-else-if="workerSet.status == WorkerSetStatus.Sleeping"
          class="rounded-sm px-0.5 text-gray-500 hover:bg-orange-100 hover:text-gray-700"
          @click="session.wakeWorkerSet()"
        >
          Wake
        </button>
      </div>
      <!-- TODO @UX: view actual nodes, latency and resource usage here -->
      <!-- maybe also show object storage usage, total records, etc.? -->
    </div>
    <!-- Packages -->
    <div class="relative mt-4 w-full flex-1 text-sm" v-if="environment != null">
      <span class="px-3 text-xs font-semibold tracking-wide text-gray-500">Packages</span>
      <!-- Hacky way to make packages list fit the remaining space -->
      <div
        class="relative mt-1 w-full"
        :style="{
          // max height - header height
          height: 'calc(100% - ' + appearance.editorHeaderHeight + 'px)',
        }"
      >
        <div class="absolute left-0 top-0 h-full w-full overflow-y-scroll" ref="packagesTableRef">
          <div class="flex max-w-full flex-col gap-y-1 px-3">
            <div v-for="pkg in environment.packages" :key="pkg.name" class="flex flex-row justify-between">
              <span class="max-w-full truncate whitespace-nowrap text-gray-900 hover:min-w-fit">{{ pkg.name }}</span>
              <span class="flex-shrink-0 text-right font-light text-gray-500">{{ pkg.version }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
