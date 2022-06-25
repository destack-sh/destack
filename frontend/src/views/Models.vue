<template>
  <Sidebar>
    <div class="mx-auto max-w-7xl px-4 pt-6 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Models</h1>
    </div>
    <ModelsList />
  </Sidebar>
</template>
<script lang="ts" setup>
import ModelsList from "@/components/ModelsList.vue";
import Sidebar from "@/components/Sidebar.vue";
</script>
