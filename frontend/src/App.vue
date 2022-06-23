<script setup lang="ts">
import { useMeta } from "vue-meta";
import { RouterView } from "vue-router";

useMeta({ title: "Bench" });
</script>

<template>
  <RouterView />
</template>

<style>
@import "@/assets/base.css";
</style>
