<script lang="ts" setup>
import type { NodeReferenceData } from "@/proto/wire";
import { fireActionById } from "@/system/action";
import type { ReadNodeGraph } from "@/system/graph";
import NodePath from "@/views/builtins/NodePath.vue";

const props = defineProps<{
  width: number;
  height: number;
  self?: NodeReferenceData;
  focus?: NodeReferenceData;
  graph: ReadNodeGraph;
}>();
</script>
<template>
  <div class="group flex w-full max-w-full flex-row px-2.5" :style="{ height: height + 'px' }">
    <!-- Breadcrumb -->
    <NodePath :self="self" :focus="focus" :graph="graph" />
    <!-- Search & such -->
    <div class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-1.5 pl-1">
      <button class="h-fit text-gray-400 hover:text-primary-900" @click="fireActionById('common.search.findInView')">
        <i class="fas fa-magnifying-glass w-5 text-center" />
      </button>
    </div>
  </div>
</template>
