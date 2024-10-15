<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

// NOTE :UX: Database view should be factored out into Table/Collection/Feed/etc. views (?)

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div ref="containerRef">
    <!-- nocheckin: Database -->
    Database!
  </div>
</template>
