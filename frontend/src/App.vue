<script setup lang="ts">
import { useMeta } from "vue-meta";
import { RouterView } from "vue-router";
import { useAppStore } from "./stores";

useMeta({ title: "Bench" });

const appStore = useAppStore();
</script>

<template>
  <div v-if="appStore.hydrating" class="pt-8 text-center">Loading...</div>
  <RouterView v-else />
</template>

<style>
@import "@/assets/base.css";
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Sans&display=swap");
@import "@braks/vue-flow/dist/style.css";
@import "@braks/vue-flow/dist/theme-default.css";

::-webkit-scrollbar {
  width: 12px;
  height: 12px;
}

::-webkit-scrollbar-thumb {
  background-color: rgba(113, 122, 148, 0.4);
  background-clip: padding-box;
  border: 2px solid rgba(0, 0, 0, 0);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background-color: #fdba74;
}

::-webkit-scrollbar-track {
  background-color: transparent;
}
</style>
