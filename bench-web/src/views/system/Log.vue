<script lang="ts" setup>
import { ViewData, NodeType, LogData } from "@/proto/wire";
import { describeNode, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "isInline">
  > & { node?: LogData }
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>
    Log!
    {{ nodePtr && describeNode(nodePtr) }}
    nocheckin
  </div>
</template>
