<template>
  <Sidebar>
    <div class="mx-auto flex max-w-7xl justify-between px-4 pt-6 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Flows</h1>
      <SButton variant="solid" color="slate" to="/flows/new" text="Create flow" />
    </div>
    <FlowList />
  </Sidebar>
</template>
<script lang="ts" setup>
import FlowList from "@/components/FlowList.vue";
import Sidebar from "@/components/Sidebar.vue";
import SButton from "@/components/basic/SButton.vue";
</script>
