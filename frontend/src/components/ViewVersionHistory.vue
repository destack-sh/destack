<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import type { Project } from "@/gql/graphql";
import { BookmarkIcon } from "@heroicons/vue/24/solid";
const props = defineProps<{ project: Project }>();

const { getTimeFromNow, getTimeFromNowString } = useTimeFromNow();
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
      <ul role="list" class="m-4 -mb-8">
        <li v-for="(version, versionIdx) in project.versions" :key="version.id">
          <div class="relative pb-8">
            <span
              v-if="versionIdx !== project.versions.length - 1"
              class="absolute top-4 left-4 -ml-px h-full w-0.5 bg-gray-200"
              aria-hidden="true"
            />
            <div class="relative flex space-x-3">
              <div>
                <span
                  :class="[
                    'text-orange-500',
                    'ring-12 flex h-8 w-8 items-center justify-center rounded-full ring-white',
                  ]"
                >
                  <BookmarkIcon class="h-5 w-5 text-gray-500" aria-hidden="true" />
                </span>
              </div>
              <div class="flex min-w-0 flex-1 justify-between space-x-4 pt-1.5">
                <div>
                  <p class="text-sm text-gray-900">{{ version.name || "Unnamed" }}</p>
                  <p class="text-sm text-gray-500">(autosave)</p>
                </div>
                <div class="whitespace-nowrap text-right text-sm text-gray-500">
                  <time :datetime="getTimeFromNowString(version.committedAt || version.createdAt)">
                    {{ getTimeFromNowString(version.committedAt || version.createdAt) }}
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
