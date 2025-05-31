<script lang="ts" setup>
import { ReadNodeGraph } from "@/language/core/graph";
import { LinkData, ViewData } from "@/proto/wire";
import { Ref, toRef } from "vue";

const props = defineProps<
  {
    graph: ReadNodeGraph;
  } & Pick<ViewData, "nodePtr">
>();

const nodePtr = toRef(props, "nodePtr");
const node = props.graph.getRef(nodePtr) as Ref<LinkData | null>;
</script>
<template>
  <a
    class="group/link flex h-[32px] cursor-pointer flex-row items-center gap-x-1.5 rounded-full border py-1 pr-3 pl-2 text-sm transition-colors duration-75 select-none hover:bg-gray-100"
    role="link"
    target="_blank"
    :href="node?.url"
  >
    <template v-if="node != null">
      <!-- Image -->
      <img v-if="node.faviconUrl" :src="node.faviconUrl" class="h-4 w-4 rounded-full" />
      <span v-else class="fas fa-link text-gray-400" />
      <!-- Domain -->
      <span
        class="truncate text-gray-700 underline decoration-transparent underline-offset-3 transition-colors duration-75 group-hover/link:decoration-gray-300"
        >{{ node.domain }}</span
      >
    </template>
  </a>
</template>
