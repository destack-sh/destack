<script lang="ts" setup>
import { NodeType, type NodeReferenceData } from "@/proto/wire";
import { toCamelName } from "@/system/lang";

const props = defineProps<{ node?: NodeReferenceData; isConnected: boolean }>();
</script>
<template>
  <div class="flex flex-col justify-center text-center">
    <template v-if="isConnected">
      <!-- Not found -->
      <span>
        <i class="fas fa-exclamation-triangle mr-1.5 text-gray-500" />
        <span class="text-gray-600">{{ node != null ? toCamelName(NodeType, node.type) : "Node" }} Not Found</span>
      </span>
      <!-- TODO :UX: help to restore node if not found (and is accessible, else help with policies) -->
    </template>
    <template v-else>
      <!-- Loading -->
      <Transition
        enter-from-class="opacity-0"
        enter-active-class="transition-opacity duration-200"
        enter-to-class="opacity-100"
        appear
      >
        <i class="fas fa-spinner-third animate-spin text-gray-400" />
      </Transition>
    </template>
  </div>
</template>
