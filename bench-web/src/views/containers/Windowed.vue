<script lang="tsx" setup>
import { NodeType } from "@/proto/wire";
import { getChildrenRef } from "@/system/graph";
import { type ViewEmits, type ViewProps } from "@/views/common";
import { toRef } from "vue";

const props = defineProps<Pick<ViewProps, "node" | "name" | "title" | "text" | "icon"> & {}>();
const emits = defineEmits<ViewEmits>();

const { children: windows } = getChildrenRef(toRef(props, "node"), NodeType.VIEW);

defineExpose({ node: toRef(props, "node") });
</script>
<template>
  <div class="bg-secondary-100 font-bold">{{ windows.length }} windows</div>
</template>
