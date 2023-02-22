<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useAuth } from "@/state/auth";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";

const userNavigation = [
  { name: "Settings", href: "#" },
  { name: "Sign out", href: "#" },
];

const auth = useAuth();
</script>
<template>
  <div v-if="!auth.loggedIn.value" class="flex flex-row gap-2 px-4">
    <router-link :to="{ name: 'Signup' }" class="rounded-sm bg-orange-600 py-1 px-2 text-sm text-white">
      Sign up
    </router-link>
    <router-link :to="{ name: 'Login' }" class="rounded-sm py-1 px-2 text-sm hover:bg-orange-50"> Log in </router-link>
  </div>
  <Menu v-else as="div" class="relative flex-shrink-0">
    <div>
      <MenuButton class="flex flex-col px-4 text-left hover:bg-gray-50 focus:bg-gray-100 focus:outline-none">
        <span class="sr-only">Open user menu</span>
        <span class="text-xs font-bold text-gray-900">{{ auth.me.value?.username }}</span>
        <span class="text-xs text-gray-500">{{ auth.me.value?.email }}</span>
      </MenuButton>
    </div>
    <FadeTransition>
      <MenuItems
        class="absolute right-0 z-10 mt-0 w-48 origin-top-right rounded-sm bg-white px-1 py-1 shadow-md ring-1 ring-black ring-opacity-5 focus:outline-none"
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
