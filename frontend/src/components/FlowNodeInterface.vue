<template>
  <div class="divide-y divide-gray-200 overflow-hidden rounded-lg bg-white shadow">
    <div class="px-4 py-3 sm:px-6">
      <h3 class="text-md font-medium leading-6 text-gray-900">{{ label }}</h3>
      <h5 class="text-sm font-normal text-gray-500">{{ node.name }}</h5>
    </div>
    <div class="px-6 py-4 text-sm">
      <slot />
    </div>
  </div>
</template>
<script lang="ts" setup>
import type { FlowNode } from "@/types";

const props = defineProps<{
  label: string;
  node: FlowNode;
}>();
</script>
