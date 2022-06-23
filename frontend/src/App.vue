<script setup lang="ts">
import { useMeta } from "vue-meta";
import { RouterView } from "vue-router";
import { useAppStore } from "./stores";

useMeta({ title: "Bench" });

const appStore = useAppStore();
</script>

<template>
  <div v-if="appStore.hydrating">loading</div>
  <div v-else>
    <RouterView />
  </div>
</template>

<style>
@import "@/assets/base.css";
</style>
