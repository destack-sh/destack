<script lang="tsx" setup>
import { bench, user } from "@/system/space";
import Dock from "@/views/intrinsics/Dock.vue";
</script>
<template>
  <div class="flex w-full flex-row items-center justify-between bg-gray-100 px-4 text-sm text-gray-900">
    <!-- Bench -->
    <button class="flex-shrink-0 rounded-md border border-gray-300 bg-white px-2 py-1" v-if="bench">Bench</button>
    <button v-else class="shadow-sm shadow-gray-300 flex-shrink-0 rounded-md border border-gray-300 bg-white px-2 py-1">
      <span class="font-semibold">Bench</span>
      <span class="skew-y-4 ml-1.5 rounded-md bg-primary-300 px-1 py-0.5 font-semibold">Beta</span>
    </button>
    <!-- Dock -->
    <Dock class="-md:hidden shadow-sm shadow-gray-300 flex-shrink-0 rounded-md border border-gray-300 bg-white px-2 py-1" />
    <!-- User -->
    <button v-if="user">
      {{ user.slug }}
    </button>
    <button v-else class="flex-shrink-0 hover:bg-primary-400 shadow-sm shadow-gray-900 rounded-md border border-gray-900 bg-primary-300 px-2 py-1 text-gray-900">
      <i class="fas fa-arrow-right-from-bracket mr-2"></i>
      <span class="font-semibold">Sign in</span>
    </button>
  </div>
</template>
