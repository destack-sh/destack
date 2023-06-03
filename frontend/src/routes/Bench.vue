<script setup lang="ts">
import ClientsPopover from "@/components/basic/ClientsPopover.vue";
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import GenericNotFound from "@/components/basic/GenericNotFound.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import DeployPopover from "@/components/bench/DeployPopover.vue";
import EditorGroup from "@/components/editors/EditorGroup.vue";
import FeedbackPopover from "@/components/bench/FeedbackPopover.vue";
import HelpPopover from "@/components/bench/HelpPopover.vue";
import NotificationArea from "@/components/basic/NotificationArea.vue";
import NotificationPopover from "@/components/bench/NotificationPopover.vue";
import ViewExplorer from "@/components/views/ViewExplorer.vue";
import ViewHistory from "@/components/views/ViewHistory.vue";
import ViewIssues from "@/components/views/ViewIssues.vue";
import ProjectPopover from "@/components/bench/ProjectPopover.vue";
import SettingsPopover from "@/components/bench/SettingsPopover.vue";
import SharePopover from "@/components/bench/SharePopover.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { ProjectVisibility } from "@/gql/graphql";
import { provideAction, useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { useAuth } from "@/state/auth";
import { useBenchMigrations, useBenchPersistence, useBenchState, type ViewId } from "@/state/bench";
import { FileHeaderType, ProjectHeaderType, ProjectVersionHeaderType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperationsStore } from "@/state/operations";
import { useCurrentInterpModule, useVisibleErrors } from "@/state/module";
import { useModuleSync, useProjectSync } from "@/state/sync";
import { WS_CONNECTED } from "@/utils/globals";
import { PopoverButton } from "@headlessui/vue";
import { ClockIcon as ClockIconSolid } from "@heroicons/vue/20/solid";
import {
  ChatBubbleLeftIcon,
  ClockIcon,
  Cog8ToothIcon,
  CubeIcon,
  DocumentDuplicateIcon,
  ExclamationTriangleIcon,
  EyeIcon,
  FaceSmileIcon,
  GlobeAltIcon,
  HandRaisedIcon,
  LockClosedIcon,
  MagnifyingGlassIcon,
  QuestionMarkCircleIcon,
  XCircleIcon,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useElementSize, useTitle } from "@vueuse/core";
import Mousetrap from "mousetrap";
import {
  computed,
  onBeforeUnmount,
  ref,
  toRef,
  watch,
  watchEffect,
  type Component,
  type ComputedRef,
  type Ref,
} from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{
  owner: string;
  project: string;
  version?: string;
}>();

// views for the sidebar
type View = {
  id: ViewId;
  label: string;
  icon: Component;
  enabled: boolean;
};
const allViews: Ref<View[]> = computed(() => [
  { id: "explorer", label: "Explorer", icon: DocumentDuplicateIcon, enabled: true },
  { id: "search", label: "Search", icon: MagnifyingGlassIcon, enabled: false },
  { id: "history", label: "History", icon: ClockIcon, enabled: true },
  { id: "issues", label: "Issues", icon: ExclamationTriangleIcon, enabled: true },
  { id: "comments", label: "Comments", icon: ChatBubbleLeftIcon, enabled: false },
  { id: "environment", label: "Environment", icon: CubeIcon, enabled: false },
]);
const availableViews = computed(() => allViews.value.filter((v) => v.enabled));
const activeView: ComputedRef<View> = computed(() => {
  const view = availableViews.value.find((v) => v.id == bench.activeViewId);
  if (!view) {
    console.error("invalid view id: " + bench.activeViewId);
    bench.setActiveView(availableViews.value[0].id);
    return availableViews.value[0];
  }
  return view;
});

function toggleActiveView(viewId: ViewId, ignoreFocus: boolean) {
  if (bench.activeViewId == viewId && bench.showViewContent && (ignoreFocus || bench.focusedViewId == viewId)) {
    bench.showViewContent = false;
    bench.focusedViewId = null;
  } else {
    bench.focusView(viewId);
  }
}
provideAction({
  id: "bench.view.openExplorer",
  label: "View Explorer",
  shortcuts: ["alt+1"],
  apply: () => toggleActiveView("explorer", false),
});
provideAction({
  id: "bench.view.openHistory",
  label: "View History",
  shortcuts: ["alt+2"],
  apply: () => toggleActiveView("history", false),
});
const openIssues = provideAction({
  id: "bench.view.openIssues",
  label: "View Issues",
  shortcuts: ["alt+3"],
  apply: () => toggleActiveView("issues", false),
});
provideAction({
  id: "bench.view.openInstruction",
  label: "View Instruction",
  shortcuts: ["alt+4"],
  apply: () => toggleActiveView("instruction", false),
});

// other buttons for sidebar
type SidebarPopover = {
  component: Component;
  icon: Component;
  label: string;
};
const sidebarPopovers: SidebarPopover[] = [
  {
    component: FeedbackPopover,
    icon: HandRaisedIcon,
    label: "Give Feedback",
  },
  {
    component: HelpPopover,
    icon: QuestionMarkCircleIcon,
    label: "Help",
  },
  {
    component: SettingsPopover,
    icon: Cog8ToothIcon,
    label: "Settings",
  },
];

// get project header
const {
  error: projectError,
  result: projectResult,
  loading: projectLoading,
} = useQuery(
  graphql(/* GraphQL */ `
    query projectBySlug($owner: String!, $project: String!) {
      projectBySlug(owner: $owner, project: $project) {
        ...ProjectHeader
      }
    }
  `),
  () => ({
    owner: props.owner,
    project: props.project,
  })
);
const projectLoaded = computed(() => !!projectResult.value?.projectBySlug);
const project = computed(() => useFragment(ProjectHeaderType, projectResult.value?.projectBySlug));
const projectHead = computed(() => useFragment(ProjectVersionHeaderType, project.value?.head));

// default version to view = head (can be overridden by URL?)
const versionToViewId = computed(() => {
  if (props.version != null) {
    return props.version;
  } else {
    return projectHead.value?.id;
  }
});

// set up bench state
const bench = useBenchState();
const appearance = useAppearance();
const editorReady = computed(
  () => bench.currentProjectVersionId != null && bench.currentProjectVersionId == versionToViewId.value
);
const notifications = useNotifications();
const router = useRouter();
const viewContainerRef = ref<HTMLElement | null>(null);
const viewContainerSize = useElementSize(viewContainerRef);
const mainContainerRef = ref<HTMLElement | null>(null);
const mainContainerSize = useElementSize(mainContainerRef);

// sync title bar with project info
const title = useTitle();
watchEffect(() => {
  if (projectError.value) {
    title.value = "Page not found";
  } else {
    if (bench.focusedEditor != null) {
      title.value = (bench.focusedEditor.path || "(Untitled)") + " • " + `${props.owner}/${props.project}`;
    } else {
      title.value = `${props.owner}/${props.project}${project.value ? " • " + project.value.name : ""}`;
    }
  }
});

// get project content
const { error: versionError, result: versionResult } = useQuery(
  graphql(/* GraphQL */ `
    query projectVersionContent($id: GlobalID!) {
      projectVersion(id: $id) {
        id
        id
        name
        tag
        description
        createdAt
        committed
        committedAt
        files(filters: { isVisible: true }) {
          totalCount
          edges {
            node {
              id
              ...FileHeader
            }
          }
        }
      }
    }
  `),
  () => ({ id: versionToViewId.value }),
  () => ({ enabled: !!versionToViewId.value })
);
const version = computed(() => versionResult.value?.projectVersion);
const versionLoaded = computed(() => !!version.value);
watch(versionError, () => {
  if (versionError.value != null) {
    const atHead = version.value?.id == projectHead.value?.id;
    notifications.show({
      kind: "error",
      type: "version.loadFailed",
      message: "Version unavailable",
      description: "Failed to load version.",
    });
    if (!atHead) {
      // revert to head
      router.replace({ hash: router.currentRoute.value.hash });
    } // otherwise there's nothing we can do (?)
  }
});

// filter deletedAt to increase responsiveness
const files = computed(
  () =>
    version.value?.files.edges.map((f) => useFragment(FileHeaderType, f.node)).filter((f) => f.deletedAt == null) || []
);

// actions (ensure global actions are available)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const actions = useActions();
const operationsStore = useOperationsStore();
const { connected: runtimeConnected, lastUpdated: runtimeLastUpdated } = useCurrentInterpModule();
const hasStaleInflightStateOps = computed(() => operationsStore.hasInflightLike({ stateless: false, stale: true }));
const { getTimeFromNowString } = useTimeFromNow();

// routing
const consideredUrl = ref(false);

function prettifyPath(path: string) {
  // replace non-URL friendly characters with dashes
  return path.replace(/[^a-zA-Z0-9-_./]/g, "-");
}

// focus file from url if hash changes and none is open (once)
watchEffect(() => {
  const hash = router.currentRoute.value.hash;
  if (versionLoaded.value && !consideredUrl.value && editorReady.value) {
    let file = null;
    if (hash) {
      const path = hash.substring(1);
      file = files.value.find((file) => prettifyPath(file.path) === path);
    } else if (files.value.length == 1) {
      // special case: open "Getting Started" file if it exists and nothing is open :GettingStarted
      file = files.value.find((file) => file.path === "Getting Started");
    }
    if (file) {
      bench.focusFile(file as any);
    }
    consideredUrl.value = true;
  }
});

// change url if focused editor changes
watchEffect(() => {
  if (bench.focusedEditor != null) {
    // set hash to open path
    if (consideredUrl.value) {
      const prettyPath = prettifyPath(bench.focusedEditor.path);
      router.replace({ hash: `#${prettyPath}`, query: router.currentRoute.value.query });
    }
  } else if (editorReady.value && consideredUrl.value) {
    // clear hash
    router.replace({ hash: ``, query: router.currentRoute.value.query });
  }
});

// suppress control+s (suggest snapshot instead)
Mousetrap.bind(["ctrl+s", "meta+s"], () => {
  notifications.showIf(
    {
      type: "editor.suppressSave",
      kind: "notice",
      message: "Saving is automatic",
      description: "Changes are automatically synchronized.",
      actionText: "Snapshot",
      action: () => actions.apply("version.commit"),
    },
    { lastActiveMs: 60000 }
  );
  return false;
});

const runtime = useCurrentInterpModule();
const moduleSync = useModuleSync(versionToViewId);
const projectSync = useProjectSync(toRef(bench, "currentProjectId"));
const visibleErrors = useVisibleErrors();
const auth = useAuth();

// show notification if disconnected/reconnected
const connectionLost = ref(false);
const wasEverConnected = ref(false);
watch(
  () => [WS_CONNECTED.value, runtime.connected.value],
  () => {
    if (WS_CONNECTED.value && runtime.connected.value) {
      wasEverConnected.value = true;
    }

    if (!WS_CONNECTED.value && !connectionLost.value && wasEverConnected.value) {
      notifications.show({
        type: "runtime.disconnected",
        kind: "warning",
        message: "Disconnected",
        description: "Bench is disconnected.",
      });
      connectionLost.value = true;
    } else if (WS_CONNECTED.value && connectionLost.value && runtime.connected.value) {
      connectionLost.value = false;
      notifications.show({
        type: "runtime.reconnected",
        kind: "success",
        message: "Reconnected",
        description: "Bench has reconnected.",
      });
      notifications.dismissIf({ type: "runtime.disconnected" });
    }
  }
);

// provide Zen mode
provideAction({
  id: "editor.zenMode",
  label: computed(() => (bench.zenMode ? "Exit Zen Mode" : "Enter Zen Mode")),
  shortcuts: ["alt+z"],
  apply: () => {
    bench.setZenMode(!bench.zenMode);
    notifications.dismissIf({ type: "zenMode" });
    notifications.show({
      type: "zenMode",
      kind: "notice",
      message: bench.zenMode ? "Zen Mode on" : "Zen Mode off",
      description: bench.zenMode ? "Minimize distractions." : "Restored full editor view.",
      action: () => bench.setZenMode(!bench.zenMode),
      actionText: "Toggle",
    });
  },
});

const { load } = useBenchPersistence();
const { migrateTo: migrate, migrating } = useBenchMigrations();

// manage read/write access
watchEffect(() => {
  bench.readonly =
    !versionLoaded.value ||
    migrating.value ||
    versionToViewId.value != projectHead.value?.id ||
    !project.value?.canWrite ||
    version.value?.committed == true;
});

// prepare bench state for project whenever project (head) changes
watchEffect(async () => {
  if (
    !migrating.value &&
    project.value != null &&
    versionToViewId.value != null &&
    (bench.currentProjectId != project.value.id || bench.currentProjectVersionId != versionToViewId.value)
  ) {
    // try to load bench state
    console.log(`load bench ${project.value.id} at ${versionToViewId.value}`);
    load(project.value.id);
    if (bench.currentProjectId == project.value?.id) {
      // migrate if there is a new version of the same project
      // (loads overwrites bench state for the entire project,
      //  so editor.currentProjectVersionId will point to its last known version)
      if (bench.currentProjectVersionId != versionToViewId.value) {
        const success = migrate(
          bench.currentProjectId as string,
          bench.currentProjectVersionId as string,
          versionToViewId.value
        );
        bench.currentProjectVersionId = versionToViewId.value;
        if (!success) {
          notifications.dismissIf({ type: "bench.migrate.failed" });
          notifications.show({
            kind: "warning",
            type: "bench.migrate.failed",
            message: "Bench migration failed",
            description: "Bench could not be migrated.",
          });
        } else {
          notifications.dismissIf({ type: "bench.migrate.success" });
          notifications.show({
            kind: "success",
            type: "bench.migrate.success",
            message: "Bench migrated",
            description: "Bench migrated successfully.",
          });
        }
      }
    } else {
      console.log(`reset bench ${project.value.id}`); // already happened
    }
  }
});

// clear bench state when exiting view
onBeforeUnmount(() => {
  if (bench.currentProjectId == project.value?.id) {
    bench.$reset();
  }
});
</script>

<template>
  <!-- Root -->
  <div class="relative flex h-full flex-col bg-gray-50">
    <!-- Header with controls and auth -->
    <FatHeader v-show="bench.showGlobalHeader">
      <!-- Left side: organizational & status -->
      <template v-slot:left>
        <!-- Home -->
        <HomeButton />
        <!-- Project menu -->
        <div
          v-if="projectLoading || projectLoaded"
          class="ml-2.5 flex flex-row items-baseline gap-0.5 whitespace-nowrap"
        >
          <!-- Owner -->
          <router-link :to="`/${props.owner}`" class="rounded-sm p-1 text-sm hover:bg-orange-100">
            {{ props.owner }}
          </router-link>
          <span class="text-gray-500">/</span>
          <!-- Project button -->
          <ProjectPopover v-if="projectLoaded" :project="project">
            <template v-slot:button="{ open }">
              <PopoverButton
                class="flex h-full items-center justify-between rounded-sm bg-white p-1 text-left hover:bg-orange-100 focus:outline-none"
                :class="{ 'bg-orange-100 focus:bg-orange-100': open }"
              >
                <span class="truncate text-sm font-bold">{{ props.project }}</span>
                <FadeTransition mode="out-in">
                  <component
                    :is="project.visibility != ProjectVisibility.Public ? LockClosedIcon : GlobeAltIcon"
                    class="ml-1.5 h-4 w-4 text-gray-700"
                  />
                </FadeTransition>
              </PopoverButton>
            </template>
          </ProjectPopover>
          <!-- While loading, imitate project button -->
          <span v-else class="truncate p-1 text-sm font-bold">
            {{ props.project }}
          </span>
          <!-- Branch info -->
          <!-- not yet -->
          <!-- Version info (if not at head) -->
          <FadeTransition>
            <div
              v-if="versionToViewId != projectHead?.id && versionLoaded"
              class="ml-1 flex flex-row gap-2 rounded-sm border border-orange-900 border-opacity-[15%] bg-orange-600 px-3 py-1 text-sm text-white"
            >
              <span class="relative">
                <ClockIconSolid class="absolute top-0.5 h-4 w-4 text-white" />
                <span class="ml-5 font-bold">{{ version?.tag ?? version?.name ?? "Autosave" }}</span>
              </span>
              <router-link
                :to="{ hash: router.currentRoute.value.hash }"
                class="font-bold underline decoration-white decoration-dashed underline-offset-4 hover:decoration-solid"
              >
                Back
              </router-link>
              <button
                v-if="project.canWrite"
                class="underline decoration-white decoration-dashed underline-offset-4 hover:decoration-solid"
                @click="actions.apply('version.restore')"
              >
                Restore
              </button>
            </div>
          </FadeTransition>
          <!-- Read-only project notice -->
          <div
            v-if="project != null && bench.readonly"
            class="ml-2 flex flex-row gap-2 rounded-sm border border-orange-900 border-opacity-[12%] bg-orange-100 px-2 py-1 text-sm"
          >
            <span class="relative flex flex-row gap-1 text-gray-900">
              <EyeIcon class="absolute top-0.5 h-4 w-4" />
              <span class="ml-5 select-none">Viewer</span>
            </span>
            <!-- <button>fork</button> -->
          </div>
        </div>
        <!-- Status -->
        <div v-if="versionLoaded" class="ml-2 flex items-center">
          <!-- Operations status -->
          <span class="flex items-center gap-1 p-1 transition-opacity" v-show="hasStaleInflightStateOps">
            <svg
              viewBox="0 0 10 10"
              class="h-1 w-1"
              :class="{
                'text-orange-600': !hasStaleInflightStateOps,
                'animate-spin text-gray-400': hasStaleInflightStateOps,
              }"
            >
              <rect width="10" height="10" rx="1" ry="1" fill="currentColor" />
            </svg>
            <span class="text-sm text-gray-500">saving</span>
          </span>
          <!-- Runtime status -->
          <span class="flex items-center gap-1 p-1 transition-all">
            <svg
              viewBox="0 0 10 10"
              class="h-1 w-1"
              :class="{ 'text-orange-600': runtimeConnected, 'text-gray-400': !runtimeConnected }"
            >
              <rect width="10" height="10" rx="1" ry="1" fill="currentColor" />
            </svg>
            <Transition appear>
              <span class="text-sm text-gray-500" v-show="!runtimeConnected">
                {{ runtimeConnected ? "connected" : "connecting" }}
              </span>
            </Transition>
            <span class="text-sm text-gray-500" v-if="bench.debug && runtimeLastUpdated != null">
              {{ getTimeFromNowString(runtimeLastUpdated) }}
            </span>
          </span>
        </div>
        <!-- Comments/notes, issues/warnings/lints, errors -->
        <div v-if="versionLoaded" class="ml-2 flex items-center gap-2">
          <!-- Errors -->
          <button
            class="flex items-center gap-0.5 rounded-sm p-1 hover:bg-orange-100"
            v-if="visibleErrors?.length || 0 > 0"
            @click="openIssues.apply"
          >
            <XCircleIcon class="h-5 w-5 text-red-700" />
            <span class="text-sm text-gray-700">{{ visibleErrors?.length }}</span>
          </button>
        </div>
      </template>

      <!-- Center: main metrics -->
      <template v-slot:center>
        <!-- No metrics right now :BuildEvaluate -->
      </template>

      <!-- Right side: controls & profile -->
      <template v-slot:right>
        <!-- Bench-global controls -->
        <FadeTransition>
          <div v-if="versionLoaded" class="flex h-full items-center space-x-2 pl-4">
            <SharePopover @show="bench.showGlobalHeader = true" />
            <DeployPopover :project="project" @show="bench.showGlobalHeader = true" />
            <OmniCreate @show="bench.showGlobalHeader = true" />
            <NotificationPopover @show="bench.showGlobalHeader = true" />
          </div>
        </FadeTransition>
        <ClientsPopover v-if="auth.loggedIn.value" class="ml-2" size="large" />
        <ProfileButton class="ml-2" />
      </template>
    </FatHeader>
    <!-- Main content (sidebar + editor), spans horizontally -->
    <!-- It's important that conditional components are all v-show (not v-if)
          both to make them instant and to provide their actions -->
    <div v-show="projectLoaded" class="relative flex flex-1 flex-row">
      <!-- Sidebar of view buttons & views -->
      <aside
        class="flex h-full resize-x"
        :class="{
          'w-64 lg:w-80': bench.showViewContent && bench.showViewSelection,
          'w-48 lg:w-64': bench.showViewContent && !bench.showViewSelection,
        }"
      >
        <div
          class="flex h-full min-h-0 flex-col border-r border-orange-900 border-opacity-[12%]"
          v-show="bench.showViewSelection"
        >
          <!-- Top of sidebar: view selection -->
          <div class="flex flex-1 flex-col">
            <button
              class="group relative rounded-sm border-l-2 border-gray-50 px-3 py-2.5 text-gray-600 hover:bg-orange-100"
              :class="view.id == activeView.id && bench.showViewContent ? 'border-orange-600 text-orange-600' : ''"
              v-for="view in availableViews"
              :key="view.id"
              @click="toggleActiveView(view.id, true)"
            >
              <span class="sr-only">{{ view.label }}</span>
              <component :is="view.icon" class="h-6 w-6" aria-hidden="true" />
              <!-- Tooltip -->
              <span
                v-if="view.id != activeView.id || !bench.showViewContent"
                class="pointer-events-none absolute left-full top-3 z-30 rounded-sm bg-white px-1 text-sm opacity-0 ring-1 ring-orange-900 ring-opacity-[25%] transition duration-75 group-hover:opacity-100"
              >
                {{ view.label }}
              </span>
            </button>
          </div>
          <!-- Bottom of sidebar: feedback, help, settings popovers -->
          <component v-for="popover in sidebarPopovers" :key="popover.label" :is="popover.component">
            <template v-slot:button="{ open }">
              <PopoverButton
                class="group relative rounded-sm border-l-2 px-3 py-2.5 text-gray-600 outline-none hover:bg-orange-100 focus:ring-0"
                :class="open ? 'border-orange-600 text-orange-600' : ''"
              >
                <span class="sr-only">{{ popover.label }}</span>
                <component :is="popover.icon" class="h-6 w-6" aria-hidden="true" />
                <!-- Tooltip -->
                <span
                  v-if="!open"
                  class="pointer-events-none absolute left-full top-3 z-10 whitespace-nowrap rounded-sm bg-white px-1 text-sm opacity-0 ring-1 ring-orange-900 ring-opacity-[25%] transition-opacity duration-75 group-hover:opacity-100"
                >
                  {{ popover.label }}
                </span>
              </PopoverButton>
            </template>
          </component>
        </div>
        <!-- View content -->
        <div
          ref="viewContainerRef"
          class="relative h-full max-h-full max-w-full flex-1 border-r border-orange-900 border-opacity-[12%]"
          v-show="bench.showViewContent"
        >
          <!-- These must be v-show, not v-if, see note above -->
          <ViewExplorer
            v-show="activeView.id == 'explorer'"
            @show="bench.focusView('explorer')"
            @blur="bench.blurView('explorer')"
            :files="files"
            :focused="bench.focusedViewId == 'explorer'"
            :container-size="viewContainerSize"
            class="scroll-hidden overflow-y-auto"
            :style="{ width: viewContainerSize.width.value + 'px', height: viewContainerSize.height.value + 'px' }"
          />
          <ViewHistory
            v-show="activeView.id == 'history'"
            v-if="project != null"
            @show="bench.focusView('history')"
            @blur="bench.blurView('history')"
            :project="project"
            :focused="bench.focusedViewId == 'history'"
            :current-version="version"
            :container-size="viewContainerSize"
            class="scroll-hidden overflow-y-auto"
            :style="{ width: viewContainerSize.width.value + 'px', height: viewContainerSize.height.value + 'px' }"
          />
          <ViewIssues
            v-show="activeView.id == 'issues'"
            @show="bench.focusView('issues')"
            @blur="bench.blurView('issues')"
            :focused="bench.focusedViewId == 'issues'"
            :container-size="viewContainerSize"
            class="scroll-hidden overflow-y-auto"
            :style="{ width: viewContainerSize.width.value + 'px', height: viewContainerSize.height.value + 'px' }"
          />
          <!-- Unknown view -->
          <div
            v-if="
              activeView.id != 'explorer' &&
              activeView.id != 'history' &&
              activeView.id != 'issues' &&
              activeView.id != 'instruction'
            "
            class="my-4 flex flex-col items-center justify-center gap-2 px-3 text-center"
          >
            <FaceSmileIcon class="h-7 w-7 rotate-180 text-gray-500" />
            <span class="text-sm text-gray-700">Let's pretend you didn't see this.</span>
          </div>
        </div>
      </aside>
      <!-- Main editor area -->
      <main
        ref="mainContainerRef"
        v-show="versionLoaded"
        class="flex h-full w-full flex-1 divide-x divide-orange-900 divide-opacity-[12%] bg-gray-50"
      >
        <!-- Left editor group -->
        <div class="relative flex-1">
          <div class="absolute left-0 top-0 h-full w-full overflow-hidden">
            <EditorGroup :group="bench.left" class="h-full w-full" />
          </div>
        </div>
        <!-- Right editor group -->
        <div class="relative flex-1" v-if="bench.right.editors.length > 0">
          <div class="absolute left-0 top-0 h-full w-full overflow-hidden">
            <EditorGroup :group="bench.right" class="h-full w-full" />
          </div>
        </div>
      </main>
    </div>
    <GenericNotFound v-if="!projectLoading && !projectLoaded" class="pb-12" />
    <NotificationArea />
  </div>
</template>
