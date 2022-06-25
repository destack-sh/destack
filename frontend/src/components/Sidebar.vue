<template>
  <div>
    <TransitionRoot as="template" :show="sidebarOpen">
      <Dialog as="div" class="relative z-40 md:hidden" @close="sidebarOpen = false">
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
            <DialogPanel class="relative flex w-full max-w-xs flex-1 flex-col bg-orange-700">
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
      <div class="flex min-h-0 flex-1 flex-col border-r border-gray-200 bg-white">
        <div class="flex flex-1 flex-col overflow-y-auto pt-5 pb-4">
          <router-link to="/" class="flex flex-shrink-0 items-center gap-2 px-4">
            <img class="h-8 w-auto" src="/android-chrome-192x192.png" alt="Bench Logo" />
            <span class="text-xl font-bold text-orange-400">Bench</span>
          </router-link>
          <nav class="mt-5 flex-1 space-y-1 bg-white px-2">
            <router-link
              v-for="item in navigation"
              :key="item.name"
              :to="item.to"
              :class="[
                item.current
                  ? 'bg-gray-100 text-gray-900'
                  : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900',
                'group flex items-center rounded-md px-2 py-2 text-sm font-medium',
              ]"
            >
              <component
                :is="item.icon"
                :class="[
                  item.current ? 'text-gray-500' : 'text-gray-400 group-hover:text-gray-500',
                  'mr-3 h-6 w-6 flex-shrink-0',
                ]"
                aria-hidden="true"
              />
              {{ item.name }}
            </router-link>
          </nav>
        </div>
        <div class="flex flex-shrink-0 border-t border-gray-200 p-4">
          <a href="#" class="group block w-full flex-shrink-0">
            <div class="flex items-center text-gray-500">v0.0.1</div>
          </a>
        </div>
      </div>
    </div>

    <!-- Content container -->
    <div class="flex flex-1 flex-col md:pl-64">
      <div class="sticky top-0 z-10 bg-gray-100 pt-1 pl-1 sm:pl-3 sm:pt-3 md:hidden">
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
        <slot />
      </main>
    </div>
  </div>
</template>
<script lang="ts" setup>
import { Dialog, DialogPanel, TransitionChild, TransitionRoot } from "@headlessui/vue";
import {
  BeakerIcon,
  ChipIcon,
  CubeIcon,
  DatabaseIcon,
  MenuIcon,
  XIcon,
} from "@heroicons/vue/outline";
import { ref } from "vue";

const navigation = [
  //   { name: "Dashboards", to: "/dashboards", icon: ChartBarIcon, current: true },
  //   { name: "Projects", to: "/projects", icon: FolderIcon, current: false },
  { name: "Models", to: "/models", icon: ChipIcon, current: false },
  { name: "Datasets", to: "/datasets", icon: DatabaseIcon, current: false },
  { name: "Playground", to: "/playground", icon: CubeIcon, current: false },
  { name: "Laboratory", to: "/laboratory", icon: BeakerIcon, current: false },
  //   { name: "Resources", to: "/resources", icon: ServerIcon, current: false },
];

const sidebarOpen = ref(false);
</script>
