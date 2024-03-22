<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import { useGetNodes, useLoadedGraph } from "@/system/connection";
import { canvas } from "@/system/space";
import { useFloating } from "@/utils/floating";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, ref, toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());

const self = toRef(props, "self");
const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(self);
const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  computed(() => ({
    roots: [props.nodePtr!],
    options: { descendantTypes: [NodeType.BLOCK] },
    enabled: props.nodePtr != null,
  })),
);
// const blocks = pkgGraph.getDescendantsRef(self, NodeType.BLOCK, )
const LOREM =
  "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Phasellus luctus rhoncus nulla sit amet mattis. Fusce ultricies quis sapien sit amet pulvinar. Nam congue metus metus, sit amet lobortis tellus pulvinar ut. Curabitur semper velit justo, in aliquam diam luctus ut. Nunc at tempus velit, at faucibus arcu. Nam facilisis vitae orci in ullamcorper. Integer faucibus vulputate erat, id scelerisque sem ornare venenatis. Fusce sagittis at dui id euismod. Duis lacinia enim sit amet neque feugiat, quis blandit est posuere. Mauris dictum varius ante, et ornare purus porttitor vitae. Ut et ex id est semper tincidunt in at risus. Curabitur dictum scelerisque scelerisque. Etiam porta bibendum sapien, eget pellentesque massa eleifend ac. Sed ac consectetur magna, ac placerat eros. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae; Maecenas dapibus at eros et luctus. Nam et luctus ex. Etiam vitae elit non odio tincidunt rhoncus. Interdum et malesuada fames ac ante ipsum primis in faucibus. Aliquam eget justo ante. Morbi imperdiet rhoncus neque, in convallis ligula volutpat vel. Phasellus nec tortor feugiat, tristique lectus at, facilisis sem. Donec vel mauris velit.";

canvas.registerSelf(self);
defineExpose({ self });
</script>
<template>
  <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md">
    <!-- Testing -->
    <div class="flex h-[150%] w-full flex-col items-center justify-center bg-secondary-100">
      <span class="text-xl">{{ self.id }}</span>
      <span class="text-3xl font-bold" v-tooltip="{ title: self.id, text: LOREM }">{{ size }}</span>
    </div>
  </Scroll>
</template>
