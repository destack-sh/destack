<script lang="ts" setup>
import { ViewType } from "@/proto/wire";
import { toCamelName } from "@/language/core/const";

const props = defineProps<{ type: ViewType }>();
</script>
<template>
  <!-- TODO :UX!: proper empty states -->
  <div class="flex flex-row items-center">
    <i class="fas fa-empty-set text-gray-500" />
    <span class="text-gray-600">Empty {{ toCamelName(ViewType, type) }} View</span>
  </div>
</template>
