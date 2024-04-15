<script lang="tsx" setup>
import { NodeType } from "@/proto/wire";
import type { NodeReferenceData } from "@/proto/wire";
import { fireActionById } from "@/system/action";
import type { ReadNodeGraph } from "@/system/graph";
import { IconInline } from "@/system/icon";
import { getNodeIcon } from "@/system/lang";
import { computed } from "vue";

const props = defineProps<{
  width: number;
  height: number;
  self?: NodeReferenceData;
  focus?: NodeReferenceData;
  graph: ReadNodeGraph;
}>();

const ancestors = props.graph.getAncestorsRef(
  computed(() => props.focus ?? props.self),
  { includeSelf: true, metatypes: [NodeType.BLOCK, NodeType.VIEW, NodeType.STEP, NodeType.FIELD] },
);
const path = computed(() => ancestors.value.slice().reverse());
</script>
<template>
  <div class="flex w-full max-w-full flex-row px-2.5" :style="{ height: height + 'px' }">
    <!-- Breadcrumb -->
    <div class="flex flex-row items-center gap-x-1.5 truncate">
      <template v-for="(node, i) in path" :key="i">
        <span
          class="flex flex-row items-center"
          :class="node.id == self?.id || node.id == focus?.id ? 'text-primary-900' : 'text-gray-600'"
        >
          <IconInline v-bind="getNodeIcon(node)" class="mr-1" />
          <span class="">{{ node.name }}</span>
        </span>
        <i v-if="i < path.length - 1" class="fas fa-chevron-right text-xs text-gray-500" />
      </template>
    </div>
    <!-- Search & such -->
    <div class="ml-auto flex flex-shrink-0 flex-row gap-x-1.5">
      <button
        class="rounded-md px-1 text-gray-500 hover:bg-gray-100 hover:text-primary-900"
        @click="fireActionById('common.search.findInView')"
      >
        <i class="fas fa-magnifying-glass text-xs" />
      </button>
    </div>
  </div>
</template>
