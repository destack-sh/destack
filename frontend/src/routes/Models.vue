<template>
  <Sidebar>
    <div class="mx-auto flex max-w-7xl justify-between px-4 pt-6 sm:px-6 md:px-8">
      <h1 class="text-2xl font-semibold text-gray-900">Models</h1>
      <SButton variant="solid" color="slate" to="/models/new" text="Create model" />
    </div>
    <ArtifactList type="model" />
  </Sidebar>
</template>
<script lang="ts" setup>
import Sidebar from "@/components/Sidebar.vue";
import SButton from "@/components/basic/SButton.vue";
import ArtifactList from "../components/ArtifactList.vue";
</script>
