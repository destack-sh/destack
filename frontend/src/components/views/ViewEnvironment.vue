<script lang="ts" setup>
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { useActiveScroll } from "@/composables/useScroll";
import { graphql } from "@/gql";
import { useAppearance } from "@/state/appearance";
import { useBenchState } from "@/state/bench";
import { useCurrentSessions, WORKER_RESOURCES_BY_PROFILE } from "@/state/session";
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

const resourcesInfo = computed(() => {
  if (workerSet.value == null) return undefined;
  const profile = workerSet.value.profile;
  return WORKER_RESOURCES_BY_PROFILE[profile];
});
</script>
<template>
  <div class="flex flex-col pb-10">
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
    <!-- Compute -->
    <div
      class="mt-0.5 grid grid-cols-3 gap-x-2 gap-y-0.5 px-3 text-sm"
      v-if="environment != null && resourcesInfo != null && workerSet != null"
    >
      <!-- Language -->
      <span class="text-gray-500">Language</span>
      <span class="col-span-2 whitespace-nowrap font-semibold text-gray-900">Python {{ environment.version }}</span>
      <!-- Profile -->
      <span class="text-gray-500">Profile</span>
      <span class="col-span-2 whitespace-nowrap font-semibold text-gray-900">
        {{ resourcesInfo.name }}
        <div class="inline font-light text-gray-500">{{ resourcesInfo.cpu }}vCPU + {{ resourcesInfo.mem }}GB</div>
      </span>
      <!-- Nodes -->
      <span class="text-gray-500">Workers</span>
      <span class="col-span-2 whitespace-nowrap font-semibold text-gray-900">
        {{ workerSet.readyReplicas }}
        <span class="font-normal text-gray-500">of {{ workerSet.desiredReplicas }}</span>
        <div
          class="ml-1.5 inline rounded-sm border border-orange-900 border-opacity-10 bg-orange-100 px-1 py-0.5 text-xs font-semibold uppercase"
        >
          {{ workerSet.status }}
        </div>
      </span>
    </div>
    <!-- Packages -->
    <div class="relative mt-4 w-full flex-1 text-sm" v-if="environment != null">
      <span class="px-3 text-xs font-semibold tracking-wide text-gray-500">Packages</span>
      <div
        class="mt-1 flex max-w-full flex-col gap-y-1 overflow-y-scroll px-3"
        ref="packagesTableRef"
        :style="{
          maxHeight: containerSize.height.value - 180 + 'px',
        }"
      >
        <div v-for="pkg in environment.packages" :key="pkg.name" class="flex flex-row justify-between">
          <span class="max-w-full truncate whitespace-nowrap text-gray-900 hover:min-w-fit">{{ pkg.name }}</span>
          <span class="flex-shrink-0 text-right font-light text-gray-500">{{ pkg.version }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
