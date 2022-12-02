<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import type { Project, ProjectVersionHeaderFragment } from "@/gql/graphql";
import { BookmarkIcon } from "@heroicons/vue/24/outline";
import { DateTime } from "luxon";
import { computed } from "vue";
const props = defineProps<{ project: Project }>();

const { now } = useTimeFromNow();

const commits = computed(() => {
  return props.project.versions.filter((v) => v.committed);
});

function getRelOrAbsTime(commit: ProjectVersionHeaderFragment) {
  const committedAt = DateTime.fromISO(commit.committedAt);
  const timeSinceCommit = now.value.diff(committedAt);
  console.log(timeSinceCommit.as("days"));
  console.log(Math.round(timeSinceCommit.as("minutes")));
  // get relative like 2h or 6d if less than 1 week
  // get absolute if more than 1 week
  if (timeSinceCommit.as("days") < 7) {
    // format as 10m, 2h, 3d
    // round to nearest whole number
    const minutes = Math.round(timeSinceCommit.as("minutes"));
    const hours = Math.round(timeSinceCommit.as("hours"));
    const days = Math.round(timeSinceCommit.as("days"));
    if (hours < 1) {
      return `${minutes}m`;
    } else if (days < 1) {
      return `${hours}h`;
    } else {
      return `${days}d`;
    }
  } else {
    // format as Nov 4, 2021
    return committedAt.toLocaleString(DateTime.DATE_MED);
  }
}

const commitTimes = computed(() => {
  return commits.value.map((c) => getRelOrAbsTime(c));
});
</script>
<template>
  <div>
    <!-- View header -->
    <div class="flex flex-row justify-between border-b border-gray-200 px-2 py-4">
      <span class="text-xs font-bold uppercase">Version History</span>
      <!-- TODO @Feature: select explorer get_view (by type, by task tree) -->
    </div>
    <!-- View contents -->
    <div class="flex flex-1 flex-col">
      <!-- View: versions -->
      <ul role="list" class="m-3 -mb-8">
        <li v-for="(version, versionIdx) in commits" :key="version.id">
          <div class="relative pb-4">
            <span
              v-if="versionIdx !== commits.length - 1"
              class="absolute top-4 left-3 -ml-px h-full w-0.5 bg-gray-200"
              aria-hidden="true"
            />
            <div class="relative flex space-x-2">
              <div>
                <span class="ring-6 flex h-6 w-6 items-center justify-center rounded-full bg-gray-50 ring-gray-50">
                  <BookmarkIcon class="h-5 w-5 text-gray-700" aria-hidden="true" />
                </span>
              </div>
              <div class="flex min-w-0 flex-1 justify-between space-x-4 pt-1">
                <div>
                  <p class="text-xs font-bold text-gray-900">{{ version.name || "Unnamed" }}</p>
                  <p class="text-xs text-gray-500">(autosave)</p>
                </div>
                <div class="whitespace-nowrap text-right text-xs text-gray-500">
                  <time :datetime="version.committedAt">
                    {{ getRelOrAbsTime(version) }}
                  </time>
                </div>
              </div>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>
