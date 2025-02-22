<script lang="ts" setup>
import { isResourceNode } from "@/language/core/const";
import { AnyNodeData, NodeType, ResourceStatusOptionInfo, RunStatusOptionInfo } from "@/proto/wire";
import { isNode } from "@/proto/wiring";
import { getColorHex } from "@/ui/style";
import { humanizeBytes } from "@/utils/string";
import { computed } from "vue";

const props = defineProps<{
  size: "regular" | "large" | "title";
  node: AnyNodeData;
  isLight?: boolean;
}>();

// :NodeReferenceStyle
const iconClass = computed(() => [
  props.size == "regular" ? "w-5" : "",
  props.size == "large" ? "w-5 text-base" : "",
  props.size == "title" ? "w-8 text-2xl" : "",
]);
const textClass = computed(() => [
  props.size == "regular" ? ["", props.isLight ? "" : ""] : "",
  props.size == "large" ? ["text-xl", props.isLight ? "font-medium" : "font-bold"] : "",
  props.size == "title" ? ["text-3xl", props.isLight ? "font-medium" : "font-bold"] : "",
]);
</script>
<template>
  <div>
    <span v-if="isNode(node, NodeType.FILE)">
      <span class="ml-1.5 flex-shrink-0 text-xs text-gray-400">
        {{ humanizeBytes(Number(node.size)) }}
      </span>
    </span>
    <span v-if="isResourceNode(node)">
      <!-- Resource metadata -->
      <span
        class="fas fa-circle-small w-5 text-center"
        :class="iconClass"
        :style="{ color: getColorHex(ResourceStatusOptionInfo[node.status]!.color!) }"
      />
    </span>
    <span v-else-if="isNode(node, NodeType.RUN)">
      <!-- Run metadata -->
      <span
        class="fas fa-circle-small w-5 text-center"
        :class="iconClass"
        :style="{ color: getColorHex(RunStatusOptionInfo[node.status]!.color!) }"
      />
    </span>
    <!-- Node mode, ... -->
  </div>
</template>
