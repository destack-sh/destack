<script lang="ts" setup>
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { useActions } from "@/state/actions";
import { ProjectHeaderType, ProjectVersionHeaderType } from "@/state/fragments";
import { useOperationsStore } from "@/state/operations";
import { BookmarkIcon, PlusIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, type Component, type Ref } from "vue";

const props = defineProps<{
  project: FragmentType<typeof ProjectHeaderType>;
  currentVersion: FragmentType<typeof ProjectVersionHeaderType>;
}>();
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
const commits = computed(() => versions.value.filter((x) => x.committed));

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
    enabled: computed(() => !opsStore.hasInflightLike("version.commit")),
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
          <span class="pl-0.5 text-xs text-gray-700">{{ action.label }}</span>
        </button>
      </span>
    </div>
    <!-- View contents -->
    <div class="relative flex-1 flex-col" v-if="!loading">
      <!-- View: versions -->
      <ul role="absolute left-0 top-0 h-full w-full overflow-y-auto list" class="m-3 -mb-8">
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
                  <p class="text-xs text-gray-500">{{ version.description }}</p>
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
