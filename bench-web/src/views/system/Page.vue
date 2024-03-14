<script lang="tsx" setup>
import { ViewData, NodeReferenceData, NodeType, Orientation, BoxData } from "@/proto/wire/";
import { useGetNodes, useLoadedGraph } from "@/system/connection";
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
const { graph: spaceGraph, connection: spaceConnection } = useLoadedGraph(self);
const { graph: pkgGraph, connection: pkgConnection } = useGetNodes(
  computed(() => ({
    roots: [props.nodePtr!],
    options: { descendantTypes: [NodeType.BLOCK] },
    enabled: props.nodePtr != null,
  })),
);
// const blocks = pkgGraph.getDescendantsRef(self, NodeType.BLOCK, )

defineExpose({ self });
</script>
<template>
  <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md">
    <!-- Placeholder content for testing vertical scrolling -->
    <div class="mx-auto max-w-[600px] px-6 py-6">
      <p v-for="i in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]" :key="i" class="mb-5">
        <span class="block text-2xl font-medium text-gray-900">{{ title }} {{ i }}</span>
        <span class="text-gray-500"
          >Lorem ipsum dolor sit amet, consectetur adipiscing elit. Mauris laoreet consectetur venenatis. Donec pulvinar
          lorem ex, consectetur dapibus tortor faucibus et. Orci varius natoque penatibus et magnis dis parturient
          montes, nascetur ridiculus mus. Aliquam lacinia arcu porta lectus dignissim, sed euismod purus semper. Aenean
          posuere arcu a nisl interdum, a sollicitudin lectus condimentum. Aliquam porttitor quis orci vitae
          sollicitudin. Cras consectetur ultricies metus, vitae sagittis felis tincidunt ac. Nulla congue erat ut
          facilisis faucibus. In ut viverra purus, vel sagittis nisi. Morbi consequat, urna eget lobortis gravida,
          turpis nibh bibendum ipsum, in lobortis nisi mauris eget justo. Curabitur urna metus, ultricies at sapien vel,
          pretium elementum magna. Integer lacinia massa sed dui dapibus laoreet. Nulla consequat, dolor quis dictum
          porttitor, mauris magna aliquam justo, ac accumsan diam ipsum vel felis. Suspendisse varius metus leo, at
          venenatis diam condimentum vitae.</span
        >
      </p>
    </div>
  </Scroll>
</template>
