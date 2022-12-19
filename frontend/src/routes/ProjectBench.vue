<script setup lang="ts">
import EditorGroupInterface from "@/components/EditorGroupInterface.vue";
import ViewExplorer from "@/components/ViewExplorer.vue";
import ViewHistory from "@/components/ViewHistory.vue";
import { graphql, useFragment } from "@/gql";
import { provideAction, useActions } from "@/utils/actions";
import { useEditorPersistence, useEditorState, type FileEditor } from "@/utils/editor";
import { CompilationHeaderType, FileHeaderType, ProjectHeaderType, ProjectVersionHeaderType } from "@/utils/fragments";
import { useOperations, useOperationsStore } from "@/utils/operations";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { ChevronDownIcon } from "@heroicons/vue/20/solid";
import {
  ClipboardDocumentIcon,
  ClockIcon,
  Cog8ToothIcon,
  PlayIcon,
  QuestionMarkCircleIcon,
  WrenchIcon,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, watchEffect, type Component, type ComputedRef } from "vue";
import { useRouter } from "vue-router";

const props = defineProps<{
  organization: string;
  project: string;
}>();

// fake data
const user = {
  name: "Florian Cäsar",
  email: "yatima@symbolx.com",
};
const projectNavigation = [{ name: "Rename", href: "#" }];
const userNavigation = [
  { name: "Settings", href: "#" },
  { name: "Sign out", href: "#" },
];

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

// real data
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

const ProjectVersionContent = graphql(/* GraphQL */ `
  fragment ProjectVersionContent on ProjectVersion {
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
    compilations {
      id
      ...CompilationHeader
    }
  }
`);

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
const content = computed(() => useFragment(ProjectVersionContent, contentQuery.value?.projectVersion));
const files = computed(
  () => content.value?.files.map((f) => useFragment(FileHeaderType, f)).filter((f) => f.deletedAt == null) || []
); // filter deletedAt to increase responsiveness
const compilations = computed(
  () => content.value?.compilations.map((c) => useFragment(CompilationHeaderType, c)) || []
);

// actions (ensure global actions are available)
const actions = useActions();
const operationsStore = useOperationsStore();
const anyInflightOps = computed(() => operationsStore.hasInflight);
const operations = useOperations();

// compile
const isCompiling = computed(() => operationsStore.hasInflightLike("compilation.compile"));
const canCompile = computed(() => !isCompiling.value && compilations.value?.length > 0);

async function compileAll() {
  await Promise.all(compilations.value.map((c) => operations.compilation.compile(c.id)));
}

provideAction({
  id: "compilation.compileAll",
  label: "Compile all",
  shortcuts: [],
  enabled: canCompile,
  apply: () => compileAll,
});

async function createDefaultCompilation() {
  throw new Error("not implemented");
}

const compileNavigation = computed(() => [
  { name: "Compile all", action: compileAll, disabled: !canCompile.value },
  { name: "Compile optimized", action: compileAll, disabled: !canCompile.value },
  { name: "Add default target", action: createDefaultCompilation },
]);

// run
const canRun = false;

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
      if (state.currentProjectVersionId != projectHead.value?.id) {
        console.log(`migrate editor state for project ${projectHeader.value.id} to version ${projectHead.value.id}`);
        await state.migrateTo(projectHead.value);
      } // otherwise no migration needed
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
    <header class="static mx-auto w-full flex-shrink-0 overflow-y-visible border-b border-gray-200 bg-white shadow-sm">
      <div class="relative flex justify-between gap-8">
        <!-- Left side: organizational & status -->
        <div class="static flex items-center">
          <!-- Home -->
          <div class="flex flex-shrink-0 items-center px-4 py-2 hover:bg-gray-50">
            <a href="#">
              <svg viewBox="0 0 100 100" class="h-8 w-8 text-orange-600">
                <!-- A workbench -->
                <path
                  d="M 50 0 L 100 25 L 100 75 L 50 100 L 0 75 L 0 25 Z"
                  fill="currentColor"
                  stroke="currentColor"
                  stroke-width="2"
                />
                <!-- With an X across edge to edge -->
                <path d="M 0 20 L 100 80" stroke="white" stroke-width="6" />
                <path d="M 100 20 L 0 80" stroke="white" stroke-width="6" />
              </svg>
            </a>
          </div>
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
            <transition
              enter-active-class="transition duration-100 ease-out"
              enter-from-class="transform opacity-0"
              enter-to-class="transform opacity-100"
              leave-active-class="transition duration-75 ease-in"
              leave-from-class="transform opacity-100"
              leave-to-class="transform opacity-0"
            >
              <MenuItems
                class="absolute left-0 z-10 mt-0 w-48 origin-top-left rounded-sm bg-white px-1 py-1 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
              >
                <MenuItem v-for="item in projectNavigation" :key="item.name" v-slot="{ active }">
                  <a :href="item.href" :class="[active ? 'bg-gray-100' : '', 'block py-2 px-4 text-sm text-gray-700']">
                    {{ item.name }}
                  </a>
                </MenuItem>
              </MenuItems>
            </transition>
          </Menu>
          <!-- Status -->
          <div class="ml-2 flex items-center">
            <!-- Sync indicator -->
            <span class="p-1 transition-all">
              <svg
                viewBox="0 0 100 100"
                class="h-1 w-1"
                :class="{ 'text-gray-400': anyInflightOps, 'text-orange-400': !anyInflightOps }"
              >
                <circle cx="50" cy="50" r="40" fill="currentColor" />
              </svg>
            </span>
          </div>
        </div>
        <!-- Right side: controls (and profile) -->
        <div class="flex min-w-fit flex-shrink-0 items-center justify-end">
          <!-- Controls -->
          <div class="flex h-full items-center space-x-2 border-r border-gray-200 px-3">
            <!-- Compile menu -->
            <div class="flex flex-row">
              <button
                :class="[
                  'group inline-flex items-center justify-center rounded-l-sm py-2 px-3 text-sm font-semibold focus:outline-none',
                  'bg-orange-600 text-white hover:bg-orange-700 hover:text-slate-100',
                  !canCompile ? 'cursor-not-allowed opacity-50' : '',
                ]"
                :disabled="!canCompile"
              >
                <WrenchIcon class="h-5 w-5" aria-hidden="true" />
                <span class="ml-1" @click="compileAll">Compile</span>
              </button>
              <Menu as="div" class="relative h-full flex-shrink-0">
                <MenuButton
                  :class="[
                    'flex h-full rounded-r-sm px-2 py-2 text-left',
                    'bg-orange-600 text-white hover:bg-orange-700 hover:text-slate-100',
                    'border-l border-orange-200',
                  ]"
                >
                  <ChevronDownIcon class="h-5 w-5" aria-hidden="true" />
                </MenuButton>
                <transition
                  enter-active-class="transition duration-100 ease-out"
                  enter-from-class="transform opacity-0"
                  enter-to-class="transform opacity-100"
                  leave-active-class="transition duration-75 ease-in"
                  leave-from-class="transform opacity-100"
                  leave-to-class="transform opacity-0"
                >
                  <MenuItems
                    class="absolute right-0 z-10 mt-0 w-48 origin-top-right rounded-sm bg-white px-1 py-1 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
                  >
                    <MenuItem v-for="item in compileNavigation" :key="item.name" v-slot="{ active }">
                      <button
                        :class="[
                          active ? 'bg-gray-100' : '',
                          item.disabled ? 'cursor-not-allowed text-gray-500' : 'text-gray-700',
                          'w-full py-2 px-4 text-left text-sm',
                        ]"
                        @click="item.action"
                        :disabled="item.disabled"
                      >
                        {{ item.name }}
                      </button>
                    </MenuItem>
                  </MenuItems>
                </transition>
              </Menu>
            </div>
            <!-- Run menu -->
            <button
              :class="[
                'group inline-flex items-center justify-center rounded-sm py-2 px-3 text-sm font-semibold focus:outline-none',
                'bg-orange-600 text-white hover:bg-orange-700 hover:text-slate-100',
                !canRun ? 'cursor-not-allowed opacity-50' : '',
              ]"
              :disabled="!canRun"
            >
              <PlayIcon class="h-5 w-5" aria-hidden="true" />
              <span class="ml-1">Run</span>
            </button>
          </div>

          <!-- Profile dropdown -->
          <Menu as="div" class="relative flex-shrink-0">
            <div>
              <MenuButton
                class="flex flex-col bg-white px-4 py-2 text-left hover:bg-gray-50 focus:bg-gray-100 focus:outline-none"
              >
                <span class="sr-only">Open user menu</span>
                <span class="text-xs font-bold text-gray-900">{{ user.name }}</span>
                <span class="text-xs text-gray-500">Personal</span>
              </MenuButton>
            </div>
            <transition
              enter-active-class="transition duration-100 ease-out"
              enter-from-class="transform opacity-0"
              enter-to-class="transform opacity-100"
              leave-active-class="transition duration-75 ease-in"
              leave-from-class="transform opacity-100"
              leave-to-class="transform opacity-0"
            >
              <MenuItems
                class="absolute right-0 z-10 mt-0 w-48 origin-top-right rounded-sm bg-white px-1 py-1 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
              >
                <MenuItem v-for="item in userNavigation" :key="item.name" v-slot="{ active }">
                  <a :href="item.href" :class="[active ? 'bg-gray-100' : '', 'block py-2 px-4 text-sm text-gray-700']">
                    {{ item.name }}
                  </a>
                </MenuItem>
              </MenuItems>
            </transition>
          </Menu>
        </div>
      </div>
    </header>
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
          <div class="absolute left-0 top-0 h-full w-full overflow-y-hidden">
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
          <div class="absolute left-0 top-0 h-full w-full overflow-hidden">
            <EditorGroupInterface :group="state.left" class="h-full w-full" />
          </div>
        </div>
        <!-- Right editor group -->
        <div class="relative flex-1" v-if="state.right.editors.length > 0">
          <div class="absolute left-0 top-0 h-full w-full overflow-hidden">
            <EditorGroupInterface :group="state.right" class="h-full w-full" />
          </div>
        </div>
      </main>
    </div>
  </div>
</template>
