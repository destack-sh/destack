<script lang="tsx" setup>
import { NodeType } from "@/proto/wire";
import { getChildrenRef } from "@/system/graph";
import { type ViewEmits, type ViewPropsAnchored } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<Pick<ViewPropsAnchored, "self" | "name" | "title" | "text" | "icon"> & {}>();
const emits = defineEmits<ViewEmits>();

const { children: tabs } = getChildrenRef(toRef(props, "self"), NodeType.VIEW);

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div class="bg-secondary-100 font-bold">{{ tabs.length }} tabs</div>
</template>