<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Home</h1>
    </div>
    <div
      class="mx-auto mt-6 max-w-7xl divide-y divide-gray-200 overflow-hidden rounded-lg px-4 pt-6 sm:grid sm:grid-cols-2 sm:gap-px sm:divide-y-0 sm:px-6 md:px-8"
    >
      <div
        v-for="(action, actionIdx) in actions"
        :key="action.title"
        :class="[
          actionIdx === 0 ? 'rounded-tl-lg rounded-tr-lg sm:rounded-tr-none' : '',
          actionIdx === 1 ? 'sm:rounded-tr-lg' : '',
          actionIdx === actions.length - 2 ? 'sm:rounded-bl-lg' : '',
          actionIdx === actions.length - 1 ? 'rounded-bl-lg rounded-br-lg sm:rounded-bl-none' : '',
          'group relative bg-white p-6 focus-within:ring-2 focus-within:ring-inset focus-within:ring-orange-500',
        ]"
      >
        <div>
          <span :class="[action.iconBackground, action.iconForeground, 'inline-flex rounded-lg p-3 ring-4 ring-white']">
            <component :is="action.icon" class="h-6 w-6" aria-hidden="true" />
          </span>
        </div>
        <div class="mt-8">
          <h3 class="text-lg font-medium">
            <router-link :to="action.to" class="focus:outline-none">
              <!-- Extend touch target to entire panel -->
              <span class="absolute inset-0" aria-hidden="true" />
              {{ action.title }}
            </router-link>
          </h3>
          <p class="mt-2 text-sm text-gray-500">
            Doloribus dolores nostrum quia qui natus officia quod et dolorem. Sit repellendus qui ut at blanditiis et
            quo et molestiae.
          </p>
        </div>
        <span
          class="pointer-events-none absolute top-6 right-6 text-gray-300 group-hover:text-gray-400"
          aria-hidden="true"
        >
          <svg class="h-6 w-6" xmlns="http://www.w3.org/2000/svg" fill="currentColor" viewBox="0 0 24 24">
            <path
              d="M20 4h1a1 1 0 00-1-1v1zm-1 12a1 1 0 102 0h-2zM8 3a1 1 0 000 2V3zM3.293 19.293a1 1 0 101.414 1.414l-1.414-1.414zM19 4v12h2V4h-2zm1-1H8v2h12V3zm-.707.293l-16 16 1.414 1.414 16-16-1.414-1.414z"
            />
          </svg>
        </span>
      </div>
    </div>
  </Sidebar>
</template>

<script setup lang="ts">
import Sidebar from "@/components/Sidebar.vue";
import { CheckCircleIcon, PlusSmIcon, ServerIcon, TableIcon, UserGroupIcon } from "@heroicons/vue/outline";

// TODO @Feature: replace placeholder actions in Home screen
const actions = [
  {
    title: "Add model",
    to: "#",
    icon: PlusSmIcon,
    iconForeground: "text-teal-700",
    iconBackground: "bg-teal-50",
  },
  {
    title: "Add dataset",
    to: "#",
    icon: PlusSmIcon,
    iconForeground: "text-yellow-700",
    iconBackground: "bg-yellow-50",
  },
  {
    title: "Test model",
    to: "#",
    icon: CheckCircleIcon,
    iconForeground: "text-purple-700",
    iconBackground: "bg-purple-50",
  },
  {
    title: "Curate dataset",
    to: "#",
    icon: TableIcon,
    iconForeground: "text-sky-700",
    iconBackground: "bg-sky-50",
  },
  {
    title: "Manage resources",
    to: "#",
    icon: ServerIcon,
    iconForeground: "text-rose-700",
    iconBackground: "bg-rose-50",
  },
  {
    title: "Manage team",
    to: "#",
    icon: UserGroupIcon,
    iconForeground: "text-orange-700",
    iconBackground: "bg-orange-50",
  },
];
</script>
