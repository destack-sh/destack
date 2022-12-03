<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import type { Project } from "@/gql/graphql";
import { BookmarkIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";
const props = defineProps<{ project: Project }>();

const { getTimeFromNowString } = useTimeFromNow();

const commits = computed(() => {
  return props.project.versions.filter((v) => v.committed);
});
</script>
<template>
  <div>
    <!-- View header -->
    <div class="flex flex-row justify-between border-b border-gray-200 px-3 py-4">
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
                  <p class="text-xs text-gray-500">{{ version.description || "(autosave)" }}</p>
                </div>
                <div class="whitespace-nowrap text-right text-xs text-gray-500">
                  <time :datetime="version.committedAt">
                    {{ getTimeFromNowString(version.committedAt) }}
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
