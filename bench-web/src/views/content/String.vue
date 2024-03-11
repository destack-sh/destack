<script lang="tsx" setup>
import { NodeReferenceData, type ViewData } from "@/proto/wire";
import { viewEmits } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<
  { self?: NodeReferenceData | null } & Pick<
    ViewData,
    "name" | "title" | "text" | "icon" | "isInput" | "isDisabled" | "isSecret"
  >
>();
const emit = defineEmits(viewEmits());
const modelValue = defineModel<string>();

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div>
    this is a string input
  </div>
</template>
