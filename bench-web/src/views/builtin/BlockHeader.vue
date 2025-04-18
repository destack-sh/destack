<script lang="ts" setup>
import { canvas } from "@/globals";
import { CANVAS_BLOCK_TYPES } from "@/language/core/const";
import { BlockType, PageNodeData, NodeReferenceData } from "@/proto/wire";
import { PreparedNodeConnection } from "@/system/connection";
import { VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { FocusAnchor, NavigationDirection, ViewEmits, ViewExpose } from "@/views/common";
import { computed, ref, Ref } from "vue";

const props = defineProps<{
  node: PageNodeData;
  connection: PreparedNodeConnection;
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

defineExpose<Partial<ViewExpose> & { focusIdentifier: (anchor: FocusAnchor) => void }>({
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
      size="base"
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
      class="cursor-pointer rounded-sm px-1 text-base text-gray-400 opacity-0 transition-opacity duration-75 group-focus-within/block-line:opacity-100 group-hover/block:opacity-100 group-hover/block-line:opacity-100 hover:bg-gray-100 hover:text-gray-700"
      @click="() => canvas.goToNode(node!)"
    >
      <i class="fas fa-arrow-up-right" />
    </button>

    <!-- Right slot -->
    <div class="mr-0.5 ml-auto flex flex-row items-center gap-x-2">
      <slot name="right" />
    </div>
  </div>
</template>
