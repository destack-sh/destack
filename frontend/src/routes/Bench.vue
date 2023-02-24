<script setup lang="ts">
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import GenericNotFound from "@/components/basic/GenericNotFound.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import OmniCreate from "@/components/basic/OmniCreate.vue";
import ProfileButton from "@/components/basic/ProfileButton.vue";
import NotificationArea from "@/components/container/NotificationArea.vue";
import EditorGroupInterface from "@/components/EditorGroupInterface.vue";
import GlobalControls from "@/components/GlobalControls.vue";
import MainSymbolControls from "@/components/MainSymbolControls.vue";
import ViewExplorer from "@/components/panels/ViewExplorer.vue";
import ViewHistory from "@/components/panels/ViewHistory.vue";
import ViewIssues from "@/components/panels/ViewIssues.vue";
import ProjectPopover from "@/components/ProjectPopover.vue";
import SettingsPopover from "@/components/SettingsPopover.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { ProjectVisibility } from "@/gql/graphql";
import { provideAction, useActions } from "@/state/actions";
import { useEditorMigrations, useEditorPersistence, useEditorState, type FileEditor } from "@/state/editor";
import { FileHeaderType, ProjectHeaderType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperationsStore } from "@/state/operations";
import { useCurrentModuleRuntime } from "@/state/runtime";
import { WS_CONNECTED } from "@/utils/globals";
import { Menu, MenuButton, MenuItem, MenuItems, PopoverButton } from "@headlessui/vue";
import {
  ClipboardDocumentIcon,
  ClockIcon,
  Cog8ToothIcon,
  ExclamationTriangleIcon,
  GlobeAltIcon,
  LockClosedIcon,
  QuestionMarkCircleIcon,
  XCircleIcon,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useFullscreen, useTitle } from "@vueuse/core";
import Mousetrap from "mousetrap";
import { computed, ref, watch, watchEffect, type Component, type ComputedRef } from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{
  owner: string;
  project: string;
}>();

const projectNavigation = [{ name: "Rename", href: "#" }];

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

provideAction({
  id: "editor.view.openExplorer",
  label: "View Explorer",
  shortcuts: ["alt+1"],
  apply: () => editor.setActiveView("explorer"),
});
provideAction({
  id: "editor.view.openHistory",
  label: "View History",
  shortcuts: ["alt+2"],
  apply: () => editor.setActiveView("history"),
});
const openIssues = provideAction({
  id: "editor.view.openIssues",
  label: "View Issues",
  shortcuts: ["alt+3"],
  apply: () => editor.setActiveView("issues"),
});

// get project header
const { error: projectError, result: projectResult } = useQuery(
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

// default version to view = head (will be overridden by URL)
const versionToViewId = computed(() => project.value?.head.id);

// sync title bar with project info
const title = useTitle();
watchEffect(() => (title.value = `${props.owner}/${props.project}${project.value ? ": " + project.value.name : ""}`));

// get project content
const { error: versionError, result: versionResult } = useQuery(
  graphql(/* GraphQL */ `
    query projectVersionContent($id: GlobalID!) {
      projectVersion(id: $id) {
        id
        id
        name
        description
        createdAt
        committed
        committedAt
        files(filters: { isVisible: true }) {
          id
          ...FileHeader
        }
      }
    }
  `),
  () => ({ id: versionToViewId.value }),
  () => ({ enabled: !!versionToViewId.value })
);
const version = computed(() => versionResult.value?.projectVersion);
const versionLoaded = computed(() => !!version.value);
// filter deletedAt to increase responsiveness
const files = computed(
  () => version.value?.files.map((f) => useFragment(FileHeaderType, f)).filter((f) => f.deletedAt == null) || []
);

// actions (ensure global actions are available)
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const actions = useActions();
const operationsStore = useOperationsStore();
const { connected: runtimeConnected, lastUpdated: runtimeLastUpdated } = useCurrentModuleRuntime();
const hasStaleInflightStateOps = computed(() => operationsStore.hasInflightLike({ stateless: false, stale: true }));
const { getTimeFromNowString } = useTimeFromNow();

// set up editor state
const editor = useEditorState();

// sync editor paths
watchEffect(() => {
  if (!files.value) return;
  editor.editors.forEach((editor) => {
    if (editor.type == "file") {
      const fileEditor = editor as FileEditor;
      const file = files.value.find((f) => f.id == fileEditor.fileId);
      if (!file) return; // ignore
      editor.path = file.path + ".x";
    }
  });
});

// routing
const router = useRouter();

// focus file from url if hash changes and none is open
watchEffect(() => {
  const hash = router.currentRoute.value.hash;
  if (hash && files.value) {
    const path = hash.slice(1).slice(0, -"x".length - 1);
    const file = files.value.find((file) => file.path === path);
    if (file && editor.focusedEditor == null) {
      editor.focusFile(file);
    }
  }
});

// change url if focused editor changes
watchEffect(() => {
  if (editor.focusedEditor) {
    router.replace({ hash: `#${editor.focusedEditor.path}` });
  }
});

// suppress control+s (offer named commit instead)
const notifications = useNotifications();
Mousetrap.bind(["ctrl+s"], () => {
  notifications.showIf(
    {
      type: "saveSuppressed",
      kind: "notice",
      message: "Saving is automatic",
      description: "All changes are synced automatically.",
      actionText: "Commit",
      action: () => actions.version.commit.value.apply(),
    },
    { lastActiveMs: 60000 }
  );
  return false;
});

const runtime = useCurrentModuleRuntime();

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

// sync fullscreen
const { isFullscreen, enter, exit } = useFullscreen();
watch(
  () => editor.fullscreen,
  () => {
    if (editor.fullscreen && !isFullscreen.value) {
      enter().catch(() => (editor.fullscreen = false));
    } else if (isFullscreen.value) {
      exit();
    }
  }
);
watch(isFullscreen, () => (editor.fullscreen = isFullscreen.value));

const { load } = useEditorPersistence();
const { migrateTo } = useEditorMigrations();

// prepare editor state for project whenever project (head) changes
watchEffect(async () => {
  if (
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
</script>

<template>
  <!-- Root -->
  <div class="relative flex h-full flex-col">
    <!-- Header with controls and auth -->
    <FatHeader v-show="editor.showGlobalHeader">
      <!-- Left side: organizational & status -->
      <template v-slot:left>
        <!-- Home -->
        <HomeButton />
        <!-- Project menu -->
        <div v-if="projectLoaded" class="ml-2 flex flex-row items-baseline gap-0.5">
          <router-link :to="`/${props.owner}`" class="rounded-sm p-1 text-sm hover:bg-orange-50">
            {{ props.owner }}
          </router-link>
          <span class="text-gray-500">/</span>
          <ProjectPopover :project="project">
            <template v-slot:button="{ open }">
              <PopoverButton
                class="flex h-full items-center justify-between rounded-sm bg-white p-1 text-left hover:bg-orange-50 focus:outline-none"
                :class="{ 'bg-orange-50 focus:bg-orange-50': open }"
              >
                <span class="text-sm font-bold">{{ props.project }}</span>
                <FadeTransition mode="out-in">
                  <component
                    :is="project.visibility != ProjectVisibility.Public ? LockClosedIcon : GlobeAltIcon"
                    class="ml-1.5 h-4 w-4 text-gray-700"
                  />
                </FadeTransition>
              </PopoverButton>
            </template>
          </ProjectPopover>
        </div>
        <!-- Status -->
        <div v-if="versionLoaded" class="ml-2 flex items-center">
          <!-- should use nicer icons here -->
          <!-- Operations status -->
          <span class="flex items-center gap-1 p-1 transition-opacity" v-show="hasStaleInflightStateOps">
            <svg
              viewBox="0 0 100 100"
              class="h-1 w-1"
              :class="{ 'text-orange-500': !hasStaleInflightStateOps, 'text-gray-400': hasStaleInflightStateOps }"
            >
              <circle cx="50" cy="50" r="40" fill="currentColor" />
            </svg>
            <span class="text-sm text-gray-500">saving</span>
          </span>
          <!-- Runtime status -->
          <span class="flex items-center gap-1 p-1 transition-all">
            <svg
              viewBox="0 0 100 100"
              class="h-1 w-1"
              :class="{ 'text-orange-500': runtimeConnected, 'text-gray-400': !runtimeConnected }"
            >
              <circle cx="50" cy="50" r="40" fill="currentColor" />
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
            class="flex items-center gap-0.5 rounded-sm p-1 hover:bg-orange-50"
            v-if="runtime.errors.value?.length || 0 > 0"
            @click="openIssues.apply"
          >
            <XCircleIcon class="h-5 w-5 text-red-700" />
            <span class="text-sm text-gray-700">{{ runtime.errors.value?.length }}</span>
          </button>
        </div>
        <!-- Current worker tasks -->
        <!-- ... -->
      </template>

      <!-- Right side: controls & profile -->
      <template v-slot:right>
        <!-- Current "main" statement controls -->
        <div v-if="versionLoaded" class="flex h-full items-center space-x-2 px-3">
          <MainSymbolControls />
        </div>
        <!-- Bench-global controls -->
        <div v-if="versionLoaded" class="flex h-full items-center space-x-2 pl-3">
          <GlobalControls />
        </div>
        <OmniCreate class="pl-1" />
        <ProfileButton class="pl-1" />
      </template>
    </FatHeader>
    <!-- Main content (sidebar + editor), spans horizontally -->
    <div v-show="projectLoaded" class="relative flex flex-1 flex-row">
      <!-- Sidebar of view buttons & views -->
      <aside
        class="flex h-full resize-x border-r border-gray-200"
        :class="{
          'w-64 lg:w-80': editor.showViewContent && editor.showViewSelection,
          'w-48 lg:w-64': editor.showViewContent && !editor.showViewSelection,
        }"
      >
        <!-- View selection -->
        <div class="flex h-full min-h-0 flex-col border-r border-gray-200 p-1.5" v-show="editor.showViewSelection">
          <div class="flex flex-1 flex-col">
            <button
              class="rounded-sm px-2 py-2 text-gray-600"
              :class="view.name == activeView.name ? 'bg-orange-100 text-orange-900' : 'hover:bg-gray-100'"
              v-for="view in views"
              :key="view.name"
              @click="editor.setActiveView(view.id)"
            >
              <span class="sr-only">{{ view.name }}</span>
              <component :is="view.icon" class="h-6 w-6" aria-hidden="true" />
            </button>
          </div>
          <!-- Help & settings -->
          <router-link
            to="/symbolx/examples"
            target="_blank"
            class="rounded-sm px-2 py-2 text-gray-600 hover:bg-gray-100"
          >
            <span class="sr-only">Help</span>
            <QuestionMarkCircleIcon class="h-6 w-6" aria-hidden="true" />
          </router-link>
          <SettingsPopover>
            <template v-slot:button="{ open }">
              <PopoverButton
                class="rounded-sm px-2 py-2 text-gray-600 outline-none hover:bg-gray-100 focus:ring-0"
                :class="open ? 'bg-gray-100' : ''"
              >
                <span class="sr-only">Settings</span>
                <Cog8ToothIcon class="h-6 w-6" aria-hidden="true" />
              </PopoverButton>
            </template>
          </SettingsPopover>
        </div>
        <!-- View content -->
        <div class="relative flex-1 flex-col" v-show="editor.showViewContent">
          <div class="absolute top-0 left-0 h-full w-full overflow-y-hidden">
            <ViewExplorer v-show="activeView.id == 'explorer'" :files="files" v-if="files" />
            <ViewHistory
              v-show="activeView.id == 'history'"
              v-if="project != null && version != null"
              :project="project as any"
              :current-version="version as any"
            />
            <ViewIssues v-show="activeView.id == 'issues'" />
          </div>
        </div>
      </aside>
      <!-- Main editor area -->
      <main class="flex h-full w-full flex-1 divide-x divide-gray-200 bg-gray-50">
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
    <GenericNotFound v-if="projectError" class="pb-12" />
    <NotificationArea />
  </div>
</template>
