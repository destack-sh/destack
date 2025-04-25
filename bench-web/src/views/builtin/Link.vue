<script lang="ts" setup>
import { ReadNodeGraph } from "@/language/core/graph";
import { LinkData, ViewData } from "@/proto/wire";
import TextLine from "@/views/content/TextLine.vue";
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
    class="group/link flex cursor-pointer flex-row items-center gap-x-1.5 rounded-full border py-1 pr-3 pl-2 text-sm select-none"
    role="link"
    target="_blank"
    :href="node?.url"
  >
    <template v-if="node != null">
      <!-- Image -->
      <img v-if="node.faviconUrl" :src="node.faviconUrl" class="h-4 w-4 rounded-full" />
      <!-- Domain -->
      <span
        class="truncate text-gray-700 underline decoration-transparent underline-offset-3 transition-colors duration-150 group-hover/link:decoration-gray-300"
        >{{ node.domain }}</span
      >
    </template>
  </a>
</template>
