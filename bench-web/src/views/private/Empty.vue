<script lang="tsx" setup>
import { ViewType } from "@/proto/wire";
import { Casing, toCasing } from "@/utils/string";

const props = defineProps<{ type: ViewType }>();
</script>
<template>
  <!-- TODO :UX!: proper empty states -->
  <div class="flex flex-row items-center">
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Empty {{ toCasing(ViewType[type], Casing.CAMEL) }} View</span>
  </div>
</template>
