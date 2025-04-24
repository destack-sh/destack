<script lang="ts" setup>
import { ReadNodeGraph } from "@/language/core/graph";
import { RunData, ViewData } from "@/proto/wire";
import NodeReference from "@/views/builtin/NodeReference.vue";
import ProcessStatus from "@/views/builtin/ProcessStatus.vue";
import TextLine from "@/views/content/TextLine.vue";
import { Ref, toRef } from "vue";

const props = defineProps<
  {
    graph: ReadNodeGraph;
  } & Pick<ViewData, "nodePtr">
>();

const nodePtr = toRef(props, "nodePtr");
const node = props.graph.getRef(nodePtr) as Ref<RunData | null>;
</script>
<template>
  <div class="flex flex-row items-baseline gap-x-1.5 rounded-full border py-1 pr-3 pl-2 text-sm select-none">
    <!-- Runnable -->
    <NodeReference :node-ptr="node?.actionPtr" hide-metadata size="sm" />
    <!-- Title/arguments -->
    <TextLine
      v-if="node?.title"
      :model-value="node?.title"
      force-line-type="inherit"
      class="truncate text-gray-500 select-none"
      truncate
      colorless
      is-minimal
    />
    <!-- Status -->
    <ProcessStatus v-if="node?.status" :node="node" icon="dot" class="ml-0.5" />
  </div>
</template>
