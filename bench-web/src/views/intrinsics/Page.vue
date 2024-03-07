<script lang="tsx" setup>
import { getNodeRef } from "@/system/graph";
import { type ViewEmits, type ViewPropsAnchored } from "@/views/common";
import { computed, toRef } from "vue";

const props = defineProps<ViewPropsAnchored & {}>();
const emits = defineEmits<ViewEmits>();

const self = getNodeRef(toRef(props, "self"));

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div></div>
</template>