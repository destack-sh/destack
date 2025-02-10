<script lang="ts" setup>
import { canvas } from "@/globals";
import { CANVAS_BLOCK_TYPES } from "@/language/core/const";
import { BlockType, InlineSourceNodeData, NodeReferenceData } from "@/proto/wire";
import { PreparedGetConnection } from "@/system/connection";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { FocusAnchor, NavigationDirection, ViewEmits, ViewExposed } from "@/views/common";
import { computed, ref, Ref } from "vue";

const props = defineProps<{
  node: InlineSourceNodeData;
  connection: PreparedGetConnection;
}>();
const { graph, connection } = props.connection;
const hasCanvas = computed(() => CANVAS_BLOCK_TYPES.includes(props.node.metatype as unknown as BlockType));
const emit = defineEmits<ViewEmits>();
const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);

function focusIdentifier(anchor: FocusAnchor) {
  nameRef.value?.focusIdentifier(anchor);
}

function focusIcon() {
  nameRef.value?.focusIcon();
}

function focus(anchor?: FocusAnchor | NodeReferenceData) {
  focusIdentifier(typeof anchor == "string" ? anchor : "top");
}

defineExpose<Partial<ViewExposed> & { focusIdentifier: (anchor: FocusAnchor) => void }>({
  focusIdentifier,
  focus,
});
</script>
<template>
  <!-- Header -->
  <div
    class="group/header flex flex-row items-center rounded-t border-b border-gray-200 px-1"
    :style="{
      height: VIEW_DEFAULT_HEADER_HEIGHT + 'px',
    }"
  >
    <!-- Title -->
    <NodeReference
      ref="nameRef"
      size="large"
      :node="node"
      is-input
      :tx="() => connection.tx"
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    />
    <!-- Left slot -->
    <div class="ml-1 flex flex-row items-center gap-x-2">
      <slot name="left" />
    </div>
    <!-- Open in its own view -->
    <button
      v-if="hasCanvas"
      v-tooltip="{ small: true, text: 'Open in full' }"
      class="ml-1 rounded px-1 text-base text-gray-400 opacity-0 transition-opacity duration-75 hover:bg-gray-100 hover:text-gray-700 group-focus-within/block-line:opacity-100 group-hover/block-line:opacity-100 group-hover/block:opacity-100"
      @click="() => canvas.goToNode(node!)"
    >
      <i class="fas fa-arrow-up-right" />
    </button>

    <!-- Right slot -->
    <div class="ml-auto flex flex-row items-center gap-x-2">
      <slot name="right" />
    </div>
  </div>
</template>
