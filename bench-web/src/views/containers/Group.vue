<script lang="tsx" setup>
import { ViewData, NodeReferenceData } from "@/proto/wire/";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef } from "vue";
import { makeViewId } from "@/views";

const props = defineProps<
  { self?: NodeReferenceData } & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div class="rounded-md border border-gray-300 bg-white shadow-md shadow-gray-300">
    <slot>
      <!-- TODO :Incomplete: default group content from inner nodes -->
    </slot>
  </div>
</template>
