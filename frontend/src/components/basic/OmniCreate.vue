<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { useActions } from "@/state/actions";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/vue";
import { DocumentPlusIcon, UserGroupIcon } from "@heroicons/vue/24/outline";
import { PlusCircleIcon } from "@heroicons/vue/24/solid";
import { computed } from "vue";

const actions = useActions();
const addables = computed(() =>
  [
    {
      name: "New file",
      action: () => {
        actions.file.create.value.apply();
      },
      enabled: actions.file.create.value.enabled,
      icon: DocumentPlusIcon,
    },
    {
      name: "New organization",
      to: { name: "CreateOrganization" },
      icon: UserGroupIcon,
    },
  ].filter((item) => item.enabled !== false)
);
</script>
<template>
  <Menu as="div" class="relative" v-slot="{ open }">
    <MenuButton
      class="flex items-center p-1 text-left hover:bg-orange-100 focus:bg-gray-100 focus:outline-none"
      :class="{ 'bg-orange-100': open }"
    >
      <PlusCircleIcon class="h-5 w-5 text-orange-600" />
    </MenuButton>
    <FadeTransition>
      <MenuItems
        class="absolute right-1 top-10 z-30 mt-0 w-48 origin-top-right rounded-sm bg-white px-1 py-1 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <MenuItem v-for="item in addables" :key="item.name" v-slot="{ active }">
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
</template>
