<script lang="tsx" setup>
import { computed, type Ref } from "vue";

type DockItem = {
  id: string;
  name: string;
  icon: string; // font awesome solid icon
  text: string;
  url?: string;
  action?: () => void;
};

const items: Ref<DockItem[]> = computed(
  () =>
    [
      {
        id: "home",
        name: "Home",
        icon: "fas fa-house",
      },
      {
        id: "search",
        name: "Search",
        icon: "fas fa-magnifying-glass",
      },
      // {
      //   id: "chat",
      //   name: "Chat",
      //   icon: "fas fa-comment-dots",
      // },
      {
        id: "inspect",
        name: "Inspector",
        icon: "fas fa-eye-dropper",
      },
      // {
      //   id: "library",
      //   name: "Library",
      //   icon: "fas fa-book",
      // },
      {
        id: "docs",
        name: "Documentation",
        icon: "fas fa-book-open",
      },
      {
        id: "community",
        name: "Community",
        icon: "fab fa-discord",
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
      class="rounded-md border hover:cursor-pointer border-gray-300 bg-secondary-300 px-1 text-gray-900 hover:bg-secondary-400"
      @click="item.action"
      :href="item.url"
      target="_blank"
    >
      <i :class="`${item.icon}`"></i>
    </component>
  </div>
</template>
