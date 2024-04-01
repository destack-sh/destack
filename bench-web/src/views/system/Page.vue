<script lang="tsx" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import { useExistingConnection, useGetNodes } from "@/system/connection";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { computed, toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());

const self = toRef(props, "self");
const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  {
    name: `page.${props.nodePtr?.id}`,
  },
  computed(() => ({
    roots: [props.nodePtr!],
    options: { descendantTypes: [NodeType.BLOCK] },
    enabled: props.nodePtr != null,
  })),
);
// const blocks = pkgGraph.getDescendantsRef(self, NodeType.BLOCK, )

canvas.registerView(self);
defineExpose({ self });
</script>
<template>
  <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md">
    <!-- Testing -->
    <div class="flex h-[150%] w-full flex-col items-center justify-center bg-secondary-100">
      <span class="text-xl">{{ self.id }}</span>
      <span>PAGE</span>
    </div>
  </Scroll>
</template>
