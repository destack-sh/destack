<script setup lang="ts">
import SButton from "@/components/basic/SButton.vue";
import FileInterface from "@/components/FileInterface.vue";
import ViewExplorer from "@/components/ViewExplorer.vue";
import ViewVersionHistory from "@/components/ViewVersionHistory.vue";
import { graphql, useFragment } from "@/gql";
import { EDITOR_STATE_KEY, type EditorState, type FileHeader, type SymbolDefinitionHeader } from "@/utils/editor";
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
import { computed, provide, ref, watchEffect, type Ref } from "vue";
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
  icon: any;
};
const views: View[] = [
  { id: "explorer", name: "Explorer", icon: ClipboardDocumentIcon },
  { id: "version-history", name: "Versions", icon: ClockIcon },
];
const activeView: Ref<View> = ref(views[0]);

const { mutate: compileTask } = useMutation(
  graphql(/* GraphQL */ `
    mutation compileTask($compilationId: UUID!) {
      compile(compilationId: $compilationId)
    }
  `)
);

// real data
const ProjectVersionHeader = graphql(/* GraphQL */ `
  fragment ProjectVersionHeader on ProjectVersion {
    name
    description
    createdAt
    committed
    committedAt
  }
`);

const { result: projectId } = useQuery(
  graphql(/* GraphQL */ `
    query getProjectBySlug($organization: String!, $project: String!) {
      projectBySlug(organization: $organization, project: $project) {
        id
      }
    }
  `),
  () => ({
    organization: props.organization,
    project: props.project,
  })
);

const { result: versionsQuery } = useQuery(
  graphql(/* GraphQL */ `
    query getProjectVersions($id: GlobalID!) {
      project(id: $id) {
        id
        name
        slug
        head {
          id
          ...ProjectVersionHeader
        }
        versions {
          id
          ...ProjectVersionHeader
        }
      }
    }
  `),
  () => ({ id: projectId.value?.projectBySlug?.id }),
  () => ({ enabled: !!projectId.value?.projectBySlug?.id })
);

const ProjectVersionContent = graphql(/* GraphQL */ `
  fragment ProjectVersionContent on ProjectVersion {
    id
    name
    description
    createdAt
    committed
    committedAt
    # mainProgram {
    #   id
    #   name
    #   type
    #   nameDotType
    #   content {
    #     ...TaskContent
    #   }
    # }
    files {
      id
      ...FileHeader
    }
  }
`);

const { result: contentQuery } = useQuery(
  graphql(/* GraphQL */ `
    query getProjectVersionContent($id: GlobalID!) {
      projectVersion(id: $id) {
        id
        ...ProjectVersionContent
      }
    }
  `),
  () => ({ id: versionsQuery.value?.project?.head?.id }),
  () => ({ enabled: !!versionsQuery.value?.project?.head?.id })
);
const content = computed(() => useFragment(ProjectVersionContent, contentQuery.value?.projectVersion));

const projectWithVersions = computed(() => versionsQuery.value?.project);
const files = computed(() => content.value?.files || []);

// set up editor state
const router = useRouter();
const state: EditorState = {
  focusedFile: ref(null),
  focusedDefinition: ref(null),

  focusFile(file: FileHeader) {
    state.focusedFile.value = file;
    // set router url to current url + #path
    router.replace({ hash: "#" + file.path + ".instruct" });
  },
  focusDefinition(definition: SymbolDefinitionHeader) {
    state.focusedDefinition.value = definition;
  },
};

// focus file from url if hash changes
watchEffect(() => {
  const hash = router.currentRoute.value.hash;
  if (hash && files.value) {
    const path = hash.slice(1).slice(0, -"instruct".length - 1);
    const file = files.value.find((file) => file.path === path);
    if (file) {
      state.focusFile(file);
    }
  }
});

provide(EDITOR_STATE_KEY, state);

// open first file if none is open and there is no hash
watchEffect(() => {
  if (files.value && files.value.length >= 1 && !state.focusedFile.value && !router.currentRoute.value.hash) {
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
              <img
                class="block h-8 w-auto"
                src="https://tailwindui.com/img/logos/mark.svg?color=orange&shade=600"
                alt="Bench"
              />
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
                  <a :href="item.href" :class="[active ? 'bg-gray-50' : '', 'block py-2 px-4 text-sm text-gray-700']">
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
            <SButton text="Build">
              <WrenchIcon class="h-5 w-5" aria-hidden="true" />
              <span class="ml-1" @click="compile">Compile</span>
            </SButton>
            <SButton text="Run">
              <PlayIcon class="h-5 w-5" aria-hidden="true" />
              <span class="ml-1">Run</span>
            </SButton>
          </div>

          <!-- Profile dropdown -->
          <Menu as="div" class="relative flex-shrink-0">
            <div>
              <MenuButton
                class="flex flex-col bg-white px-4 py-2 text-left hover:bg-gray-50 focus:bg-gray-100 focus:outline-none"
              >
                <span class="sr-only">Open user menu</span>
                <span class="text-xs font-bold">{{ user.name }}</span>
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
                  <a :href="item.href" :class="[active ? 'bg-gray-50' : '', 'block py-2 px-4 text-sm text-gray-700']">
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
          <ViewVersionHistory v-else-if="activeView.id == 'version-history'" :project="projectWithVersions" />
        </div>
      </aside>
      <!-- Main editor area -->
      <main class="relative flex h-full w-full flex-1 bg-gray-100">
        <div class="absolute top-0 left-0 h-full w-full flex-1 overflow-y-auto">
          <!-- Editors for each open file -->
          <FileInterface
            v-if="state.focusedFile.value"
            :file="state.focusedFile.value"
            :key="state.focusedFile.value.id"
          />
        </div>
      </main>
    </div>
  </div>
</template>
