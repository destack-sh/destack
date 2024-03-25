<script lang="tsx" setup>
import { BoxData, NodeReferenceData, Orientation, ViewData } from "@/proto/wire/";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());

const self = toRef(props, "self");

canvas.registerView(self);
defineExpose({ self });
</script>
<template>
  <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md">
    <div class="flex h-[150%] w-full flex-col items-center justify-center bg-secondary-100">
      <span class="text-xl">{{ self.id }}</span>
      <span class="text-3xl font-bold">{{ size }}</span>
    </div>
  </Scroll>
</template>
