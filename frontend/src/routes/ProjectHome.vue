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
                  <a :href="item.href" :class="[active ? 'bg-gray-50' : '', 'block py-2 px-4 text-sm text-gray-700']">{{
                    item.name
                  }}</a>
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
              <span class="ml-1" @click="compileProgram">Compile</span>
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
                  <a :href="item.href" :class="[active ? 'bg-gray-50' : '', 'block py-2 px-4 text-sm text-gray-700']">{{
                    item.name
                  }}</a>
                </MenuItem>
              </MenuItems>
            </transition>
          </Menu>
        </div>
      </div>
    </header>
    <!-- Main content (sidebar + editor), spans horizontally -->
    <div class="flex flex-1 flex-row">
      <!-- Sidebar of get_view buttons & views -->
      <aside class="flex h-full w-80 resize-x border-r border-gray-200">
        <!-- View selection -->
        <div class="flex h-full min-h-0 flex-col border-r border-gray-200 p-1.5">
          <div class="flex flex-1 flex-col">
            <button
              class="rounded-sm px-2 py-2 text-gray-600"
              :class="get_view.selected ? 'bg-orange-100 text-orange-900' : 'hover:bg-gray-100'"
              v-for="get_view in viewNavigation"
              :key="get_view.name"
            >
              <span class="sr-only">{{ get_view.name }}</span>
              <component :is="get_view.icon" class="h-6 w-6" aria-hidden="true" />
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
          <!-- View header -->
          <div class="flex flex-row justify-between border-b border-gray-200 px-2 py-4">
            <span class="text-xs font-bold uppercase">Explorer</span>
            <!-- TODO @Feature: select explorer get_view (by type, by task tree) -->
          </div>
          <!-- View contents -->
          <div class="flex flex-1 flex-col">
            <!-- View: explorer -->
            <ul role="list" class="flex flex-col gap-1 text-sm">
              <li
                v-for="file in files"
                :key="file.id"
                class="py-1 pl-6 pr-2 hover:cursor-pointer"
                :class="
                  file.id == focusedFile?.id
                    ? 'bg-orange-100 font-bold text-orange-700'
                    : 'text-gray-700 hover:text-orange-700'
                "
              >
                {{ file.nameDotType }}
              </li>
            </ul>
          </div>
        </div>
      </aside>
      <!-- Main editor -->
      <main class="relative flex-1 flex-shrink-0 bg-gray-100">
        <div class="my-4 mx-auto w-2/5 rounded-sm border border-orange-600 shadow-md shadow-orange-200">
          <MonacoEditor v-model="focusedInstructionCode" />
        </div>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import SButton from "@/components/basic/SButton.vue";
import MonacoEditor from "@/components/MonacoEditor.vue";
import { graphql } from "@/gql";
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
import { computed } from "vue";

const props = defineProps<{
  organization: string;
  project: string;
}>();

const user = {
  name: "Florian Cäsar",
  email: "yatima@symbolx.com",
};
const projectNavigation = [{ name: "Rename", href: "#" }];
const userNavigation = [
  { name: "Settings", href: "#" },
  { name: "Sign out", href: "#" },
];

const viewNavigation = [
  { name: "Explorer", icon: ClipboardDocumentIcon, selected: true },
  { name: "Versions", icon: ClockIcon },
];

function compileProgram() {
  console.log("compileProgram");
}

const ProjectVersionFragment = graphql(/* GraphQL */ `
  fragment ProjectVersionFragment on ProjectVersion {
    name
    description
    createdAt
    committedAt
  }
`);

const TaskFragment = graphql(/* GraphQL */ `
  fragment TaskFragment on Task {
    name
    createdAt
    updatedAt
  }
`);

const InstructionFragment = graphql(/* GraphQL */ `
  fragment InstructionFragment on Instruction {
    name
    createdAt
    updatedAt
    builtinId
    code
    scope
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
          ...ProjectVersionFragment
        }
        versions {
          id
          ...ProjectVersionFragment
        }
      }
    }
  `),
  () => ({ id: projectId.value?.projectBySlug?.id }),
  () => ({ enabled: !!projectId.value?.projectBySlug?.id })
);

const currentVersion = computed(() => versionsQuery.value?.project?.head);
const { result: filesResult } = useQuery(
  graphql(/* GraphQL */ `
    query getProjectVersionFiles($id: GlobalID!) {
      projectVersion(id: $id) {
        id
        files {
          id
          name
          type
          nameDotType
        }
        program {
          id
          name
          schema
          children {
            id
            ...TaskFragment
            templateImplementation {
              id
              ...InstructionFragment
            }
            children {
              id
              ...TaskFragment
            }
          }
        }
      }
    }
  `),
  () => ({ id: currentVersion.value?.id }),
  () => ({ enabled: !!currentVersion.value?.id })
);
const files = computed(() => filesResult.value?.projectVersion?.files);
const focusedFile = computed(() => files.value?.[0]);

const focusedInstructionCode = "\
def x(): \n\
  print('Hello world!');\n\
  let y = 1;\n\
  return y;\n\
";
</script>
