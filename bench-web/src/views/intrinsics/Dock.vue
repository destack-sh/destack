<script lang="tsx" setup>
import { Tooltip } from "@/utils/tooltip";
import { computed, shallowRef, type Ref } from "vue";

type DockItem = {
  id: string;
  name: string;
  icon: string; // font awesome solid icon
  text: string;
  url?: string;
  shortcut?: string;
  action?: () => void;
};

const items: Ref<DockItem[]> = computed(
  () =>
    [
      {
        id: "home",
        name: "Home",
        text: "Go to the home page",
        icon: "fas fa-house",
      },
      {
        id: "search",
        name: "Search",
        text: "Search everything in the space",
        icon: "fas fa-magnifying-glass",
        shortcut: "Ctrl+K",
      },
      // {
      //   id: "chat",
      //   name: "Chat",
      //   text: "Launch chat with your Bench",
      //   icon: "fas fa-comment-dots",
      // },
      {
        id: "inspect",
        name: "Inspector",
        text: "Get details on some node",
        icon: "fas fa-eye-dropper",
        shortcut: "Ctrl+I",
      },
      // {
      //   id: "library",
      //   name: "Library",
      //   text: "Reuse common nodes from our library"
      //   icon: "fas fa-book",
      // },
      {
        id: "docs",
        name: "Documentation",
        text: "Read up on help and guides",
        icon: "fas fa-book-open",
      },
      {
        id: "community",
        name: "Community",
        icon: "fab fa-discord",
        text: "Join our community on Discord",
        url: "https://discord.gg/pSBdq6XC",
      },
    ] as DockItem[],
);
</script>
<template>
  <div class="flex flex-row items-center gap-x-2">
    <component
      :is="item.url ? 'a' : 'button'"
      v-for="item in items"
      :key="item.id"
      class="group relative rounded-md border border-gray-300 bg-secondary-300 px-1 text-gray-800 hover:cursor-pointer hover:bg-secondary-400"
      @click="item.action"
      :href="item.url"
      target="_blank"
    >
      <i :class="`${item.icon}`" />
      <Tooltip :icon="item.icon" :title="item.name" :text="item.text" :shortcut="item.shortcut" position="top-6 -left-3" />
    </component>
  </div>
</template>
