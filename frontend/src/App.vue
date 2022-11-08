<template>
  <div v-if="appStore.hydrating" class="pt-8 text-center">Loading...</div>
  <RouterView v-else />
</template>

<script setup lang="ts">
import { useMeta } from "vue-meta";
import { RouterView } from "vue-router";
import { useAppStore } from "./stores";

useMeta({ title: "Bench" });

const appStore = useAppStore();
</script>

<style>
@import "@/assets/base.css";
@import url("https://fonts.googleapis.com/css2?family=IBM+Plex+Sans&display=swap");
@import "@braks/vue-flow/dist/style.css";
@import "@braks/vue-flow/dist/theme-default.css";
</style>
