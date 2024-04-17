<script lang="tsx" setup>
import { ViewData, NodeReferenceData } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";
import { makeViewId } from "@/views";

const props = defineProps<
  { self?: NodeReferenceData } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "valueType" | "isInput" | "isInline" | "isDisabled"
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
    I'm a picker
    <!-- nocheckin -->
  </div>
</template>
