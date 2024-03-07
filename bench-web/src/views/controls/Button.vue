<script lang="tsx" setup>
import { type ViewEmits, type ViewPropsAnchored } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<Pick<ViewPropsAnchored, "self" | "name" | "title" | "text" | "icon"> & {}>();
const emits = defineEmits<ViewEmits & {}>();

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div></div>
</template>