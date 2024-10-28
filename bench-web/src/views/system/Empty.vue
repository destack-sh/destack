<script lang="ts" setup>
import { BoxData, NodeType, Orientation, ViewData } from "@/proto/wire/";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { ScrollbarWidth } from "@/ui/layout";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import { toRef } from "vue";

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string; size: Required<Pick<BoxData, "width" | "height">> } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "deletedAt"
  >
>();
const emit = defineEmits(viewEmits());
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
