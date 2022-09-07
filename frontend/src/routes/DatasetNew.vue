<template>
  <Sidebar>
    <DatasetCreator v-bind="attrs" />
  </Sidebar>
</template>
<script lang="ts" setup>
import { useAttrs } from "vue";
import Sidebar from "@/components/Sidebar.vue";
import DatasetCreator from "@/views/DatasetCreator.vue";

const attrs = useAttrs();
</script>
