<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { useActions } from "@/state/actions";
import { useEditorState, type ProjectHeader } from "@/state/editor";
import { ProjectVersionHeaderType } from "@/state/fragments";
import { useOperationsStore } from "@/state/operations";
import { BookmarkIcon, PencilIcon, PlusIcon, TagIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Component, type Ref } from "vue";

const props = defineProps<{
  project: ProjectHeader;
  currentVersion: FragmentType<typeof ProjectVersionHeaderType>;
}>();
const { getTimeFromNowString } = useTimeFromNow();

const editor = useEditorState();
function isCurrent(version: { id: string }): boolean {
  return editor.currentProjectVersionId == version.id;
}

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
    projectId: props.project.id,
  })
);
const versions = computed(
  () => versionsQuery.value?.project?.versions.map((x) => useFragment(ProjectVersionHeaderType, x)) || []
);

type Action = {
  icon: Component;
  label: string;
  enabled: Ref<boolean>;
  action: () => void;
};

const actions = useActions();
const opsStore = useOperationsStore();
const globalActions: Action[] = [
  {
    icon: PlusIcon,
    label: "Commit",
    enabled: computed(() => !opsStore.hasInflightLike({ types: ["version.commit"] })),
    action: () => actions.version.commit.value.apply(),
  },
];
</script>
<template>
  <div>
    <!-- View header -->
    <div class="flex h-[31px] flex-row items-center justify-between border-b border-gray-200 px-3 py-2">
      <span class="text-xs font-bold uppercase">History</span>
      <!-- Version controls -->
      <span class="inline-flex flex-row gap-1">
        <button
          v-for="action in globalActions"
          :key="action.label"
          :disabled="!action.enabled.value"
          class="inline-flex flex-row rounded-sm p-0.5"
          :class="{
            'text-gray-400 hover:bg-gray-50': !action.enabled.value,
            'hover:bg-gray-100 hover:text-gray-700': action.enabled.value,
          }"
          @click.prevent="action.action"
        >
          <component :is="action.icon" class="h-4 w-4 text-gray-600" />
          <span class="sr-only pl-0.5 text-xs text-gray-700">{{ action.label }}</span>
        </button>
      </span>
    </div>
    <!-- View versions -->
    <div class="relative flex-1 flex-col" v-if="!loading">
      <!-- Versions -->
      <ul role="absolute left-0 top-0 h-full w-full overflow-y-auto list" class="-mb-8 py-2">
        <li v-for="(version, versionIdx) in versions" :key="version.id">
          <div class="group relative mb-2 pb-1 hover:bg-orange-50">
            <!-- Vertical line connecting versions -->
            <div class="mx-3">
              <span
                v-if="versionIdx !== versions.length - 1"
                class="absolute top-4 left-3 ml-[11px] h-full w-0.5 bg-gray-200"
                aria-hidden="true"
              />
            </div>
            <div class="relative flex space-x-2 px-3 py-0.5">
              <!-- Version icon -->
              <span
                class="ring-6 flex h-6 w-6 items-center justify-center rounded-full bg-gray-50 ring-gray-50 group-hover:bg-orange-50"
              >
                <BookmarkIcon
                  class="h-5 w-5"
                  :class="isCurrent(version) ? 'text-orange-600' : 'text-gray-700'"
                  aria-hidden="true"
                />
              </span>
              <!-- Version info -->
              <div class="flex min-w-0 flex-1 items-baseline justify-between space-x-4">
                <!-- Name, tag, description -->
                <div class="pt-0.5">
                  <!-- Past version -->
                  <p v-if="versionIdx > 0" class="flex flex-row items-start gap-0.5 text-xs font-bold">
                    <router-link :to="`/`" class="text-gray-900 hover:underline">{{
                      version.name || "Autosave"
                    }}</router-link>
                    <button
                      class="invisible p-0.5 text-gray-300 hover:bg-orange-50 hover:text-gray-700 group-hover:visible"
                    >
                      <PencilIcon class="h-3 w-3" />
                    </button>
                  </p>
                  <!-- Head version -->
                  <p v-else class="text-xs font-bold" :class="isCurrent(version) ? 'text-orange-600' : 'text-gray-700'">
                    (Working)
                    <!-- Save button here? -->
                  </p>
                  <button
                    class="flex w-fit flex-row gap-0.5 rounded-sm p-0.5 text-xs"
                    :class="
                      version.tag == null ? 'text-gray-300 hover:bg-orange-50 hover:text-gray-700' : 'text-gray-700'
                    "
                  >
                    <TagIcon class="h-4 w-4" />
                    <span>{{ version.tag || "Tag" }}</span>
                  </button>
                </div>
                <!-- Time -->
                <div class="whitespace-nowrap text-right text-xs">
                  <svg v-if="versionIdx == 0" viewBox="0 0 100 100" class="mb-0.5 h-1 w-1 text-orange-600">
                    <circle cx="50" cy="50" r="40" fill="currentColor" />
                  </svg>
                  <time v-else class="text-gray-500" :datetime="version.committedAt">
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
