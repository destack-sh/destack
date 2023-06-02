<script lang="ts" setup>
import CommitPopover from "@/components/bench/CommitPopover.vue";
import BookmarkDashedIcon from "@/components/basic/BookmarkDashedIcon.vue";
import { useNavigationGrid } from "@/composables/useGrid";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { provideGlobalAction } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useBenchState, type ProjectHeader } from "@/state/bench";
import { ProjectVersionHeaderType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { parseSemVer } from "@/utils/semver";
import { PopoverButton } from "@headlessui/vue";
import { BookmarkIcon, PencilIcon, TagIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useFocusWithin } from "@vueuse/core";
import { computed, nextTick, ref, toRef, watch, type Ref } from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{
  project: ProjectHeader;
  currentVersion?: FragmentType<typeof ProjectVersionHeaderType>;
  focused: boolean;
}>();
const emit = defineEmits<{ (e: "show"): void; (e: "blur"): void }>();

const { getTimeFromNowString } = useTimeFromNow();

const bench = useBenchState();
const appearance = useAppearance();

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
const versionsCount = computed(() => versionsQuery.value?.project?.versions.totalCount);
const head = computed(() => useFragment(ProjectVersionHeaderType, versionsQuery.value?.project?.head));
const isAtHead = computed(() => bench.currentProjectVersionId == head.value?.id);

function isCurrent(version: { id: string }): boolean {
  return bench.currentProjectVersionId == version.id;
}

function isHead(version: { id: string }): boolean {
  return head.value?.id == version.id;
}

const router = useRouter();
const notifications = useNotifications();
const ops = useOperations();
const lastSemVerTag = computed(() => {
  // note that this may fail when we paginate versions (and there are many untagged versions)
  if (versions.value == null) return null;
  const tag = versions.value.find((x) => x.tag != null && parseSemVer(x.tag) != null)?.tag;
  if (tag != null) {
    return parseSemVer(tag);
  }
  return tag;
});

const snapshotButtonRef: Ref<InstanceType<typeof PopoverButton> | null> = ref(null);

const commit = provideGlobalAction({
  id: "version.commit",
  label: "Snapshot...",
  shortcuts: ["ctrl+k", "meta+k"],
  enabled: computed(
    () =>
      props.project.canWrite &&
      bench.currentProjectVersionId != null &&
      !ops.state.hasInflightLike({ types: ["version.commit"] })
  ),
  apply: () => {
    // just open snapshot history view (view must be visible for popover to render)
    emit("show");
    snapshotButtonRef.value?.$el.click();
  },
});

async function doCommit(c: {
  projectVersionId: string;
  name?: string;
  tag?: string;
  description?: string;
  autoDeploy?: boolean;
}) {
  const ret = await ops.version.commit(c);
  if (ret?.data?.commit.__typename == "CommitPayload") {
    notifications.show({
      type: "commit.success",
      kind: "success",
      message: `Snapshot created`,
      description: `${c.name ?? "Snapshot"} is safe in the archives.`,
    });
  }
}

// instant commit (aka manual autosave)
provideGlobalAction({
  id: "version.commitInstant",
  label: "Snapshot (auto)",
  shortcuts: ["ctrl+shift+k", "meta+shift+k"],
  enabled: computed(
    () =>
      props.project.canWrite &&
      bench.currentProjectVersionId != null &&
      !ops.state.hasInflightLike({ types: ["version.commit"] })
  ),
  apply: async () => {
    await doCommit({ projectVersionId: bench.currentProjectVersionId as string });
  },
});

const restore = provideGlobalAction({
  id: "version.restore",
  label: "Restore",
  shortcuts: [],
  enabled: computed(() => props.project.canWrite && !isAtHead.value),
  apply: async () => {
    ops.state.reset();
    const ret = await ops.version.restore(bench.currentProjectVersionId as string);
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

function goToVersion(version: { id: string }) {
  const versionIdx = versions.value.findIndex((x) => x.id == version.id);
  router.push({
    query: versionIdx == 0 ? undefined : { version: version.id },
    hash: router.currentRoute.value.hash,
  });
}

const containerRef = ref<HTMLElement | null>(null);
const { focused: inContainerFocused } = useFocusWithin(containerRef);
const versionsGrid = useNavigationGrid<"name", HTMLElement>(
  computed(() => ["name"]),
  versions
);

// focus view when getting focus
watch(inContainerFocused, () => {
  if (inContainerFocused.value) {
    emit("show");
  } else {
    emit("blur");
  }
});
// handle explorer view focus and editor focus
watch(
  toRef(props, "focused"),
  () => {
    if (props.focused) {
      if (!inContainerFocused.value) {
        // focus currently active version if nothing was directly selected
        nextTick(() => versionsGrid.focus(0, "name"));
      }
    } else {
      versionsGrid.blur();
    }
  },
  { immediate: true }
);

defineExpose({
  count: computed(() => versionsQuery.value?.project?.versions.totalCount),
});
</script>
<template>
  <!-- Container (views should be a single root element) -->
  <div ref="containerRef">
    <!-- View header -->
    <div
      class="flex h-[31px] flex-row items-center justify-between border-b border-orange-900 border-opacity-[12%] px-3 py-2"
      :style="{
        height: appearance.editorHeaderHeight + 'px',
      }"
    >
      <span class="text-xs font-bold uppercase">
        History
        <span class="ml-1 rounded-lg bg-gray-200 px-1 font-normal text-gray-800" v-if="versionsCount != null">
          {{ versionsCount }}
        </span>
      </span>
      <!-- Version controls -->
      <!-- Note that this commit popover duplicates the one from the main version list -->
      <!-- This is because it's easier to open the right popover in the right place that way -->
      <CommitPopover
        v-if="head != null && isCurrent(head)"
        :version="head"
        :projectId="props.project.id"
        :prev-sem-ver-tag="lastSemVerTag ?? undefined"
        @commit="(c) => doCommit(c)"
        v-slot="{ open }"
      >
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
      </CommitPopover>
    </div>
    <!-- View versions -->
    <div class="relative flex-1" v-if="!loading">
      <!-- Versions -->
      <ul role="list" class="-mb-8 h-full w-full pb-10 pt-2">
        <li
          v-for="(version, versionIdx) in versions"
          :key="version.id"
          :ref="(ref) => versionsGrid.registerColumnRef(version.id, 'name', ref)"
          tabindex="-1"
          @keydown.up.exact.prevent="versionsGrid.navigateUp(version.id, 'name')"
          @keydown.down.exact.prevent="versionsGrid.navigateDown(version.id, 'name')"
          @keydown.enter.exact.prevent="goToVersion(version)"
          class="group outline-none"
        >
          <!-- Focus border is inside the inner div because of the vertical margin required for the line -->
          <div class="relative mb-2 border border-transparent pb-1 hover:bg-orange-100 group-focus:border-orange-600">
            <!-- Vertical line connecting versions -->
            <div class="mx-3">
              <span
                v-if="versionIdx !== versions.length - 1"
                class="absolute left-3 top-4 ml-[11px] h-full w-0.5 bg-gray-200"
                aria-hidden="true"
              />
            </div>
            <!-- Ideally we would have individual edits/revisions here as well -->
            <div class="relative flex space-x-2 px-3 py-0.5">
              <!-- Version icon -->
              <span
                class="ring-6 flex h-6 w-6 items-center justify-center rounded-full bg-gray-50 ring-gray-50 group-hover:bg-orange-100"
              >
                <component
                  :is="isHead(version) && !version.committed ? BookmarkDashedIcon : BookmarkIcon"
                  class="h-5 w-5"
                  :class="isCurrent(version) ? 'text-orange-600' : 'text-gray-700'"
                  aria-hidden="true"
                />
              </span>
              <!-- Version info -->
              <CommitPopover
                :version="version"
                :projectId="props.project.id"
                :prev-sem-ver-tag="versionIdx == 0 ? lastSemVerTag ?? undefined : undefined"
                as="div"
                class="flex min-w-0 flex-1 items-baseline justify-between space-x-4"
                @commit="(c) => doCommit(c)"
                v-slot="{ open }"
              >
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
                    <PopoverButton
                      v-if="project.canWrite"
                      class="p-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 group-hover:visible"
                      :class="open ? 'visible bg-orange-100 text-gray-700' : 'invisible'"
                    >
                      <PencilIcon class="h-3 w-3" />
                    </PopoverButton>
                  </p>
                  <!-- Tag button -->
                  <PopoverButton
                    class="flex w-fit flex-row gap-0.5 rounded-sm p-0.5 text-xs outline-none"
                    :class="[
                      version.tag == null ? 'text-gray-300 ' : 'text-gray-700',
                      project.canWrite ? 'hover:bg-orange-100 hover:text-gray-700' : '',
                      open ? 'bg-orange-100' : '',
                    ]"
                    :disabled="!project.canWrite"
                  >
                    <TagIcon class="h-4 w-4" />
                    <span :class="version.tag == null ? 'invisible group-hover:visible' : ''">
                      {{ version.tag || "Tag" }}
                    </span>
                  </PopoverButton>
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
              </CommitPopover>
            </div>
          </div>
        </li>
      </ul>
    </div>
  </div>
</template>
