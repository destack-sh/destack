<script lang="ts" setup>
import {
  isNodeActive,
  isNodeInstance,
  isProcessableNode,
  isProvisionableResourceNode,
  PRE_PROCESS_STATUSES,
} from "@/language/core/const";
import { isProcessActive } from "@/language/runtime/process";
import {
  AnyNodeData,
  ColorShade,
  NodeMode,
  NodeModeOptionInfo,
  NodeType,
  ObjectType,
  ProcessStatus,
  ProcessStatusOptionInfo,
  ResourceStatusOptionInfo,
} from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { getColorHex } from "@/ui/style";
import { humanizeBytes } from "@/utils/string";
import { computed } from "vue";

const props = defineProps<{
  size: "xs" | "sm" | "base" | "title" | "inherit";
  node: AnyNodeData;
  isLight?: boolean;
}>();

// :NodeReferenceStyle
const iconClass = computed(() => [
  props.size == "xs" ? "w-4 text-xs" : "",
  props.size == "sm" ? "w-5" : "",
  props.size == "base" ? "w-5 text-base" : "",
  props.size == "title" ? "w-8 text-2xl" : "",
]);
const textClass = computed(() => [
  props.size == "xs" ? ["text-xs", props.isLight ? "" : ""] : "",
  props.size == "sm" ? ["", props.isLight ? "" : ""] : "",
  props.size == "base" ? ["text-xl", props.isLight ? "font-medium" : "font-bold"] : "",
  props.size == "title" ? ["text-3xl", props.isLight ? "font-medium" : "font-bold"] : "",
]);
</script>
<template>
  <div class="flex flex-row items-center gap-x-0.5">
    <!-- Node mode -->
    <span
      v-if="'mode' in node && node.mode != NodeMode.MAIN"
      v-tooltip="{
        title: NodeModeOptionInfo[node.mode]!.title,
        text: NodeModeOptionInfo[node.mode]!.text,
        small: true,
      }"
      class="w-5 px-1 py-0.5 text-center text-gray-900"
      :class="[NodeModeOptionInfo[node.mode]!.icon]"
      :style="{
        color: getColorHex(NodeModeOptionInfo[node.mode]!.color!, ColorShade.S500),
      }"
    />
    <!-- Resource metadata -->
    <span
      v-if="isProvisionableResourceNode(node) && isNodeActive(node)"
      class="fas fa-circle-small w-5 text-center"
      :class="[iconClass]"
      :style="{ color: getColorHex(ResourceStatusOptionInfo[node.status]!.color!) }"
    />
    <!-- File metadata -->
    <span v-if="isNode(node, NodeType.FILE)" class="ml-0.5 shrink-0 text-xs text-gray-400">
      {{ humanizeBytes(Number(node.size)) }}
    </span>
    <!-- Run metadata -->
    <span
      v-if="isProcessableNode(node) && node.status != ProcessStatus.IDLE && !PRE_PROCESS_STATUSES.includes(node.status)"
      class="fas fa-circle-small relative w-5 text-center"
      :class="[iconClass, isProcessActive(node) ? '' : '']"
      :style="{ color: getColorHex(ProcessStatusOptionInfo[node.status]!.color!, ColorShade.S500) }"
    >
      <span v-if="isProcessActive(node)" class="fas fa-circle-small absolute inset-0 animate-ping" />
    </span>
  </div>
</template>
