<script lang="tsx" setup>
import { ViewData, NodeReferenceData, ViewType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { useExistingConnection } from "@/system/connection";
import { toRef } from "vue";

const props = defineProps<
  { self: NodeReferenceData } & Pick<ViewData, "type" | "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);

canvas.registerView(self);
defineExpose<ViewExposed>({ self });
</script>
<template>
  <div class="h-full w-full bg-white">
		<!-- nocheckin :Incomplete: explorer -->
    {{ ViewType[type] }}
  </div>
</template>
