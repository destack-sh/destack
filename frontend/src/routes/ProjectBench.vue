<script setup lang="ts">
import FadeTransition from "@/components/basic/FadeTransition.vue";
import FatHeader from "@/components/basic/FatHeader.vue";
import HomeButton from "@/components/basic/HomeButton.vue";
import ProfileMenuButton from "@/components/basic/ProfileMenuButton.vue";
import EditorGroupInterface from "@/components/EditorGroupInterface.vue";
import ViewExplorer from "@/components/ViewExplorer.vue";
import ViewHistory from "@/components/ViewHistory.vue";
import { graphql, useFragment } from "@/gql";
import { provideAction, useActions } from "@/state/actions";
import { useEditorPersistence, useEditorState, type FileEditor } from "@/state/editor";
import {
  FileHeaderType,
  ProjectHeaderType,
  ProjectVersionContentType,
  ProjectVersionHeaderType,
} from "@/state/fragments";
import { useOperationsStore } from "@/state/operations";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { ChevronDownIcon } from "@heroicons/vue/20/solid";
import { ClipboardDocumentIcon, ClockIcon, Cog8ToothIcon, QuestionMarkCircleIcon } from "@heroicons/vue/24/outline";
import { useLazyQuery, useQuery } from "@vue/apollo-composable";
import { useTitle } from "@vueuse/core";
import { computed, ref, watch, watchEffect, type Component, type ComputedRef } from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{
  organization: string;
  project: string;
}>();

const projectNavigation = [{ name: "Rename", href: "#" }];

// views for the sidebar
type View = {
  id: "explorer" | "history";
  name: string;
  icon: Component;
};
const views: View[] = [
  { id: "explorer", name: "Explorer", icon: ClipboardDocumentIcon },
  { id: "history", name: "History", icon: ClockIcon },
];
const activeView: ComputedRef<View> = computed(() => {
  const view = views.find((v) => v.id == state.activeViewId);
  if (!view) {
    console.error("invalid view id: " + state.activeViewId);
    state.setActiveView(views[0].id);
    return views[0];
  }
  return view;
});

provideAction({
  id: "editor.view.openExplorer",
  label: "View Explorer",
  shortcuts: ["alt+1"],
  apply: () => state.setActiveView("explorer"),
});
provideAction({
  id: "editor.view.openHistory",
  label: "View History",
  shortcuts: ["alt+2"],
  apply: () => state.setActiveView("history"),
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
const anyInflightOps = computed(() => operationsStore.hasInflight);

// set up editor state
const state = useEditorState();

// editors paths sync
// TODO @Cleanup:
watchEffect(() => {
  if (!files.value) return;
  state.editors.forEach((editor) => {
    if (editor.type == "file") {
      const fileEditor = editor as FileEditor;
      const file = files.value.find((f) => f.id == fileEditor.fileId);
      if (!file) return; // ignore
      editor.path = file.path;
    }
  });
});

// router sync
const router = useRouter();
// focus file from url if hash changes and none is open
watchEffect(() => {
  const hash = router.currentRoute.value.hash;
  if (hash && files.value) {
    const path = hash.slice(1).slice(0, -"instruct".length - 1);
    const file = files.value.find((file) => file.path === path);
    if (file && state.focusedEditor == null) {
      state.focusFile(file);
    }
  }
});

// change url if focused editor changes
watchEffect(() => {
  if (state.focusedEditor) {
    router.replace({ hash: `#${state.focusedEditor.path}` });
  }
});

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
      console.error("unable to migrate, error getting intermediate refs", projectMigrationError.value);
      await state.migrateTo(projectHead.value, undefined);
      migrating.value = false;
    } else if (projectMigrationRefs.value) {
      const intermediateVersions = [...(projectMigrationRefs.value?.project?.versions ?? [])];
      const intermediateRefs = intermediateVersions
        ?.sort((a, b) => a.createdAt - b.createdAt)
        .map((v) => v.parentsRefs);
      await state.migrateTo(projectHead.value, intermediateRefs);
      console.log(`migrated through ${intermediateVersions?.map((v) => v.id)} intermediate versions`);
      migrating.value = false;
    }
  },
  { deep: true }
);

// reset editor state for project if project (head) changes
watchEffect(async () => {
  const loaded = projectHeader.value != null && projectHead.value != null && content.value != null;
  if (
    loaded &&
    (state.currentProjectId != projectHeader.value.id || state.currentProjectVersionId != projectHead.value.id)
  ) {
    // try to load editor state
    state.setProject(projectHeader.value, projectHead.value);
    load();
    console.log(`loaded editor state for project ${projectHeader.value.id} version ${projectHead.value.id}`);
    if (state.currentProjectId == projectHeader.value?.id) {
      // migrate if there is a new version
      if (state.currentProjectVersionId != projectHead.value?.id) {
        console.log(`migrate editor state for project ${projectHeader.value.id} to version ${projectHead.value.id}`);
        migrating.value = true;
        // get all ref mappings
        getProjectMigrationRefs(
          undefined,
          {
            projectId: projectHeader.value.id,
            afterId: state.currentProjectVersionId,
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
  <div class="flex h-full flex-col">
    <!-- Header with controls and auth -->
    <FatHeader>
      <!-- Left side: organizational & status -->
      <template v-slot:left>
        <!-- Home -->
        <HomeButton />
        <!-- Current project menu -->
        <Menu as="div" class="relative h-full flex-shrink-0 border-l border-r border-gray-200">
          <div class="h-full">
            <MenuButton
              class="flex h-full items-center justify-between bg-white px-4 py-2 text-left hover:bg-gray-50 focus:bg-gray-100 focus:outline-none"
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
              class="absolute left-0 z-10 mt-0 w-48 origin-top-left rounded-sm bg-white px-1 py-1 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
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
          <span class="p-1 transition-all" v-if="state.debug">
            <svg
              viewBox="0 0 100 100"
              class="h-1 w-1"
              :class="{ 'text-gray-400': !anyInflightOps, 'text-orange-400': anyInflightOps }"
            >
              <circle cx="50" cy="50" r="40" fill="currentColor" />
            </svg>
          </span>
        </div>
      </template>
      <!-- Right side: controls & profile -->
      <template v-slot:right>
        <!-- Controls -->
        <div class="flex h-full items-center space-x-2 border-r border-gray-200 px-3">
          <!-- Compile -->

          <!-- Run (and compile deps if needed) -->

          <!-- Deploy run(s) -->

          <!-- Share -->
        </div>

        <!-- Profile dropdown -->
        <ProfileMenuButton />
      </template>
    </FatHeader>
    <!-- Main content (sidebar + editor), spans horizontally -->
    <div class="relative flex flex-1 flex-row">
      <!-- Sidebar of view buttons & views -->
      <aside class="flex h-full w-64 resize-x border-r border-gray-200 lg:w-72">
        <!-- View selection -->
        <div class="flex h-full min-h-0 flex-col border-r border-gray-200 p-1.5">
          <div class="flex flex-1 flex-col">
            <button
              class="rounded-sm px-2 py-2 text-gray-600"
              :class="view.name == activeView.name ? 'bg-orange-100 text-orange-900' : 'hover:bg-gray-100'"
              v-for="view in views"
              :key="view.name"
              @click="state.setActiveView(view.id)"
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
          </div>
        </div>
      </aside>
      <!-- Main editor area -->
      <main class="flex h-full w-full flex-1 divide-x divide-gray-200 bg-gray-50">
        <!-- Left editor group -->
        <div class="relative flex-1">
          <div class="absolute top-0 left-0 h-full w-full overflow-hidden">
            <EditorGroupInterface :group="state.left" class="h-full w-full" />
          </div>
        </div>
        <!-- Right editor group -->
        <div class="relative flex-1" v-if="state.right.editors.length > 0">
          <div class="absolute top-0 left-0 h-full w-full overflow-hidden">
            <EditorGroupInterface :group="state.right" class="h-full w-full" />
          </div>
        </div>
      </main>
    </div>
  </div>
</template>
