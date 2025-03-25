<script lang="ts" setup>
import { isResourceNode, toCamelName } from "@/language/core/const";
import {
  AnyNodeData,
  ColorShade,
  NodeMode,
  NodeModeOptionInfo,
  NodeType,
  ResourceStatusOptionInfo,
  RunStatusOptionInfo,
} from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { getColorHex } from "@/ui/style";
import { humanizeBytes } from "@/utils/string";
import { computed } from "vue";

const props = defineProps<{
  size: "sm" | "base" | "title" | "inherit";
  node: AnyNodeData;
  isLight?: boolean;
}>();

// :NodeReferenceStyle
const iconClass = computed(() => [
  props.size == "sm" ? "w-5" : "",
  props.size == "base" ? "w-5 text-base" : "",
  props.size == "title" ? "w-8 text-2xl" : "",
]);
const textClass = computed(() => [
  props.size == "sm" ? ["", props.isLight ? "" : ""] : "",
  props.size == "base" ? ["text-xl", props.isLight ? "font-medium" : "font-bold"] : "",
  props.size == "title" ? ["text-3xl", props.isLight ? "font-medium" : "font-bold"] : "",
]);
</script>
<template>
  <div>
    <!-- Resource metadata -->
    <span
      v-if="isResourceNode(node) && node.mode < NodeMode.TEMPLATE"
      class="fas fa-circle-small w-5 text-center"
      :class="iconClass"
      :style="{ color: getColorHex(ResourceStatusOptionInfo[node.status]!.color!) }"
    />
    <!-- File metadata -->
    <span v-if="isNode(node, NodeType.FILE)" class="ml-0.5 flex-shrink-0 text-xs text-gray-400">
      {{ humanizeBytes(Number(node.size)) }}
    </span>
    <!-- Run metadata -->
    <span
      v-if="isNode(node, NodeType.RUN)"
      class="fas fa-circle-small w-5 text-center"
      :class="iconClass"
      :style="{ color: getColorHex(RunStatusOptionInfo[node.status]!.color!) }"
    />
    <!-- Node mode, ... -->
    <span
      v-if="'mode' in node && node.mode != NodeMode.MAIN"
      class="ml-0.5 rounded-sm border px-1 py-0.5 text-xs text-gray-900"
      :style="{
        backgroundColor: getColorHex(NodeModeOptionInfo[node.mode]!.color!, ColorShade.S200),
        borderColor: getColorHex(NodeModeOptionInfo[node.mode]!.color!, ColorShade.S300),
      }"
    >
      {{ toCamelName(NodeMode, node.mode) }}
    </span>
  </div>
</template>
