<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useActions } from "@/state/actions";
import { useAuth } from "@/state/auth";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { useBrowserLocation } from "@vueuse/core";
import { computed } from "vue";

const auth = useAuth();
const location = useBrowserLocation();
const actions = useActions();

const userNavigation = computed(() => [
  { name: "Profile", to: "/" + auth.me.value?.username },
  { name: "Settings", to: "/settings/profile" },
  { name: "Log out", action: () => actions.user.logout.value.apply() },
]);
</script>
<template>
  <div v-if="!auth.loggedIn.value" class="flex flex-row gap-2 px-4">
    <router-link
      :to="{ name: 'Signup', query: { next: location.href } }"
      class="rounded-sm bg-orange-600 py-1 px-2 text-sm text-white"
    >
      Sign up
    </router-link>
    <router-link
      :to="{ name: 'Login', query: { next: location.href } }"
      class="rounded-sm py-1 px-2 text-sm hover:bg-orange-50"
    >
      Log in
    </router-link>
  </div>
  <Menu v-else as="div" class="relative h-full flex-shrink-0">
    <MenuButton class="flex h-full items-center px-4 text-left hover:bg-orange-50 focus:bg-gray-100 focus:outline-none">
      <div class="flex flex-col">
        <span class="text-xs font-bold text-gray-900">{{ auth.me.value?.username }}</span>
        <span class="text-xs text-gray-500">{{ auth.me.value?.firstName }}</span>
      </div>
    </MenuButton>
    <FadeTransition>
      <MenuItems
        class="absolute right-1 top-12 z-10 mt-0 w-48 origin-top-right rounded-sm bg-white px-1 py-1 shadow-md ring-1 ring-orange-900 ring-opacity-40 focus:outline-none"
      >
        <MenuItem v-for="item in userNavigation" :key="item.name" v-slot="{ active }">
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
