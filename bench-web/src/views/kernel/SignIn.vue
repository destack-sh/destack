<script lang="tsx" setup>
import { type ViewEmits, type ViewPropsAnchored } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<Pick<ViewPropsAnchored, "self" | "name" | "title" | "text" | "icon" | "nodePtr"> & {}>();
const emits = defineEmits<ViewEmits & {}>();

defineExpose({ node: toRef(props, "nodePtr") });
</script>
<template>
  <div>
    
  </div>
</template>