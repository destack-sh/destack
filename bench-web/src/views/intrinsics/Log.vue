<script lang="tsx" setup>
import { toNodeReference } from "@/system/graph";
import { type ViewEmits, type ViewProps } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<ViewProps & {}>();
const emits = defineEmits<ViewEmits>();
const nodeRef = computed(() => props.node != null ? toNodeReference(props.node) : null);

defineExpose({ node: toRef(props, "node") });
</script>
<template>
  <div></div>
</template>