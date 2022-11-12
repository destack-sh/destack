<template>
  <header class="static mx-auto overflow-y-visible bg-white shadow-sm">
    <div class="relative flex justify-between gap-8">
      <!-- Left side: organizational -->
      <div class="static flex items-center">
        <!-- Home -->
        <div class="flex flex-shrink-0 items-center py-2 px-4 hover:bg-gray-50">
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
              class="flex h-full items-center justify-between bg-white py-2 px-4 text-left hover:bg-gray-50 focus:bg-gray-100 focus:outline-none"
            >
              <span class="sr-only">Open project menu</span>
              <span class="text-sm font-bold">{{ organization }}/{{ project }}</span>
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
              <MenuItem v-for="item in userNavigation" :key="item.name" v-slot="{ active }">
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
            <span class="ml-1">Compile</span>
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
              class="flex flex-col bg-white py-2 px-4 text-left hover:bg-gray-50 focus:bg-gray-100 focus:outline-none"
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
</template>

<script setup lang="ts">
import SButton from "@/components/basic/SButton.vue";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { ChevronDownIcon } from "@heroicons/vue/20/solid";
import { PlayIcon, WrenchIcon } from "@heroicons/vue/24/outline";

const props = defineProps<{
  organization: string;
  project: string;
}>();

const user = {
  name: "Florian Cäsar",
  email: "yatima@symbolx.com",
};
const userNavigation = [
  { name: "Settings", href: "#" },
  { name: "Sign out", href: "#" },
];
</script>
