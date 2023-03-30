<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { formatDiffSeconds, humanizeNumber, useNow, useTimeFromNow } from "@/composables/useNow";
import { JobStatus, JobType } from "@/gql/graphql";
import { useJobs } from "@/state/jobs";
import { Popover, PopoverButton, PopoverPanel } from "@headlessui/vue";
import { EllipsisHorizontalIcon } from "@heroicons/vue/24/outline";
import { DateTime } from "luxon";
import { computed, toRef, ref } from "vue";

const props = defineProps<{
  projectId: string;
  projectVersionId: string;
}>();

const now = useNow(100);
const { getTimeFromNowString } = useTimeFromNow();
const { jobs: liveJobs, totalCount } = useJobs(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    statusIn: ref(null),
  },
  { live: true, first: 20 }
);
// recent live jobs does not include potentially older still running jobs
// so we load them in a separate query on initial load (not needed afterwards since it's live)
const { jobs: initialActiveJobs } = useJobs(
  {
    projectId: toRef(props, "projectId"),
    projectVersionId: toRef(props, "projectVersionId"),
    statusIn: ref([JobStatus.Running, JobStatus.Cancelling]),
  },
  { live: false, first: 10 }
);
const allJobs = computed(() => [...(initialActiveJobs.value ?? []), ...(liveJobs.value ?? [])]);
const activeJobs = computed(() =>
  allJobs.value
    ?.filter(
      (job) =>
        job.status == JobStatus.Running &&
        (job.startedAt == null || now.value.diff(DateTime.fromISO(job.startedAt)).as("milliseconds") > 500)
    )
    .sort((a, b) => (a.createdAt < b.createdAt ? -1 : 1))
);

const JOB_VERBS_INF = {
  [JobType.Interp]: "Interpreting",
  [JobType.Lint]: "Analyzing",
  [JobType.Build]: "Building",
  [JobType.Generate]: "Generating",
  [JobType.Evaluate]: "Evaluating",
};
const JOB_VERBS = {
  [JobType.Interp]: "Interpret",
  [JobType.Lint]: "Analyze",
  [JobType.Build]: "Build",
  [JobType.Generate]: "Generate",
  [JobType.Evaluate]: "Evaluate",
};
</script>
<template>
  <Popover v-slot="{ open }" class="relative">
    <!-- Current active jobs preview (truncated) -->
    <PopoverButton
      ref="jobsButtonRef"
      class="rounded-sm p-1 text-sm focus:outline-none"
      :class="{
        'hover:bg-orange-100': true,
        'bg-orange-100': open,
      }"
    >
      <!-- Active jobs (truncated) -->
      <div class="flex items-center gap-2" v-if="activeJobs.length > 0">
        <!-- Spinner -->
        <svg viewBox="0 0 10 10" class="h-1 w-1 animate-spin text-gray-400">
          <rect width="10" height="10" rx="2" ry="2" fill="currentColor" />
        </svg>
        <!-- Truncated jobs -->
        <span v-for="job in activeJobs.slice(0, 3)" :key="job.type" class="text-gray-500">
          {{ JOB_VERBS_INF[job.type] }}
        </span>
        <span v-if="activeJobs.length > 3" class="whitespace-nowrap text-gray-500">+{{ activeJobs.length - 2 }}</span>
      </div>
      <!-- No active jobs (ellipsis) -->
      <div v-else>
        <EllipsisHorizontalIcon class="h-5 w-5 text-gray-400" />
      </div>
    </PopoverButton>
    <!-- All recent jobs -->
    <FadeTransition>
      <PopoverPanel
        class="absolute top-10 left-0 z-10 mt-0 flex h-60 w-80 flex-col gap-2 overflow-y-scroll rounded-sm bg-white px-4 pb-4 text-sm shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Header -->
        <div class="sticky top-0 flex flex-row justify-between bg-white pt-2">
          <h2 class="flex flex-row items-baseline gap-1">
            <span class="font-bold text-gray-900">Jobs</span>
            <!-- Count -->
            <span class="rounded-2xl bg-gray-200 px-1 text-gray-900">{{ humanizeNumber(totalCount) }}</span>
          </h2>
          <span class="text-gray-700" v-if="activeJobs.length > 0"> {{ activeJobs.length }} active </span>
        </div>
        <!-- Recent jobs -->
        <ul class="mt-2 flex w-full flex-col gap-2">
          <li class="flex w-full flex-row justify-between" v-for="job in allJobs" :key="job.id">
            <!-- Job info -->
            <span>
              {{ JOB_VERBS[job.type] }}
              <span class="pl-2 text-gray-500">{{ getTimeFromNowString(job.updatedAt) }}</span>
            </span>
            <!-- Status & duration -->
            <span class="flex flex-row items-center gap-1 text-gray-500">
              <!-- Duration -->
              <span class="text-gray-500" v-if="job.terminatedAt != null">
                {{ formatDiffSeconds(job.startedAt, job.terminatedAt) }}
              </span>
              <span class="text-gray-500" v-else-if="job.startedAt != null">
                {{ formatDiffSeconds(job.startedAt, now) }}
              </span>
              <!-- Status -->
              <svg
                viewBox="0 0 10 10"
                class="h-2 w-2"
                :class="{
                  'text-green-600': job.status == JobStatus.Completed,
                  'text-red-600': job.status == JobStatus.Failed || job.status == JobStatus.Cancelled,
                  'text-gray-500':
                    job.status == JobStatus.Queued ||
                    job.status == JobStatus.Running ||
                    job.status == JobStatus.Cancelling,
                  'animate-spin': job.status == JobStatus.Running,
                }"
              >
                <rect width="10" height="10" rx="2" ry="2" fill="currentColor" />
              </svg>
            </span>
          </li>
          <!-- Pagination (soonish) -->
          <li v-if="allJobs.length < totalCount" class="text-center text-xs text-gray-700">
            <span>{{ totalCount - allJobs.length }} more</span>
          </li>
        </ul>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
