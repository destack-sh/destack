<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import ViewSection from "@/components/views/ViewSection.vue";
import ViewSectionGroup from "@/components/views/ViewSectionGroup.vue";
import { humanizeNumber, useTimeFromNow } from "@/composables/useNow";
import { useActiveScroll } from "@/composables/useScroll";
import { graphql } from "@/gql";
import { WorkerSetStatus } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import { useBenchState } from "@/state/bench";
import { humanizeBytes } from "@/state/blob";
import {
  useCurrentSessions,
  WORKER_RESOURCES_BY_PROFILE,
  WORKER_STATUS_COLOR,
  WORKER_STATUS_ICON_SOLID,
  WORKER_STATUS_TITLE,
} from "@/state/session";
import {
  ClockIcon,
  DocumentIcon,
  BoltIcon,
  ChevronDownIcon,
  CircleStackIcon,
  PowerIcon,
} from "@heroicons/vue/24/solid";
import { CodeBracketSquareIcon, ServerStackIcon } from "@heroicons/vue/24/solid";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, toRef, type Ref, onBeforeUnmount } from "vue";

const props = defineProps<{
  active: boolean;
  focused: boolean;
  containerSize: { width: Ref<number>; height: Ref<number> };
}>();
const emit = defineEmits<{ (e: "show"): void; (e: "blur"): void }>();

const bench = useBenchState();
const session = useCurrentSessions();
const workerSet = computed(() => session.workerSet.value);
const appearance = useAppearance();
const now = useTimeFromNow(100);

const packagesTableRef = ref<HTMLDivElement | null>(null);
useActiveScroll(packagesTableRef);

const {
  result: environmentQuery,
  loading,
  refetch,
} = useQuery(
  graphql(/* GraphQL */ `
    query environment($projectId: GlobalID!) {
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
      project(id: $projectId) {
        id
        usage {
          recordsActive
          objectsBytesTotal
          cacheBytesTotal
        }
      }
    }
  `),
  {
    projectId: toRef(bench, "projectId"),
  },
  {
    enabled: computed(() => props.active) as any,
    fetchPolicy: "cache-and-network",
  }
);
const environment = computed(() =>
  environmentQuery.value?.environment.__typename == "Environment" ? environmentQuery.value?.environment : undefined
);
const projectUsage = computed(() => environmentQuery.value?.project?.usage);
const resourcesInfo = computed(() => {
  if (workerSet.value == null) return undefined;
  const profile = workerSet.value.profile;
  return WORKER_RESOURCES_BY_PROFILE[profile];
});

// hardcoded quotas for now
const maxRecordsActive = 50000;
const maxObjectsBytesTotal = 50 * 1024 * 1024 * 1024; // 50GB
const maxCacheBytesTotal = 50 * 1024 * 1024; // 50MB

// auto reload environment every minute if active
const interval = setInterval(() => {
  if (props.active) {
    refetch();
  }
}, 60 * 1000);
onBeforeUnmount(() => clearInterval(interval));

function getNicePercentage(value: number, total: number): string {
  // format percentage with up to 2 significant digits
  // so 0.00073 is 0.0%, 0.0073 is 0.7%, 0.07326 is 7.3%, 0.7348 is 73%
  const percentage = (value / total) * 100;
  if (percentage < 0.1) {
    return "0";
  } else if (percentage < 1) {
    return percentage.toFixed(1);
  } else {
    return percentage.toFixed(0);
  }
}

const effectiveWorkerStatus = computed(() => {
  if (session.waking.value || session.restarting.value) {
    return WorkerSetStatus.Pending;
  } else {
    return workerSet.value?.status ?? WorkerSetStatus.Unknown;
  }
});
</script>
<template>
  <ViewSectionGroup :focused="props.focused" @show="emit('show')" @blur="emit('blur')">
    <!-- Environment -->
    <ViewSection
      title="Enviroment"
      :index="0"
      :loading="environment == null || resourcesInfo == null || workerSet == null || projectUsage == null"
    >
      <div
        class="flex flex-col gap-y-1.5 pl-3 pr-5 text-sm"
        v-if="environment != null && resourcesInfo != null && workerSet != null && projectUsage != null"
      >
        <!-- Status/actions -->
        <div class="flex flex-row items-center justify-between">
          <!-- Status -->
          <div class="flex flex-row items-center">
            <component
              :is="WORKER_STATUS_ICON_SOLID[effectiveWorkerStatus]"
              class="mr-1.5 h-4 w-4"
              :class="[
                WORKER_STATUS_COLOR[effectiveWorkerStatus].includes('gray')
                  ? 'text-gray-400'
                  : WORKER_STATUS_COLOR[effectiveWorkerStatus],
                WORKER_STATUS_ICON_SOLID[effectiveWorkerStatus] == BusySpinnerIcon ? 'animate-spin' : '',
              ]"
            />
            <span
              class="mr-1.5 whitespace-nowrap font-semibold"
              :class="
                WORKER_STATUS_COLOR[effectiveWorkerStatus].includes('gray')
                  ? 'text-gray-900'
                  : WORKER_STATUS_COLOR[effectiveWorkerStatus]
              "
            >
              {{ WORKER_STATUS_TITLE[effectiveWorkerStatus] }}
            </span>
          </div>
          <!-- Idle / actions -->
          <div class="flex flex-row items-center whitespace-nowrap">
            <button
              v-if="workerSet.status == WorkerSetStatus.Healthy || workerSet.status == WorkerSetStatus.Unhealthy"
              class="flex flex-row items-center rounded-sm px-0.5 text-gray-500"
              @click="session.restartWorkerSet()"
              :class="[
                session.restarting.value ? 'animate-pulse' : ' hover:bg-orange-100 hover:text-gray-700',
                !bench.canUse ? 'cursor-not-allowed opacity-50' : '',
              ]"
              :disabled="(session.restarting.value && !bench.debug) || !bench.canUse"
            >
              <PowerIcon class="mr-1 h-4 w-4" />
              {{ session.restarting.value ? "Restarting..." : "Restart" }}
            </button>
            <button
              v-else-if="workerSet.status == WorkerSetStatus.Sleeping"
              class="flex flex-row items-center rounded-sm px-0.5 text-gray-500"
              @click="session.wakeWorkerSet()"
              :class="[
                session.waking.value ? 'animate-pulse' : 'hover:bg-orange-100 hover:text-gray-700',
                !bench.canUse ? 'cursor-not-allowed opacity-50' : '',
              ]"
              :disabled="(session.waking.value && !bench.debug) || !bench.canUse"
            >
              <PowerIcon class="mr-1 h-4 w-4" />
              {{ session.waking.value ? "Waking..." : "Wake" }}
            </button>
          </div>
        </div>
        <!-- Profile -->
        <div class="flex flex-row items-center justify-between">
          <div class="flex flex-row items-center">
            <ServerStackIcon class="mr-1.5 h-4 w-4 text-gray-400" />
            <span class="whitespace-nowrap font-semibold text-gray-900">
              <span>{{ workerSet.desiredReplicas }}x {{ resourcesInfo.name }}</span>
            </span>
            <ChevronDownIcon class="ml-1 h-4 w-4 text-gray-400" />
          </div>
          <div class="ml-1.5 inline font-normal text-gray-500">
            {{ resourcesInfo.cpu }}vCPU + {{ resourcesInfo.mem }}GB
          </div>
        </div>
        <!-- Records / files -->
        <div class="flex flex-row items-center justify-between">
          <div class="flex flex-row items-center">
            <CircleStackIcon class="mr-1.5 h-4 w-4 text-gray-400" />
            <span class="whitespace-nowrap font-semibold text-gray-900">
              <span>{{ humanizeNumber(maxRecordsActive) }}</span>
              <span class="ml-1 font-normal text-gray-500"
                >{{ getNicePercentage(projectUsage.recordsActive, maxRecordsActive) }}%</span
              >
            </span>
          </div>
          <div class="flex flex-row items-center">
            <DocumentIcon class="mr-1.5 h-4 w-4 text-gray-400" />
            <span class="whitespace-nowrap font-semibold text-gray-900">
              <span>{{ humanizeBytes(maxObjectsBytesTotal) }}</span>
              <span class="ml-1 font-normal text-gray-500"
                >{{ getNicePercentage(projectUsage.objectsBytesTotal, maxObjectsBytesTotal) }}%</span
              >
            </span>
          </div>
        </div>
        <!-- Retention / cache -->
        <!-- (too much info for now) -->
        <!-- <div class="flex flex-row items-center justify-between">
        <div class="flex flex-row items-center">
          <BoltIcon class="mr-1.5 h-4 w-4 text-gray-400" />
          <span class="whitespace-nowrap font-semibold text-gray-900">
            <span>{{ humanizeBytes(maxCacheBytesTotal) }}</span>
            <span class="ml-1 font-normal text-gray-500"
              >{{ getNicePercentage(projectUsage.cacheBytesTotal, maxCacheBytesTotal) }}%</span
            >
          </span>
        </div>
        <div class="flex flex-row items-center">
          <ClockIcon class="mr-1.5 h-4 w-4 text-gray-400" />
          <span class="whitespace-nowrap text-gray-500">
            <span>15d</span>
          </span>
        </div>
      </div> -->
        <!-- Runtime -->
        <div class="flex max-w-full flex-row items-center justify-between">
          <span class="flex flex-shrink-0 flex-row items-center">
            <CodeBracketSquareIcon class="mr-1.5 h-4 w-4 text-gray-400" />
            <span class="whitespace-nowrap font-semibold text-gray-900">Python {{ environment.version }}</span>
          </span>
          <span class="ml-2 max-w-full truncate whitespace-nowrap text-gray-500">{{ environment.platform }}</span>
        </div>
        <!-- TODO @UX: view actual nodes, latency and resource usage here -->
        <!-- maybe also show object storage usage, total records, etc.? -->
      </div>
    </ViewSection>
    <!-- Packages -->
    <ViewSection title="Packages" :index="1" :loading="environment == null">
      <div class="flex max-w-full flex-col gap-y-1 px-3 text-sm">
        <div v-for="pkg in environment?.packages" :key="pkg.name" class="flex flex-row justify-between">
          <span class="max-w-full truncate whitespace-nowrap text-gray-900 hover:min-w-fit">{{ pkg.name }}</span>
          <span class="flex-shrink-0 text-right text-gray-500">{{ pkg.version }}</span>
        </div>
      </div>
    </ViewSection>
  </ViewSectionGroup>
</template>
