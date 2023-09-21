<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import UserAvatar from "@/components/basic/UserAvatar.vue";
import { useActions } from "@/state/actions";
import { useAuth } from "@/state/auth";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { ArrowLeftOnRectangleIcon, Cog8ToothIcon, UserCircleIcon } from "@heroicons/vue/24/outline";
import { useBrowserLocation } from "@vueuse/core";
import { computed } from "vue";

const auth = useAuth();
const location = useBrowserLocation();
const actions = useActions();

const userNavigation = computed(() => [
  { name: "Profile", icon: UserCircleIcon, to: "/" + auth.me.value?.username },
  { name: "Settings", icon: Cog8ToothIcon, to: "/settings/" + auth.me.value?.username },
  { name: "Log out", icon: ArrowLeftOnRectangleIcon, action: () => actions.user.logout.value.apply() },
]);
</script>
<template>
  <FadeTransition mode="out-in">
    <div v-if="!auth.loggedIn.value || auth.me.value == null" class="flex flex-row gap-2 px-4">
      <router-link
        :to="{ name: 'Signup', query: { next: location.href } }"
        class="rounded-sm bg-orange-600 px-2 py-1 text-sm text-white"
      >
        Sign up
      </router-link>
      <router-link
        :to="{ name: 'Login', query: { next: location.href } }"
        class="rounded-sm px-2 py-1 text-sm hover:bg-orange-100"
      >
        Log in
      </router-link>
    </div>
    <Menu v-else as="div" class="relative h-full flex-shrink-0" v-slot="{ open }">
      <MenuButton
        class="group flex h-full items-center px-2 text-left focus:bg-orange-100 focus:outline-none"
        :class="{ 'bg-orange-100': open }"
      >
        <UserAvatar
          :client-id="auth.me.value.id"
          :user="auth.me.value"
          class="h-7 w-7 text-gray-900 transition-transform duration-150 group-hover:bg-orange-200"
        />
      </MenuButton>
      <FadeTransition>
        <MenuItems
          class="absolute right-1 top-12 z-30 mt-0 w-48 origin-top-right rounded-sm bg-white px-1 py-1 shadow-md outline-none ring-1 ring-orange-900 ring-opacity-40"
        >
          <div class="flex flex-row items-center gap-1 px-2">
            <div class="p-1">
              <UserAvatar :client-id="auth.me.value.id" :user="auth.me.value" class="h-8 w-8 text-gray-700" />
            </div>
            <p class="flex max-w-full flex-col px-2 py-2">
              <span class="truncate text-sm text-gray-900">{{ auth.me.value?.username }}</span>
              <span class="truncate text-xs text-gray-500">{{ auth.me.value?.name }}</span>
              <!-- Future plan info -->
            </p>
          </div>
          <MenuItem v-for="item in userNavigation" :key="item.name" v-slot="{ active }">
            <router-link
              v-if="item.to"
              :to="item.to"
              class="flex flex-row items-center gap-2"
              :class="[active ? 'bg-orange-100' : '', 'block px-2 py-2 text-sm text-gray-900']"
            >
              <component :is="item.icon" class="h-5 w-5 text-gray-700" />
              {{ item.name }}
            </router-link>
            <button
              v-else
              class="flex flex-row items-center gap-2"
              :class="[active ? 'bg-orange-100' : '', 'block w-full px-2 py-2 text-left text-sm text-gray-900']"
              @click="item.action"
            >
              <component :is="item.icon" class="h-5 w-5 text-gray-700" />
              {{ item.name }}
            </button>
          </MenuItem>
        </MenuItems>
      </FadeTransition>
    </Menu>
  </FadeTransition>
</template>
