<script lang="tsx" setup>
import { type ViewEmits, type ViewPropsAnchored } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<
  Pick<ViewPropsAnchored, "self" | "name" | "title" | "text" | "icon" | "isInput" | "isDisabled" | "isSecret"> & {}
>();
const emits = defineEmits<ViewEmits & {}>();

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div></div>
</template>
