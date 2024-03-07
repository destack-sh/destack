<script lang="tsx" setup>
import { bench, user } from "@/system/space";
import Dock from "@/views/system/Dock.vue";
</script>
<template>
  <div class="flex w-full flex-row items-center justify-between gap-x-4 bg-gray-100 px-4 text-sm text-gray-900">
    <!-- Left -->
    <div class="flex">
      <!-- Bench -->
      <button class="flex-shrink-0 rounded-md border border-gray-300 bg-white px-2 py-1" v-if="bench">Bench</button>
      <button
        v-else
        class="flex-shrink-0 rounded-md border border-gray-300 bg-white px-2 py-1 shadow-sm shadow-gray-300"
      >
        <span class="font-semibold">Bench</span>
        <span class="skew-y-4 ml-1.5 rounded-md bg-primary-300 px-1 py-0.5 font-semibold">Beta</span>
      </button>
      <!-- Status -->
      <!-- ... -->
    </div>

    <!-- Middle -->
    <div class="flex flex-1 items-center justify-center -sm:hidden">
      <!-- Dock -->
      <Dock
        class="w-fit flex-shrink-0 rounded-md border border-gray-300 bg-white px-2 py-1 shadow-sm shadow-gray-300"
      />
    </div>

    <!-- Right -->
    <div class="flex flex-row">
      <!-- User -->
      <div v-if="user" class="">
        <button>
          {{ user.slug }}
        </button>
      </div>
      <div v-else class="">
        <button
          class="flex-shrink-0 rounded-md border border-gray-900 bg-primary-300 px-2 py-1 text-gray-900 shadow-sm shadow-gray-900 hover:bg-primary-400"
        >
          <i class="fas fa-arrow-right-from-bracket mr-2"></i>
          <span class="font-semibold">Sign in</span>
        </button>
      </div>
    </div>
  </div>
</template>
