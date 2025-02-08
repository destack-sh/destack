<script lang="ts" setup>
import { RectangleData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/ui/layout";
import { type ViewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { toRef } from "vue";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string; size: Required<Pick<RectangleData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "deletedAt"
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div>
    <Scroll id="scroll" :size="size" :orientation="Orientation.VERTICAL" :track-width="ScrollbarWidth.md" class="bg-white">
      <div class="flex h-[150%] w-full flex-col items-center justify-center">
        <span class="text-xl">{{ self.id }}</span>
        <span v-if="deletedAt">deleted:{{ deletedAt }}</span>
        <span class="text-3xl font-bold">{{ size }}</span>
      </div>
    </Scroll>
  </div>
</template>
