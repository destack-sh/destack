<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";

// fake data
const user = {
  name: "Florian Cäsar",
  email: "yatima@symbolx.com",
};
const userNavigation = [
  { name: "Settings", href: "#" },
  { name: "Sign out", href: "#" },
];
</script>
<template>
  <Menu as="div" class="relative flex-shrink-0">
    <div>
      <MenuButton class="flex flex-col px-4 py-2 text-left hover:bg-gray-50 focus:bg-gray-100 focus:outline-none">
        <span class="sr-only">Open user menu</span>
        <span class="text-xs font-bold text-gray-900">{{ user.name }}</span>
        <span class="text-xs text-gray-500">Personal</span>
      </MenuButton>
    </div>
    <FadeTransition>
      <MenuItems
        class="absolute right-0 z-10 mt-0 w-48 origin-top-right rounded-sm bg-white px-1 py-1 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
      >
        <MenuItem v-for="item in userNavigation" :key="item.name" v-slot="{ active }">
          <a :href="item.href" :class="[active ? 'bg-gray-100' : '', 'block py-2 px-4 text-sm text-gray-700']">
            {{ item.name }}
          </a>
        </MenuItem>
      </MenuItems>
    </FadeTransition>
  </Menu>
</template>
