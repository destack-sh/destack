<script lang="ts" setup>
import { PlusIcon } from "@heroicons/vue/20/solid";
</script>
<template>
  <button
    class="group relative flex w-full cursor-default py-1 opacity-0 outline-none transition-opacity hover:opacity-100"
  >
    <div class="justify-left relative flex align-top">
      <span class="bg-white px-2">
        <PlusIcon class="h-4 w-4 text-gray-300" aria-hidden="true" />
      </span>
    </div>
  </button>
</template>
