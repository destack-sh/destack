<script lang="tsx" setup>
import { ViewData, NodeReferenceData } from "@/proto/wire/";
import { viewEmits } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<{self?: NodeReferenceData | null} & Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr">>();
const emit = defineEmits(viewEmits());

const self = toRef(props, "self");
defineExpose({ self });
</script>
<template>
  <div>
    bench wizard
  </div>
</template>