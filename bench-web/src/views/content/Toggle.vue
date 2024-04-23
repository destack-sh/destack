<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { ViewContentWrapper, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { ref, toRef } from "vue";
import { makeViewId } from "@/views";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: boolean } & Partial<
    Pick<
      ViewData,
      "name" | "title" | "text" | "icon" | "nodePtr" | "variant" | "orientation" | "isInput" | "isDisabled"
    >
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);
const inputRef = ref<HTMLInputElement | null>(null);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <ViewContentWrapper v-bind="props">
		<!-- nocheckin: Toggle -->
		<input type="checkbox" :checked="modelValue" />
  </ViewContentWrapper>
</template>
