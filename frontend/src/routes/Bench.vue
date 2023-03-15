<script setup lang="ts">
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import GenericNotFound from "@/components/basic/GenericNotFound.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import DeployPopover from "@/components/DeployPopover.vue";
import EditorGroupInterface from "@/components/EditorGroupInterface.vue";
import HelpPopover from "@/components/HelpPopover.vue";
import MainSymbolControls from "@/components/MainSymbolControls.vue";
import NotificationArea from "@/components/notifications/NotificationArea.vue";
import NotificationPopover from "@/components/notifications/NotificationPopover.vue";
import ViewExplorer from "@/components/panels/ViewExplorer.vue";
import ViewHistory from "@/components/panels/ViewHistory.vue";
import ViewIssues from "@/components/panels/ViewIssues.vue";
import ProjectPopover from "@/components/ProjectPopover.vue";
import SettingsPopover from "@/components/SettingsPopover.vue";
import SharePopover from "@/components/SharePopover.vue";
import { useNow, useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { JobStatus, JobType, ProjectVisibility, type InterpJob, type InterpSymbol } from "@/gql/graphql";
import { provideAction, useActions } from "@/state/actions";
import { hostStatementActions, provideStatementActions } from "@/state/actions/statement";
import {
  useEditorMigrations,
  useEditorPersistence,
  useEditorState,
  type FileEditor,
  type ViewId,
} from "@/state/editor";
import { FileHeaderType, ProjectHeaderType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperationsStore } from "@/state/operations";
import { symbolOf, useCurrentModuleRuntime, useVisibleErrors } from "@/state/runtime";
import { WS_CONNECTED } from "@/utils/globals";
import { PopoverButton } from "@headlessui/vue";
import { ClockIcon as ClockIconSolid } from "@heroicons/vue/20/solid";
import {
  ClipboardDocumentIcon,
  ClockIcon,
  Cog8ToothIcon,
  ExclamationTriangleIcon,
  EyeIcon,
  GlobeAltIcon,
  LockClosedIcon,
  QuestionMarkCircleIcon,
  XCircleIcon,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useTitle, whenever } from "@vueuse/core";
import { DateTime } from "luxon";
import Mousetrap from "mousetrap";
import { computed, onBeforeUnmount, ref, toRef, watch, watchEffect, type Component, type ComputedRef } from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{
  owner: string;
  project: string;
  version?: string;
}>();

// views for the sidebar
type View = {
  id: "explorer" | "history" | "issues";
  name: string;
  icon: Component;
};
const views: View[] = [
  { id: "explorer", name: "Explorer", icon: ClipboardDocumentIcon },
  { id: "history", name: "History", icon: ClockIcon },
  { id: "issues", name: "Issues", icon: ExclamationTriangleIcon },
];
const activeView: ComputedRef<View> = computed(() => {
  const view = views.find((v) => v.id == editor.activeViewId);
  if (!view) {
    console.error("invalid view id: " + editor.activeViewId);
    editor.setActiveView(views[0].id);
    return views[0];
  }
  return view;
});

function toggleActiveView(viewId: ViewId, ignoreFocus: boolean) {
  if (editor.activeViewId == viewId && editor.showViewContent && (ignoreFocus || editor.focusedViewId == viewId)) {
    editor.showViewContent = false;
    editor.focusedViewId = null;
  } else {
    editor.focusView(viewId);
  }
}
provideAction({
  id: "editor.view.openExplorer",
  label: "View Explorer",
  shortcuts: ["alt+1"],
  apply: () => toggleActiveView("explorer", false),
});
provideAction({
  id: "editor.view.openHistory",
  label: "View History",
  shortcuts: ["alt+2"],
  apply: () => toggleActiveView("history", false),
});
const openIssues = provideAction({
  id: "editor.view.openIssues",
  label: "View Issues",
  shortcuts: ["alt+3"],
  apply: () => toggleActiveView("issues", false),
});

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

// default version to view = head (can be overridden by URL?)
const versionToViewId = computed(() => {
  if (props.version != null) {
    return props.version;
  } else {
    return project.value?.head.id;
  }
});

// set up editor state
const editor = useEditorState();
const editorReady = computed(
  () => editor.currentProjectVersionId != null && editor.currentProjectVersionId == versionToViewId.value
);
const notifications = useNotifications();
const router = useRouter();

// sync title bar with project info
const title = useTitle();
watchEffect(() => {
  if (projectError.value) {
    title.value = "Page not found";
  } else {
    if (editor.focusedEditor != null) {
      title.value = editor.focusedEditor.path + " • " + `${props.owner}/${props.project}`;
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
    const atHead = version.value?.id == project.value?.head.id;
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
const { connected: runtimeConnected, lastUpdated: runtimeLastUpdated } = useCurrentModuleRuntime();
const hasStaleInflightStateOps = computed(() => operationsStore.hasInflightLike({ stateless: false, stale: true }));
const { getTimeFromNowString } = useTimeFromNow();

// get currently running jobs
const now = useNow(100);
const { jobs } = useCurrentModuleRuntime();
const activeJobs = computed(() =>
  jobs.value?.filter(
    (job) =>
      job.status == JobStatus.Running &&
      job.startedAt != null &&
      now.value.diff(DateTime.fromISO(job.startedAt)).as("milliseconds") > 500
  )
);

function getJobTitle(job: InterpJob) {
  if (job.type == JobType.Interp) {
    return "Analyzing";
  } else if (job.type == JobType.Build) {
    return "Building";
  } else if (job.type == JobType.Generate) {
    return "Generating";
  } else if (job.type == JobType.Evaluate) {
    return "Evaluating";
  }
}
function getJobSubject(job: InterpJob): InterpSymbol | undefined {
  if (job.symbol?.id == null) {
    return undefined;
  } else {
    return symbolOf(job.symbol?.id);
  }
}

// sync editor paths
// TODO @Cleanup: move sync editor paths into EditorInterface
watchEffect(() => {
  if (!files.value) return;
  editor.editors.forEach((editor) => {
    if (editor.type == "file") {
      const fileEditor = editor as FileEditor;
      const file = files.value.find((f) => f.id == fileEditor.fileId);
      if (!file) return; // ignore
      editor.path = file.path;
    }
  });
});

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
      editor.focusFile(file as any);
    }
    consideredUrl.value = true;
  }
});

// change url if focused editor changes
watchEffect(() => {
  if (editor.focusedEditor != null) {
    // set hash to open path
    if (consideredUrl.value) {
      const prettyPath = prettifyPath(editor.focusedEditor.path);
      router.replace({ hash: `#${prettyPath}`, query: router.currentRoute.value.query });
    }
  } else if (editorReady.value && consideredUrl.value) {
    // clear hash
    router.replace({ hash: ``, query: router.currentRoute.value.query });
  }
});

// suppress control+s (offer named commit instead)
Mousetrap.bind(["ctrl+s"], () => {
  notifications.showIf(
    {
      type: "saveSuppressed",
      kind: "notice",
      message: "Saving is automatic",
      description: "All changes are synced automatically.",
      actionText: "Snapshot",
      action: () => actions.apply("version.commit"),
    },
    { lastActiveMs: 60000 }
  );
  return false;
});

const runtime = useCurrentModuleRuntime();
const visibleErrors = useVisibleErrors();

// show notification if disconnected/reconnected
const connectionLost = ref(false);
const everConnected = ref(false);
watch(
  () => [WS_CONNECTED.value, runtime.connected.value],
  () => {
    if (WS_CONNECTED.value && runtime.connected.value) {
      everConnected.value = true;
    }

    if (!WS_CONNECTED.value && !connectionLost.value && everConnected.value) {
      notifications.show({
        type: "runtime.disconnected",
        kind: "warning",
        message: "Disconnected",
        description: "The Bench runtime disconnected.",
      });
      connectionLost.value = true;
    } else if (WS_CONNECTED.value && connectionLost.value && runtime.connected.value) {
      connectionLost.value = false;
      notifications.show({
        type: "runtime.reconnected",
        kind: "success",
        message: "Reconnected",
        description: "The Bench runtime reconnected nicely.",
      });
      notifications.dismissIf({ type: "runtime.disconnected" });
    }
  }
);

// provide Zen mode
provideAction({
  id: "editor.zenMode",
  label: computed(() => (editor.zenMode ? "Exit Zen Mode" : "Enter Zen Mode")),
  shortcuts: ["alt+z"],
  apply: () => {
    editor.setZenMode(!editor.zenMode);
    notifications.dismissIf({ type: "zenMode" });
    notifications.show({
      type: "zenMode",
      kind: "notice",
      message: editor.zenMode ? "Zen Mode on" : "Zen Mode off",
      description: editor.zenMode ? "Minimize distractions." : "Restored full editor view.",
      action: () => editor.setZenMode(!editor.zenMode),
      actionText: "Toggle",
    });
  },
});

// left click anywhere clears editor selection
function clearSelectionIfLeftClick(e: MouseEvent) {
  if (e.button == 0) {
    editor.clearSelection();
  }
}
document.addEventListener("click", clearSelectionIfLeftClick);
onBeforeUnmount(() => document.removeEventListener("click", clearSelectionIfLeftClick));
// whenever editing -> clears selection
whenever(toRef(editor, "editingElement"), () => editor.clearSelection());

const { load } = useEditorPersistence();
const { migrateTo, migrating } = useEditorMigrations();

// manage read/write access
watchEffect(() => {
  editor.readonly =
    !versionLoaded.value ||
    migrating.value ||
    versionToViewId.value != project.value?.head.id ||
    !project.value?.canWrite ||
    version.value?.committed == true;
});

// prepare editor state for project whenever project (head) changes
watchEffect(async () => {
  if (
    !migrating.value &&
    project.value != null &&
    versionToViewId.value != null &&
    (editor.currentProjectId != project.value.id || editor.currentProjectVersionId != versionToViewId.value)
  ) {
    // try to load editor state
    editor.setProject(project.value.id, versionToViewId.value);
    load();
    console.log(`loaded editor state for project ${project.value.id} version ${versionToViewId.value}`);
    if (editor.currentProjectId == project.value?.id) {
      // migrate if there is a new version of the same project
      // (loads overwrites editor state for the entire project,
      //  so editor.currentProjectVersionId will point to its last known version)
      if (editor.currentProjectVersionId != versionToViewId.value) {
        migrateTo(editor.currentProjectId as string, versionToViewId.value, editor.currentProjectVersionId as string);
      }
    } else {
      console.log(`reset editor state for project ${project.value.id}`);
      // (happens in state.setProject)
    }
  }
});

// clear editor state when exiting view
onBeforeUnmount(() => {
  if (editor.currentProjectId == project.value?.id) {
    editor.$reset();
  }
});
</script>

<template>
  <!-- Root -->
  <div class="relative flex h-full flex-col bg-gray-50">
    <!-- Header with controls and auth -->
    <FatHeader v-show="editor.showGlobalHeader">
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
              v-if="versionToViewId != project?.head?.id && versionLoaded"
              class="ml-1 flex flex-row gap-2 rounded-sm bg-orange-600 px-3 py-1 text-sm text-white"
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
          <!-- Read-only project info -->
          <div
            v-if="project != null && !project?.canWrite"
            class="ml-2 flex flex-row gap-2 rounded-sm bg-orange-100 px-2 py-1 text-sm"
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
              :class="{ 'text-orange-600': !hasStaleInflightStateOps, 'text-gray-400': hasStaleInflightStateOps }"
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
            <span class="text-sm text-gray-500" v-if="editor.debug && runtimeLastUpdated != null">
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
        <!-- Current worker jobs -->
        <div v-if="versionLoaded" class="ml-2 flex items-center gap-2">
          <FadeTransition>
            <span v-for="job in activeJobs" :key="job.id" class="text-sm text-gray-500">
              {{ getJobTitle(job as InterpJob) }}
              <span v-if="getJobSubject(job as InterpJob) != null">
                {{ getJobSubject(job as InterpJob)?.name }}
              </span>
              ...
            </span>
          </FadeTransition>
        </div>
      </template>

      <!-- Right side: controls & profile -->
      <template v-slot:right>
        <!-- Current "main" statement controls -->
        <FadeTransition>
          <MainSymbolControls v-if="versionLoaded && !editor.readonly" />
        </FadeTransition>
        <!-- Bench-global controls -->
        <FadeTransition>
          <div v-if="versionLoaded" class="flex h-full items-center space-x-2 pl-4">
            <SharePopover @show="editor.showGlobalHeader = true" />
            <DeployPopover :project="project" @show="editor.showGlobalHeader = true" />
            <OmniCreate @show="editor.showGlobalHeader = true" />
            <NotificationPopover @show="editor.showGlobalHeader = true" />
          </div>
        </FadeTransition>
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
          'w-64 lg:w-80': editor.showViewContent && editor.showViewSelection,
          'w-48 lg:w-64': editor.showViewContent && !editor.showViewSelection,
        }"
      >
        <div
          class="flex h-full min-h-0 flex-col border-r border-orange-900 border-opacity-[12%]"
          v-show="editor.showViewSelection"
        >
          <!-- View selection -->
          <div class="flex flex-1 flex-col">
            <button
              class="rounded-sm border-l-2 border-gray-50 py-2.5 px-3 text-gray-600 hover:bg-orange-100"
              :class="view.name == activeView.name && editor.showViewContent ? 'border-orange-600 text-orange-600' : ''"
              v-for="view in views"
              :key="view.name"
              @click="toggleActiveView(view.id, true)"
            >
              <span class="sr-only">{{ view.name }}</span>
              <component :is="view.icon" class="h-6 w-6" aria-hidden="true" />
            </button>
          </div>
          <!-- Help & settings -->
          <HelpPopover>
            <template v-slot:button="{ open }">
              <PopoverButton
                class="rounded-sm border-l-2 px-3 py-2.5 text-gray-600 outline-none hover:bg-orange-100 focus:ring-0"
                :class="open ? 'border-orange-600 text-orange-600' : ''"
              >
                <span class="sr-only">Help</span>
                <QuestionMarkCircleIcon class="h-6 w-6" aria-hidden="true" />
              </PopoverButton>
            </template>
          </HelpPopover>
          <SettingsPopover>
            <template v-slot:button="{ open }">
              <PopoverButton
                class="rounded-sm border-l-2 px-3 py-2.5 text-gray-600 outline-none hover:bg-orange-100 focus:ring-0"
                :class="open ? 'border-orange-600 text-orange-600' : ''"
              >
                <span class="sr-only">Settings</span>
                <Cog8ToothIcon class="h-6 w-6" aria-hidden="true" />
              </PopoverButton>
            </template>
          </SettingsPopover>
        </div>
        <!-- View content -->
        <div
          ref="viewsContainerRef"
          class="relative h-full flex-1 border-r border-orange-900 border-opacity-[12%]"
          v-show="editor.showViewContent"
        >
          <!-- These must be v-show, not v-if, see note above -->
          <div class="absolute top-0 left-0 h-full w-full overflow-hidden">
            <ViewExplorer
              v-show="activeView.id == 'explorer'"
              @show="editor.focusView('explorer')"
              @blur="editor.blurView('explorer')"
              :files="files"
              :focused="editor.focusedViewId == 'explorer'"
            />
            <ViewHistory
              v-show="activeView.id == 'history'"
              v-if="project != null"
              @show="editor.focusView('history')"
              @blur="editor.blurView('history')"
              :project="project"
              :focused="editor.focusedViewId == 'history'"
              :current-version="version"
            />
            <ViewIssues
              v-show="activeView.id == 'issues'"
              @show="editor.focusView('issues')"
              @blur="editor.blurView('issues')"
              :focused="editor.focusedViewId == 'issues'"
            />
          </div>
        </div>
      </aside>
      <!-- Main editor area -->
      <main
        v-show="versionLoaded"
        class="flex h-full w-full flex-1 divide-x divide-orange-900 divide-opacity-[12%] bg-gray-50"
      >
        <!-- Left editor group -->
        <div class="relative flex-1">
          <div class="absolute top-0 left-0 h-full w-full overflow-hidden">
            <EditorGroupInterface :group="editor.left" class="h-full w-full" />
          </div>
        </div>
        <!-- Right editor group -->
        <div class="relative flex-1" v-if="editor.right.editors.length > 0">
          <div class="absolute top-0 left-0 h-full w-full overflow-hidden">
            <EditorGroupInterface :group="editor.right" class="h-full w-full" />
          </div>
        </div>
      </main>
    </div>
    <GenericNotFound v-if="!projectLoading && !projectLoaded" class="pb-12" />
    <NotificationArea />
  </div>
</template>
