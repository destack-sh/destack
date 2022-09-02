<template>
  <Sidebar>
    <div class="mx-auto flex max-w-7xl justify-between px-4 pt-6 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Models</h1>
      <button
        to="/models/new"
        type="button"
        class="inline-flex items-center rounded-md border border-transparent bg-slate-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-orange-500 focus:ring-offset-2"
        text="Create model"
      >
        Create model
      </button>
    </div>
    <ModelList />
  </Sidebar>
</template>
<script lang="ts" setup>
import ModelList from "@/components/ModelList.vue";
import Sidebar from "@/components/Sidebar.vue";
</script>
