<script lang="ts" setup>
import FadeTransition from "@/components/basic/FadeTransition.vue";
import { VERSION } from "@/utils/globals";
import { Popover, PopoverPanel } from "@headlessui/vue";
import {
  BookOpenIcon,
  ChatBubbleBottomCenterIcon,
  ClipboardDocumentIcon,
  LifebuoyIcon,
  UserGroupIcon,
} from "@heroicons/vue/24/outline";
import { RouterLink, useRoute } from "vue-router";

const route = useRoute();

const helpActions = [
  {
    name: "View examples",
    to: "/symbolx/examples",
    icon: ClipboardDocumentIcon,
  },
  {
    name: "Read the docs",
    icon: BookOpenIcon,
    to: "/symbolx/docs",
  },
  {
    name: "Ask the community",
    icon: UserGroupIcon,
    href: "https://forum.bench.is",
    soon: true,
  },
  {
    name: "Join the Discord",
    icon: ChatBubbleBottomCenterIcon,
    href: "https://discord.gg/BUaeEn8FHN",
  },
  {
    name: "Get help (fast)",
    icon: LifebuoyIcon,
    href: "mailto:florian@symbolx.com?subject=" + encodeURIComponent("Help with " + route.path),
  },
];
</script>

<template>
  <Popover v-slot="{ open }" class="relative">
    <slot name="button" :open="open" />

    <FadeTransition>
      <PopoverPanel
        class="absolute bottom-0 left-11 z-40 flex w-60 flex-col gap-2 rounded-sm bg-white px-2 py-2 shadow-md ring-1 ring-orange-900 ring-opacity-40"
      >
        <!-- Version -->
        <span class="px-2 text-center text-xs text-gray-700">Bench {{ VERSION }}</span>
        <!-- Actions -->
        <template v-for="action in helpActions" :key="action.name">
          <component
            :is="action.to == null ? 'a' : RouterLink"
            v-bind="action.to == null ? { href: action.href } : { to: action.to }"
            target="_blank"
            class="flex flex-row items-center gap-2 rounded-sm px-2 py-1 hover:bg-orange-100"
          >
            <component :is="action.icon" class="h-5 w-5 text-gray-700" />
            <span class="text-sm text-gray-900">{{ action.name }}</span>
            <span class="rounded-sm border border-orange-600 px-1 text-xs font-bold text-orange-600" v-if="action.soon">
              soon
            </span>
          </component>
        </template>
      </PopoverPanel>
    </FadeTransition>
  </Popover>
</template>
