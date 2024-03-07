<script lang="tsx" setup>
import { user } from "@/system/space";
import Dock from "@/views/intrinsics/Dock.vue";
</script>
<template>
  <div class="w-full flex text-sm items-center justify-between flex-row px-4 bg-gray-50">
    <!-- Bench info -->
    <div class="bg-white px-2 py-1 rounded-sm border border-gray-200">Bench</div>
    <!-- Dock -->
    <Dock class="bg-white px-2 py-1 rounded-sm border border-gray-200" />
    <!-- User -->
    <div class="bg-white px-2 py-1 rounded-sm border border-gray-200">
      {{ user ? user.slug : "Log in" }}
    </div>
  </div>
</template>
