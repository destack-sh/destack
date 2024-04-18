<script lang="ts" setup>
import { BoxData, NodeReferenceData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/utils/layout";
import { type ViewExposed, viewEmits } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { toRef } from "vue";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "nodePtr" | "archivedAt" | "deletedAt"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <Scroll :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md" class="bg-white">
    <div class="flex h-[150%] w-full flex-col items-center justify-center">
      <span class="text-xl">{{ self.id }}</span>
      <span v-if="archivedAt">archived:{{ archivedAt }}</span>
      <span v-if="deletedAt">deleted:{{ deletedAt }}</span>
      <span class="text-3xl font-bold">{{ size }}</span>
    </div>
  </Scroll>
</template>
