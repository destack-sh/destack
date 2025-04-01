<script lang="ts" setup>
import { supergraph } from "@/globals";
import { ClaimData, NodeType, ViewData } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { PreparedNodeConnection, useAutoConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { getNodeIcon, IconInline } from "@/ui/icon";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { computed, Ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedNodeConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "isMinimal">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const nodePtr = toRef(props, "nodePtr");
const { graph, connection } = props.preparedConnection ?? useAutoConnection(nodePtr);
const claim = graph.getRef(nodePtr) as Ref<ClaimData | null>;
const targetPtr = computed(() => claim.value?.targetPtr ?? claim.value?.targetTemplatePtr);
const target = supergraph.getRef(targetPtr.value);

// view
const isInspected = computed(() => canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(nodePtr.value));
const isSelected = computed(() => nodePtr.value != null && state.isSelected(nodePtr.value));

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div
    v-if="claim"
    ref="claimRef"
    class="flex items-center gap-x-1.5 rounded transition-colors duration-150"
    :class="[
      !isMinimal ? 'border px-1 py-1' : '',
      isSelected ? 'border-gray-400 bg-orange-100' : '',
      !isSelected && (isInspected || isHighlighted) ? 'border-gray-400 bg-gray-100' : '',
      !(isSelected || isInspected || isHighlighted) ? 'border-gray-200 bg-white' : '',
    ]"
    :style="{}"
  >
    <!-- Icon (as big square if not minimal) -->
    <div v-if="!isMinimal" class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded bg-yellow-400">
      <IconInline v-bind="getNodeIcon(target ?? claim)" class="rounded text-center text-lg text-gray-800" />
    </div>

    <!-- TODO :Incomplete: show/control? actual Claim status somehow -->
    <NodeReference :node-ptr="targetPtr" :tx="() => connection.tx" :hide-icon="!isMinimal" size="sm" />
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :connection="connection" />
</template>
