<script lang="ts" setup>
import { ViewData, NodeType, TypeInfoData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";
import { makeViewId } from "@/views";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; modelValue?: TypeInfoData } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "isInline" | "isInput" | "isDisabled">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    <!-- nocheckin: Type -->
    Type!
  </div>
</template>
