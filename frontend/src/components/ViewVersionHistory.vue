<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { ProjectHeaderType, ProjectVersionHeaderType } from "@/utils/fragments";
import { BookmarkIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const props = defineProps<{ project: FragmentType<typeof ProjectHeaderType> }>();
const project = computed(() => useFragment(ProjectHeaderType, props.project));
const { getTimeFromNowString } = useTimeFromNow();

const { result: versionsQuery, loading } = useQuery(
  graphql(/* GraphQL */ `
    query projectVersions($projectId: GlobalID!) {
      project(id: $projectId) {
        id
        versions {
          ...ProjectVersionHeader
        }
      }
    }
  `),
  () => ({
    projectId: project.value.id,
  })
);
const versions = computed(
  () => versionsQuery.value?.project?.versions.map((x) => useFragment(ProjectVersionHeaderType, x)) || []
);
</script>
<template>
  <div>
    <!-- View header -->
    <div class="flex flex-row justify-between border-b border-gray-200 px-3 py-4">
      <span class="text-xs font-bold uppercase">Version History</span>
    </div>
    <!-- View contents -->
    <div class="flex flex-1 flex-col" v-if="!loading">
      <!-- View: versions -->
      <ul role="list" class="m-3 -mb-8">
        <li v-for="(version, versionIdx) in versions" :key="version.id">
          <div class="relative pb-4">
            <span
              v-if="versionIdx !== versions.length - 1"
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
                  <p class="text-xs text-gray-500">{{ version.description || "(autosave)" }}</p>
                </div>
                <div class="whitespace-nowrap text-right text-xs text-gray-500" v-if="version.committed">
                  <time :datetime="version.committedAt">
                    {{ getTimeFromNowString(version.committedAt) }}
                  </time>
                </div>
                <div v-else class="whitespace-nowrap text-right text-xs text-orange-600">current</div>
              </div>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>
