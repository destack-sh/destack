<script lang="tsx" setup>
import { NodeReferenceData, NodeType, ViewData } from "@/proto/wire";
import { getChildrenRef } from "@/system/graph";
import { viewEmits } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<{ self?: NodeReferenceData | null } & Pick<ViewData, "name" | "title" | "text" | "icon">>();
const emit = defineEmits(viewEmits());

const { children: tabs } = getChildrenRef(toRef(props, "self"), NodeType.VIEW);

defineExpose({ self: toRef(props, "self") });
</script>
<template>
  <div class="bg-secondary-100 font-bold">{{ tabs.length }} tabs</div>
</template>
