<template>
  <div>
    <TransitionRoot as="template" :show="sidebarOpen">
      <Dialog
        as="div"
        class="relative z-40 md:hidden"
        @close="sidebarOpen = false"
      >
        <TransitionChild
          as="template"
          enter="transition-opacity ease-linear duration-300"
          enter-from="opacity-0"
          enter-to="opacity-100"
          leave="transition-opacity ease-linear duration-300"
          leave-from="opacity-100"
          leave-to="opacity-0"
        >
          <div class="fixed inset-0 bg-gray-600 bg-opacity-75" />
        </TransitionChild>

        <div class="fixed inset-0 z-40 flex">
          <TransitionChild
            as="template"
            enter="transition ease-in-out duration-300 transform"
            enter-from="-translate-x-full"
            enter-to="translate-x-0"
            leave="transition ease-in-out duration-300 transform"
            leave-from="translate-x-0"
            leave-to="-translate-x-full"
          >
            <DialogPanel
              class="relative flex w-full max-w-xs flex-1 flex-col bg-orange-700"
            >
              <TransitionChild
                as="template"
                enter="ease-in-out duration-300"
                enter-from="opacity-0"
                enter-to="opacity-100"
                leave="ease-in-out duration-300"
                leave-from="opacity-100"
                leave-to="opacity-0"
              >
                <div class="absolute top-0 right-0 -mr-12 pt-2">
                  <button
                    type="button"
                    class="ml-1 flex h-10 w-10 items-center justify-center rounded-full focus:outline-none focus:ring-2 focus:ring-inset focus:ring-white"
                    @click="sidebarOpen = false"
                  >
                    <span class="sr-only">Close sidebar</span>
                    <XIcon class="h-6 w-6 text-white" aria-hidden="true" />
                  </button>
                </div>
              </TransitionChild>
              <div class="h-0 flex-1 overflow-y-auto pt-5 pb-4">
                <nav class="mt-5 space-y-1 px-2">
                  <router-link
                    v-for="item in navigation"
                    :key="item.name"
                    :to="item.to"
                    :class="[
                      item.current
                        ? 'bg-orange-800 text-white'
                        : 'text-white hover:bg-orange-600 hover:bg-opacity-75',
                      'group flex items-center rounded-md px-2 py-2 text-base font-medium',
                    ]"
                  >
                    <component
                      :is="item.icon"
                      class="mr-4 h-6 w-6 flex-shrink-0 text-orange-300"
                      aria-hidden="true"
                    />
                    {{ item.name }}
                  </router-link>
                </nav>
              </div>
            </DialogPanel>
          </TransitionChild>
          <div class="w-14 flex-shrink-0" aria-hidden="true">
            <!-- Force sidebar to shrink to fit close icon -->
          </div>
        </div>
      </Dialog>
    </TransitionRoot>

    <!-- Static sidebar for desktop -->
    <div class="hidden md:fixed md:inset-y-0 md:flex md:w-64 md:flex-col">
      <!-- Sidebar component, swap this element with another sidebar if you like -->
      <div class="flex min-h-0 flex-1 flex-col bg-orange-700">
        <div class="flex flex-1 flex-col overflow-y-auto pt-5 pb-4">
          <div class="flex flex-shrink-0 items-center px-4">
            <img
              class="h-8 w-auto"
              src="/android-chrome-192x192.png"
              alt="Workflow"
            />
            <span class="pl-2 font-bold text-white"> bench </span>
          </div>
          <nav class="mt-5 flex-1 space-y-1 px-2">
            <router-link
              v-for="item in navigation"
              :key="item.name"
              :to="item.to"
              :class="[
                item.current
                  ? 'bg-orange-800 text-white'
                  : 'text-white hover:bg-orange-600 hover:bg-opacity-75',
                'group flex items-center rounded-md px-2 py-2 text-sm font-medium',
              ]"
            >
              <component
                :is="item.icon"
                class="mr-3 h-6 w-6 flex-shrink-0 text-orange-300"
                aria-hidden="true"
              />
              {{ item.name }}
            </router-link>
          </nav>
        </div>
      </div>
    </div>
    <div class="flex flex-1 flex-col md:pl-64">
      <div
        class="sticky top-0 z-10 bg-gray-100 pt-1 pl-1 sm:pl-3 sm:pt-3 md:hidden"
      >
        <button
          type="button"
          class="-ml-0.5 -mt-0.5 inline-flex h-12 w-12 items-center justify-center rounded-md text-gray-500 hover:text-gray-900 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-orange-500"
          @click="sidebarOpen = true"
        >
          <span class="sr-only">Open sidebar</span>
          <MenuIcon class="h-6 w-6" aria-hidden="true" />
        </button>
      </div>
      <main class="flex-1">
        <div class="py-6">
          <div class="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
            <h1 class="text-2xl font-semibold text-gray-900">Models</h1>
          </div>
          <div class="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
            <ul
              role="list"
              class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3"
            >
              <li
                v-for="model in artifactsStore.models"
                :key="model.id"
                class="col-span-1 divide-y divide-gray-200 rounded-lg bg-white shadow"
              >
                <div
                  class="flex w-full items-center justify-between space-x-6 p-6"
                >
                  <div class="flex-1 truncate">
                    <div class="flex items-center space-x-3">
                      <h3 class="truncate text-sm font-medium text-gray-900">
                        {{ model.name }}
                      </h3>
                      <span
                        class="inline-block flex-shrink-0 rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-800"
                        >{{ model.type }}</span
                      >
                    </div>
                    <p class="mt-1 truncate text-sm text-gray-500">
                      {{ model.description }}
                    </p>
                  </div>
                  <component
                    :is="getIconForModel(model)"
                    class="h-10 w-10 flex-shrink-0 text-orange-300"
                    aria-hidden="true"
                  />
                </div>
                <div>
                  <div class="-mt-px flex divide-x divide-gray-200">
                    <div class="flex w-0 flex-1">
                      <router-link
                        to="#"
                        class="relative -mr-px inline-flex w-0 flex-1 items-center justify-center rounded-bl-lg border border-transparent py-4 text-sm font-medium text-gray-700 hover:text-gray-500"
                      >
                        <GlobeIcon
                          class="h-5 w-5 text-gray-400"
                          aria-hidden="true"
                        />
                        <span class="ml-3">Explore</span>
                      </router-link>
                    </div>
                    <div class="-ml-px flex w-0 flex-1">
                      <router-link
                        to="#"
                        class="relative inline-flex w-0 flex-1 items-center justify-center rounded-br-lg border border-transparent py-4 text-sm font-medium text-gray-700 hover:text-gray-500"
                      >
                        <BeakerIcon
                          class="h-5 w-5 text-gray-400"
                          aria-hidden="true"
                        />
                        <span class="ml-3">Analyse</span>
                      </router-link>
                    </div>
                  </div>
                </div>
              </li>
            </ul>
          </div>
        </div>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useArtifactsStore } from "@/stores/artifacts";
import type { Artifact } from "@/types/artifacts";
import {
  Dialog,
  DialogPanel,
  TransitionChild,
  TransitionRoot,
} from "@headlessui/vue";
import {
  BeakerIcon,
  ChipIcon,
  DatabaseIcon,
  DocumentTextIcon,
  GlobeIcon,
  MenuIcon,
  XIcon,
} from "@heroicons/vue/outline";
import { ref } from "vue";

const artifactsStore = useArtifactsStore();

const navigation = [
  // { name: "Dashboards", to: "/dashboards", icon: ChartBarIcon, current: true },
  // { name: "Projects", to: "/projects", icon: FolderIcon, current: false },
  { name: "Models", to: "/models", icon: ChipIcon, current: false },
  { name: "Datasets", to: "/datasets", icon: DatabaseIcon, current: false },
  { name: "Analysis", to: "/analysis", icon: BeakerIcon, current: false },
  // { name: "Resources", to: "/resources", icon: CloudIcon, current: false },
];

const sidebarOpen = ref(false);

function getIconForModel(model: Artifact) {
  if (model.type == "model") {
    return DocumentTextIcon;
  }
}
</script>
