<script setup lang="ts">
import EditorGroupInterface from "@/components/EditorGroupInterface.vue";
import ViewExplorer from "@/components/ViewExplorer.vue";
import ViewVersionHistory from "@/components/ViewVersionHistory.vue";
import { graphql, useFragment } from "@/gql";
import { useEditorState } from "@/utils/editor";
import { CompilationHeaderType, FileHeaderType, ProjectHeaderType, ProjectVersionHeaderType } from "@/utils/fragments";
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
import { useMutation, useQuery } from "@vue/apollo-composable";
import { computed, provide, ref, watchEffect, type Component, type Ref } from "vue";
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
  id: "explorer" | "version-history";
  name: string;
  icon: Component;
};
const views: View[] = [
  { id: "explorer", name: "Explorer", icon: ClipboardDocumentIcon },
  { id: "version-history", name: "Versions", icon: ClockIcon },
];
const activeView: Ref<View> = ref(views[0]);

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
    mainProgram {
      id
      name
      type
      nameDotType
    }
    files {
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

const files = computed(() => content.value?.files.map((f) => useFragment(FileHeaderType, f)) || []);
const compilations = computed(
  () => content.value?.compilations.map((c) => useFragment(CompilationHeaderType, c)) || []
);

// mutations
const { mutate: compileTask } = useMutation(
  graphql(/* GraphQL */ `
    mutation compileTask($compilationId: GlobalID!) {
      compile(input: { compilationId: $compilationId }) {
        compilation {
          id
          name
          createdAt
          updatedAt
          targetTask {
            ...TaskContent
          }
          targetCode {
            ...CodeContent
          }
        }
      }
    }
  `),
  // TODO @Performance: don't refetch all file contents post compilation
  //  just update the cache with new source mappings (and remove old ones)
  //  This applies to all mutations, not just this one.
  { refetchQueries: ["projectVersionContent", "fileContentById"] }
);

const isCompiling = ref(false);
const canCompile = computed(() => !isCompiling.value && compilations.value?.length > 0);
const canRun = false;

async function compileAll() {
  isCompiling.value = true;
  console.log("compiling", compilations.value);
  const compilationMutations = compilations.value.map((c) => compileTask({ compilationId: c.id }));
  const compilationPayloads = await Promise.all(compilationMutations);
  console.log("compiled", compilationPayloads);
  isCompiling.value = false;
}

const { mutate: addCompilationTarget } = useMutation(
  graphql(/* GraphQL */ `
    mutation addCompilationTarget($input: AddCompilationInput!) {
      addCompilationTarget(input: $input) {
        compilation {
          id
          name
          createdAt
          updatedAt
        }
      }
    }
  `),
  { refetchQueries: ["projectVersionContent"] }
);
async function createCompilation() {
  if (!content.value?.mainProgram) {
    throw new Error("no main program in current version");
  }

  await addCompilationTarget({
    input: {
      taskDefinitionId: content.value?.mainProgram?.id,
      name: "default",
      backends: ["openai/text-davinci-003"],
    },
  });
}

const compileNavigation = computed(() => [
  { name: "Compile all", action: compileAll, disabled: !canCompile.value },
  { name: "Compile optimized", action: compileAll, disabled: !canCompile.value },
  { name: "Add build target", action: createCompilation },
]);

// set up editor state
const router = useRouter();
const state = useEditorState();

// focus file from url if hash changes and none is open
watchEffect(() => {
  const hash = router.currentRoute.value.hash;
  if (hash && files.value) {
    const path = hash.slice(1).slice(0, -"instruct".length - 1);
    const file = files.value.find((file) => file.path === path);
    if (file && state.focusedFile == null) {
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

// open first file if none is open and there is no hash
watchEffect(() => {
  if (files.value && files.value.length >= 1 && !state.focusedFile && !router.currentRoute.value.hash) {
    state.focusFile(files.value[0]);
  }
});
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- Header with controls and auth -->
    <header class="static mx-auto w-full flex-shrink-0 overflow-y-visible border-b border-gray-200 bg-white shadow-sm">
      <div class="relative flex justify-between gap-8">
        <!-- Left side: organizational -->
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
        </div>
        <!-- Right side: controls (and profile) -->
        <div class="flex items-center justify-end">
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
    <div class="flex flex-1 flex-row">
      <!-- Sidebar of view buttons & views -->
      <aside class="flex h-full w-64 flex-shrink-0 resize-x border-r border-gray-200 lg:w-80">
        <!-- View selection -->
        <div class="flex h-full min-h-0 flex-col border-r border-gray-200 p-1.5">
          <div class="flex flex-1 flex-col">
            <button
              class="rounded-sm px-2 py-2 text-gray-600"
              :class="view.name == activeView.name ? 'bg-orange-100 text-orange-900' : 'hover:bg-gray-100'"
              v-for="view in views"
              :key="view.name"
              @click="activeView = view"
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
        <div class="flex flex-1 flex-col">
          <ViewExplorer v-if="activeView.id == 'explorer'" :files="files" />
          <ViewVersionHistory v-else-if="activeView.id == 'version-history'" :project="projectHeader" />
        </div>
      </aside>
      <!-- Main editor area -->
      <main class="relative flex h-full w-full flex-1 flex-row bg-gray-50">
        <!-- Left editor group -->
        <EditorGroupInterface
          :group="state.left"
          class="absolute top-0 left-0 h-full w-full min-w-[700px] flex-1 overflow-auto"
        />
      </main>
    </div>
  </div>
</template>
