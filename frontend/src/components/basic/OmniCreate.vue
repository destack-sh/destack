<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { PlusIcon } from "@heroicons/vue/24/outline";
import { computed } from "vue";

const addables = computed(() =>
  [
    {
      name: "New organization",
      to: { name: "CreateOrganization" },
      enabled: false,
    },
    {
      name: "New project",
      to: { name: "CreateProject" },
    },
    {
      name: "New file",
      action: () => {
        console.log("new file");
      },
    },
  ].filter((item) => item.enabled !== false)
);
</script>
<template>
  <Menu as="div" class="relative h-full flex-shrink-0 ring-0 focus:outline-none focus:ring-0" v-slot="{ open }">
    <MenuButton
      class="flex h-full items-center px-2 text-left hover:bg-orange-50 focus:bg-gray-100 focus:outline-none"
      :class="{ 'bg-orange-50': open }"
    >
      <PlusIcon class="h-5 w-5 text-orange-600" />
    </MenuButton>
    <FadeTransition>
      <MenuItems
        class="absolute right-1 top-12 z-10 mt-0 w-48 origin-top-right rounded-sm bg-white px-1 py-1 shadow-md ring-1 ring-orange-900 ring-opacity-40 focus:outline-none"
      >
        <MenuItem v-for="item in addables" :key="item.name" v-slot="{ active }">
          <router-link
            v-if="item.to"
            :to="item.to"
            :class="[active ? 'bg-orange-50' : '', 'block py-2 px-4 text-sm text-gray-900']"
          >
            {{ item.name }}
          </router-link>
          <button
            v-else
            :class="[active ? 'bg-orange-50' : '', 'block w-full py-2 px-4 text-left text-sm text-gray-900']"
            @click="item.action"
          >
            {{ item.name }}
          </button>
        </MenuItem>
      </MenuItems>
    </FadeTransition>
  </Menu>
</template>
