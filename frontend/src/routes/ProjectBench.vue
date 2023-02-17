<script setup lang="ts">
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import ProfileMenuButton from "@/components/basic/ProfileMenuButton.vue";
import NotificationArea from "@/components/container/NotificationArea.vue";
import EditorGroupInterface from "@/components/EditorGroupInterface.vue";
import GlobalControls from "@/components/GlobalControls.vue";
import MainSymbolControls from "@/components/MainSymbolControls.vue";
import ViewExplorer from "@/components/panels/ViewExplorer.vue";
import ViewHistory from "@/components/panels/ViewHistory.vue";
import ViewIssues from "@/components/panels/ViewIssues.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { provideAction, useActions } from "@/state/actions";
import { useEditorPersistence, useEditorState, type FileEditor } from "@/state/editor";
import {
  FileHeaderType,
  ProjectHeaderType,
  ProjectVersionContentType,
  ProjectVersionHeaderType,
} from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperationsStore } from "@/state/operations";
import { useCurrentModuleRuntime } from "@/state/runtime";
import { WS_CONNECTED } from "@/utils/globals";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { ChevronDownIcon } from "@heroicons/vue/20/solid";
import {
  ClipboardDocumentIcon,
  ClockIcon,
  Cog8ToothIcon,
  ExclamationTriangleIcon,
  QuestionMarkCircleIcon,
  XCircleIcon,
} from "@heroicons/vue/24/outline";
import { useLazyQuery, useQuery } from "@vue/apollo-composable";
import { useRefHistory, useTitle } from "@vueuse/core";
import Mousetrap from "mousetrap";
import { computed, ref, watch, watchEffect, type Component, type ComputedRef } from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{
  organization: string;
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
const { result: projectHeaderQuery } = useQuery(
  graphql(/* GraphQL */ `
    query projectBySlug($organization: String!, $project: String!) {
      projectBySlug(organization: $organization, project: $project) {
        ...ProjectHeader
      }
    }
  `),
  () => ({
    organization: props.organization,
    project: props.project,
  })
);
const projectHeader = computed(() => useFragment(ProjectHeaderType, projectHeaderQuery.value?.projectBySlug));
const projectHead = computed(() => useFragment(ProjectVersionHeaderType, projectHeader.value?.head));

// sync title bar with project head
const title = useTitle();
watchEffect(
  () =>
    (title.value = `${props.organization}/${props.project}${
      projectHeader.value ? ": " + projectHeader.value.name : ""
    }`)
);

// get project content
const { result: contentQuery } = useQuery(
  graphql(/* GraphQL */ `
    query projectVersionContent($id: GlobalID!) {
      projectVersion(id: $id) {
        id
        ...ProjectVersionContent
      }
    }
  `),
  () => ({ id: projectHead.value?.id }),
  () => ({ enabled: !!projectHead.value?.id })
);
const content = computed(() => useFragment(ProjectVersionContentType, contentQuery.value?.projectVersion));
// filter deletedAt to increase responsiveness
const files = computed(
  () => content.value?.files.map((f) => useFragment(FileHeaderType, f)).filter((f) => f.deletedAt == null) || []
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

// router sync
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

const { load } = useEditorPersistence();

// migration logic on version change
const migrating = ref(false);
const {
  load: getProjectMigrationRefs,
  loading: projectMigrationLoading,
  error: projectMigrationError,
  result: projectMigrationRefs,
} = useLazyQuery(
  graphql(/* GraphQL */ `
    query projectMigrationRefs($projectId: GlobalID!, $afterId: GlobalID!) {
      project(id: $projectId) {
        versions(filters: { afterId: $afterId }) {
          id
          name
          createdAt
          parentsRefs {
            source
            target
          }
        }
      }
    }
  `)
);

// the second half of applying a migration (since we can't await lazy queries directly)
watch(
  () => [projectMigrationRefs, projectMigrationLoading, projectMigrationError],
  async () => {
    if (!migrating.value) return;
    if (!projectHead.value) {
      // shouldn't happen but cancel migration if it does
      migrating.value = false;
      return;
    }
    if (projectMigrationLoading.value) return;

    if (projectMigrationError.value != null) {
      await editor.migrateTo(projectHead.value, undefined);
      console.error("unable to migrate, error getting intermediate refs", projectMigrationError.value);
      migrating.value = false;
      notifications.show({
        kind: "warning",
        type: "editorMigration.fail",
        message: "Migration failed",
        description: "Editor state could not be migrated.",
      });
    } else if (projectMigrationRefs.value) {
      const intermediateVersions = [...(projectMigrationRefs.value?.project?.versions ?? [])];
      const intermediateRefs = intermediateVersions
        ?.sort((a, b) => a.createdAt - b.createdAt)
        .map((v) => v.parentsRefs);
      await editor.migrateTo(projectHead.value, intermediateRefs);
      console.log(`migrated through ${intermediateVersions?.map((v) => v.id)} intermediate versions`);
      migrating.value = false;
      notifications.show({
        kind: "success",
        type: "editorMigration.success",
        message: "Migrated",
        description: "Editor state has been migrated.",
      });
    }
  },
  { deep: true }
);

// reset editor state for project if project (head) changes
watchEffect(async () => {
  const loaded = projectHeader.value != null && projectHead.value != null && content.value != null;
  if (
    loaded &&
    (editor.currentProjectId != projectHeader.value.id || editor.currentProjectVersionId != projectHead.value.id)
  ) {
    // try to load editor state
    editor.setProject(projectHeader.value, projectHead.value);
    load();
    console.log(`loaded editor state for project ${projectHeader.value.id} version ${projectHead.value.id}`);
    if (editor.currentProjectId == projectHeader.value?.id) {
      // migrate if there is a new version
      if (editor.currentProjectVersionId != projectHead.value?.id) {
        console.log(`migrate editor state for project ${projectHeader.value.id} to version ${projectHead.value.id}`);
        migrating.value = true;
        // get all ref mappings
        getProjectMigrationRefs(
          undefined,
          {
            projectId: projectHeader.value.id,
            afterId: editor.currentProjectVersionId,
          },
          { fetchPolicy: "network-only" }
        );
        // migrating flag triggers migration completion above
      }
    } else {
      console.log(`reset editor state for project ${projectHeader.value.id}`);
      // (happens in state.setProject)
    }
  }
});
</script>

<template>
  <!-- Root -->
  <div class="relative flex h-full flex-col">
    <!-- Header with controls and auth -->
    <FatHeader>
      <!-- Left side: organizational & status -->
      <template v-slot:left>
        <!-- Home -->
        <HomeButton />
        <!-- Project menu -->
        <Menu as="div" class="relative h-full flex-shrink-0 border-l border-r border-gray-200">
          <div class="h-full">
            <MenuButton
              class="flex h-full items-center justify-between bg-white px-4 text-left hover:bg-gray-50 focus:bg-gray-100 focus:outline-none"
            >
              <span class="sr-only">Open project menu</span>
              <span class="text-sm">
                {{ organization }}
                /
                <span class="font-bold">{{ project }}</span>
              </span>
              <ChevronDownIcon class="ml-2 -mr-1 h-5 w-5 text-gray-300" aria-hidden="true" />
            </MenuButton>
          </div>
          <FadeTransition>
            <MenuItems
              class="absolute left-0 z-10 mt-0 w-48 origin-top-left rounded-sm bg-white px-1 py-1 shadow-md ring-1 ring-black ring-opacity-5 focus:outline-none"
            >
              <MenuItem v-for="item in projectNavigation" :key="item.name" v-slot="{ active }">
                <a :href="item.href" :class="[active ? 'bg-gray-100' : '', 'block py-2 px-4 text-sm text-gray-700']">
                  {{ item.name }}
                </a>
              </MenuItem>
            </MenuItems>
          </FadeTransition>
        </Menu>
        <!-- Status -->
        <div class="ml-2 flex items-center">
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
            <span class="text-sm text-gray-500" v-show="!runtimeConnected">connecting</span>
            <span class="text-sm text-gray-500" v-if="editor.debug && runtimeLastUpdated != null">
              {{ getTimeFromNowString(runtimeLastUpdated) }}
            </span>
          </span>
        </div>
        <!-- Comments/notes, issues/warnings/lints, errors -->
        <div class="ml-2 flex items-center gap-2">
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
        <div class="flex h-full items-center space-x-2 border-r border-gray-200 px-3">
          <MainSymbolControls />
        </div>
        <!-- Global controls -->
        <div class="flex h-full items-center space-x-2 border-r border-gray-200 px-3">
          <GlobalControls />
        </div>
        <!-- Profile -->
        <ProfileMenuButton />
      </template>
    </FatHeader>
    <!-- Main content (sidebar + editor), spans horizontally -->
    <div class="relative flex flex-1 flex-row">
      <!-- Sidebar of view buttons & views -->
      <aside class="flex h-full w-64 resize-x border-r border-gray-200 lg:w-80">
        <!-- View selection -->
        <div class="flex h-full min-h-0 flex-col border-r border-gray-200 p-1.5">
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
          <button class="rounded-sm px-2 py-2 text-gray-600 hover:bg-gray-100">
            <span class="sr-only">Help</span>
            <QuestionMarkCircleIcon class="h-6 w-6" aria-hidden="true" />
          </button>
          <button class="rounded-sm px-2 py-2 text-gray-600 hover:bg-gray-100">
            <span class="sr-only">Settings</span>
            <Cog8ToothIcon class="h-6 w-6" aria-hidden="true" />
          </button>
        </div>
        <!-- View content -->
        <div class="relative flex-1 flex-col">
          <div class="absolute top-0 left-0 h-full w-full overflow-y-hidden">
            <ViewExplorer v-show="activeView.id == 'explorer'" :files="files" v-if="files" />
            <ViewHistory
              v-show="activeView.id == 'history'"
              v-if="projectHeader && projectHead"
              :project="projectHeader"
              :current-version="projectHead"
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
    <NotificationArea />
  </div>
</template>
