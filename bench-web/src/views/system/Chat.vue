<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<Pick<ViewData, "name" | "title" | "nodePtr">>
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>Chat!</div>
</template>
