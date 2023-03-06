<script lang="ts" setup>
import CommitPopover from "@/components/basic/CommitPopover.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { provideGlobalAction } from "@/state/actions";
import { useEditorState, type ProjectHeader } from "@/state/editor";
import { ProjectVersionHeaderType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { parseSemVer } from "@/utils/semver";
import { PopoverButton } from "@headlessui/vue";
import { BookmarkIcon, PencilIcon, TagIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref, type Component, type Ref } from "vue";
import { useRouter } from "vue-router";

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
        head {
          ...ProjectVersionHeader
        }
        versions {
          totalCount
          edges {
            node {
              ...ProjectVersionHeader
            }
          }
        }
      }
    }
  `),
  () => ({
    projectId: props.project.id,
  })
);
const versions = computed(
  () => versionsQuery.value?.project?.versions.edges.map((x) => useFragment(ProjectVersionHeaderType, x.node)) || []
);
const head = computed(() => useFragment(ProjectVersionHeaderType, versionsQuery.value?.project?.head));
const isAtHead = computed(() => editor.currentProjectVersionId == head.value?.id);

type Action = {
  icon: Component;
  label: string;
  enabled: Ref<boolean>;
  action: () => void;
};

const router = useRouter();
const notifications = useNotifications();
const operations = useOperations();
const lastSemVerTag = computed(() => {
  // note that this may fail when we paginate versions (and there are many untagged versions)
  if (versions.value == null) return null;
  const tag = versions.value.find((x) => x.tag != null && parseSemVer(x.tag) != null)?.tag;
  if (tag != null) {
    return parseSemVer(tag);
  }
  return tag;
});
const snapshotButtonRef: Ref<HTMLButtonElement | null> = ref(null);

const commit = provideGlobalAction({
  id: "version.commit",
  label: "Snapshot...",
  shortcuts: ["ctrl+k"],
  enabled: computed(
    () =>
      props.project.canWrite &&
      editor.currentProjectVersionId != null &&
      !operations.state.hasInflightLike({ types: ["version.commit"] })
  ),
  apply: () => {
    // just open snapshot create menu
    snapshotButtonRef.value?.click();
  },
});

async function doCommit(versionId: string, name?: string, tag?: string) {
  const ret = await operations.version.commit(versionId, name, tag);
  if (ret?.data?.commit.__typename == "CommitPayload") {
    notifications.show({
      type: "commit.success",
      kind: "success",
      message: `Snapshot created`,
      description: `Version is safe in the archives.`,
    });
  }
}

// instant commit (aka manual autosave)
provideGlobalAction({
  id: "version.commitInstant",
  label: "Snapshot (auto)",
  shortcuts: ["ctrl+shift+k"],
  enabled: computed(
    () =>
      props.project.canWrite &&
      editor.currentProjectVersionId != null &&
      !operations.state.hasInflightLike({ types: ["version.commit"] })
  ),
  apply: async () => {
    await doCommit(editor.currentProjectVersionId as string, null, null);
  },
});

const restore = provideGlobalAction({
  id: "version.restore",
  label: "Restore",
  shortcuts: [],
  enabled: computed(() => props.project.canWrite && !isAtHead.value),
  apply: async () => {
    operations.state.reset();
    const ret = await operations.version.restore(editor.currentProjectVersionId as string);
    if (ret?.data?.restore.__typename == "CommitPayload") {
      notifications.show({
        type: "restore.success",
        kind: "success",
        message: `Snapshot restored`,
        description: `The previous working state was autosaved.`,
      });
    }
    router.replace({ hash: router.currentRoute.value.hash }); // clear version query param
  },
});
</script>
<template>
  <!-- Container (views should be a single root element) -->
  <div>
    <!-- View header -->
    <div class="flex h-[31px] flex-row items-center justify-between border-b border-gray-200 px-3 py-2">
      <span class="text-xs font-bold uppercase">History</span>
      <!-- Version controls -->
      <CommitPopover
        v-if="head != null"
        :version="head"
        :projectId="props.project.id"
        :prev-sem-ver-tag="lastSemVerTag ?? undefined"
        @commit="(id, name, tag) => doCommit(id, name, tag)"
      >
        <template v-slot:button="{ open }">
          <PopoverButton
            ref="snapshotButtonRef"
            :disabled="!commit.enabled"
            class="inline-flex flex-row rounded-sm p-0.5 outline-none"
            :class="{
              'text-gray-300': !commit.enabled,
              'text-gray-400 hover:bg-orange-100 hover:text-gray-700': commit.enabled,
              'bg-orange-100': open,
            }"
          >
            <BookmarkIcon class="h-4 w-4" />
          </PopoverButton>
        </template>
      </CommitPopover>
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
            <!-- Ideally we would have individual edits/revisions here as well -->
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
                  <p class="flex flex-row items-start gap-0.5 text-xs font-bold">
                    <!-- Version link -->
                    <router-link
                      :to="{
                        query: versionIdx == 0 ? undefined : { version: version.id },
                        hash: router.currentRoute.value.hash,
                      }"
                      class="hover:underline"
                      :class="isCurrent(version) ? 'text-orange-600' : 'text-gray-900'"
                    >
                      {{ version.name || (versionIdx == 0 ? "(Working)" : "Autosave") }}
                    </router-link>
                    <!-- Edit button -->
                    <button
                      v-if="project.canWrite"
                      class="invisible p-0.5 text-gray-300 hover:bg-orange-50 hover:text-gray-700 group-hover:visible"
                    >
                      <PencilIcon class="h-3 w-3" />
                    </button>
                  </p>
                  <!-- Tag button -->
                  <button
                    class="flex w-fit flex-row gap-0.5 rounded-sm p-0.5 text-xs"
                    :class="[
                      version.tag == null ? 'text-gray-300 ' : 'text-gray-700',
                      project.canWrite ? 'hover:bg-orange-50 hover:text-gray-700' : '',
                    ]"
                    :disabled="!project.canWrite"
                  >
                    <TagIcon class="h-4 w-4" />
                    <span :class="version.tag == null ? 'invisible group-hover:visible' : ''">
                      {{ version.tag || "Tag" }}
                    </span>
                  </button>
                </div>
                <!-- Time -->
                <div class="whitespace-nowrap text-right text-xs">
                  <svg
                    v-if="versionIdx == 0"
                    viewBox="0 0 10 10"
                    class="mr-1 h-1 w-1"
                    :class="isCurrent(version) ? 'text-orange-600' : 'text-gray-700'"
                  >
                    <rect width="10" height="10" rx="1" ry="1" fill="currentColor" />
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
