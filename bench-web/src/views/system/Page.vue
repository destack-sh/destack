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

const referenceRef = ref<HTMLElement | null>(null);
const referencePos = ref({ top: 20, left: 20 });
const referenceMoving = ref(false);
const floatingRef = ref<HTMLElement | null>(null);
const arrowRef = ref<HTMLElement | null>(null);

const { recompute } = useFloating({
  floating: floatingRef,
  reference: referenceRef,
  arrow: arrowRef,
  options: { margin: 10, placement: "top" },
  watchElements: true,
});

canvas.registerSelf(self);
defineExpose({ self });
</script>
<template>
  <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md">
    <!-- Testing -->
    <div
      class="flex h-[150%] w-full flex-col items-center justify-center bg-green-100"
      @mouseup="
        () => {
          referenceMoving = false;
        }
      "
      @mousemove="
        (e) => {
          if (referenceMoving) {
            referencePos.top += e.movementY;
            referencePos.left += e.movementX;
            recompute();
          }
        }
      "
    >
      <span class="text-xl">{{ self.id }}</span>
      <span class="text-3xl font-bold">{{ size }}</span>
      <!-- Reference for floating/tooltip test -->
      <div
        ref="referenceRef"
        class="absolute h-[200px] w-[500px] rounded-md border border-gray-700 bg-white shadow-md shadow-gray-700 hover:cursor-pointer"
        :style="{ top: referencePos.top + 'px', left: referencePos.left + 'px' }"
        @mousedown="
          () => {
            referenceMoving = true;
          }
        "
      />
      <!-- Floating -->
      <div ref="floatingRef" class="h-[20px] w-[80px] rounded-md border bg-red-300" />
    </div>
  </Scroll>
</template>
