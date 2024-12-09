<script lang="ts" setup>
import { _setDragImage, activeDrag } from "@/ui/drag";
import NodeReference from "@/views/builtins/NodeReference.vue";
</script>
<template>
  <!-- Drag image overlay -->
  <!-- Wrapper to ensure dragImageRef is always set -->
  <div :ref="(ref) => _setDragImage(ref as any)" class="absolute -top-[100px] left-20 py-1 pl-2">
    <div v-if="activeDrag" class="">
      <!-- And wrapper to offset within the image to ensure the text isn't obscured by the cursor -->
      <div v-if="activeDrag.kind == 'node' || activeDrag.kind == 'selection'" class="flex flex-row items-center gap-x-1">
        <NodeReference
          v-for="node in activeDrag.nodes.slice(0, 3)"
          :key="node.id"
          :node="node"
          size="regular"
          class="max-w-48 rounded-2xl border border-gray-300 bg-white px-1.5 py-0.5 text-gray-900"
        />
        <div v-if="activeDrag.nodes.length > 3" class="text-gray-400">+{{ activeDrag.nodes.length - 3 }}</div>
      </div>
      <div v-else>
        <span class="text-danger-600">???</span>
      </div>
    </div>
  </div>
</template>
