<script lang="ts" setup>
import { NodeType, type NodeReferenceData } from "@/proto/wire";
import { toCamelName } from "@/system/lang";
import { Casing, toCasing } from "@/utils/string";

const props = defineProps<{ node?: NodeReferenceData; isConnected: boolean }>();
</script>
<template>
  <div class="flex flex-col justify-center text-center">
    <template v-if="isConnected">
      <!-- Not found -->
      <i class="fas fa-exclamation-triangle text-gray-500" />
      <p class="text-gray-600">{{ node != null ? toCamelName(NodeType, node.type) : "Node" }} Not Found</p>
      <!-- TODO :UX: help to restore node if not found (and is accessible, else help with policies) -->
    </template>
    <template v-else>
      <!-- Loading -->
      <i class="fas fa-spinner-third animate-spin text-gray-400" />
    </template>
  </div>
</template>
