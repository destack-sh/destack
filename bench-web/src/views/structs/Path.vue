<script lang="ts" setup>
import { ViewData, NodeType, PathData } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "valueType">
  >
>();
const modelValue = defineModel<PathData | undefined>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <!-- nocheckin: Path view -->
    <span class="text-red-500">Path!</span>
    {{ modelValue }}
  </div>
</template>
