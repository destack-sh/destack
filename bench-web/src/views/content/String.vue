<script lang="tsx" setup>
import type { TextData } from "@/proto/wire";
import { type ViewEmits, type ViewPropsBase } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<
  Pick<ViewPropsBase, "self" | "name" | "title" | "text" | "icon" | "isInput" | "isDisabled" | "isSecret"> & {}
>();
const emits = defineEmits<ViewEmits & {}>();
const modelValue = defineModel<string>();

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div></div>
</template>
