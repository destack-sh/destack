<script lang="ts" setup>
import { graphql } from "@/gql";
import { useActions } from "@/state/actions";
import { useBenchState, type EditorGroup } from "@/state/bench";
import { DocumentIcon, MagnifyingGlassIcon, PlusIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const props = defineProps<{ group: EditorGroup }>();

const bench = useBenchState();
const actions = useActions();

const { result: suggestedFiles } = useQuery(
  graphql(/* GraphQL */ `
    query emptyEditorSuggestedFiles($projectVersionId: GlobalID!, $last: Int!) {
      projectVersion(id: $projectVersionId) {
        files(filters: { isVisible: true, isGenerated: false }, last: $last) {
          totalCount
          edges {
            node {
              id
              name
              path
              deletedAt
              directory
            }
          }
        }
      }
    }
  `),
  computed(() => ({
    projectVersionId: bench.currentProjectVersionId,
    last: 8,
  })) as any,
  {
    enabled: computed(() => !!bench.currentProjectVersionId),
  }
);
const files = computed(() =>
  suggestedFiles.value?.projectVersion?.files.edges
    .map((edge) => edge.node)
    .filter((file) => file.deletedAt == null && !file.directory && file.name.trim() !== "")
);
const totalCount = computed(() => suggestedFiles.value?.projectVersion?.files.totalCount);

const createActions = computed(() => [
  {
    label: "View files",
    icon: DocumentIcon,
    action: () => actions.apply("bench.view.openExplorer"),
    enabled: true,
  },
  {
    label: "Search files",
    icon: MagnifyingGlassIcon,
    action: () => actions.apply("bench.view.openSearch"),
    enabled: false,
  },
  {
    label: "Create file",
    icon: PlusIcon,
    action: () => actions.file.create.value.apply(),
    enabled: actions.file.create.value.enabled,
  },
]);

function openFile(file: { id: string; path: string }) {
  bench.openFile(file, { group: props.group, create: true, focus: true });
}
</script>
<template>
  <div class="flex h-full w-full flex-col justify-center pb-32">
    <div
      class="flex w-full max-w-xl flex-row justify-center gap-4 self-center px-4 pb-16 pt-8 text-left transition-colors duration-75 sm:px-6 lg:px-8"
    >
      <!-- Actions -->
      <div class="flex-1 p-1">
        <h3 class="px-1 text-lg font-bold text-gray-900">Tools</h3>
        <!-- Create actions -->
        <div class="my-1 flex flex-col gap-1.5">
          <button
            v-for="action in createActions"
            :key="action.label"
            class="group flex items-center rounded-sm bg-transparent px-1 py-0.5"
            :class="[
              action.enabled
                ? 'hover:bg-orange-100 focus:bg-orange-100'
                : 'opacity-50 hover:cursor-not-allowed hover:bg-gray-50 focus:bg-gray-50',
            ]"
            @click="action.action"
            :disabled="!action.enabled"
          >
            <component :is="action.icon" class="h-5 w-5 text-gray-500" />
            <span class="ml-1 text-gray-700 group-hover:text-gray-900">{{ action.label }}</span>
          </button>
        </div>
      </div>
      <!-- Recent files -->
      <div class="flex-1 p-1">
        <h3 class="px-1 text-lg font-bold text-gray-900">Files</h3>
        <!-- Files -->
        <div class="my-1 flex flex-col gap-1.5" v-if="(totalCount ?? 0) > 0">
          <button
            v-for="file in files"
            :key="file.id"
            class="group flex items-center rounded-sm bg-transparent hover:bg-orange-100 focus:bg-orange-100"
            @click="openFile(file)"
          >
            <span class="px-1 py-0.5 text-gray-700 group-hover:text-gray-900">{{ file.name }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
