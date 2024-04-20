<script lang="ts" setup>
import { NodeType, ViewData } from "@/proto/wire/";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { canvas } from "@/system/space";
import { makeViewId } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW> } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div class="rounded-md border border-gray-300 bg-white">
    <slot>
      <!-- TODO :Incomplete: default group content from inner nodes -->
    </slot>
  </div>
</template>
