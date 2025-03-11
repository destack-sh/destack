<script lang="ts" setup>
import { FieldType, NodeType, Orientation, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapKit } from "@/ui/action";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { computed, nextTick, ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "isMinimal" | "nodePtr" | "orientation">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const fieldRef = ref<HTMLElement | null>(null);
const nameRef = ref<InstanceType<typeof NodeReference> | null>(null);

const nodePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.FIELD>);
const { graph: graph, connection: connection } = props.preparedConnection ?? useExistingConnection(nodePtr);
const field = graph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const isInspected = computed(() => canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(nodePtr.value));
const isSelected = computed(() => state.isSelected(nodePtr.value));

// actions
const actions: Partial<ActionMapKit<"space">> = {
  // space
  "space.edit.rename": {
    action: () => {
      nextTick(() => nameRef.value?.focusIdentifier());
    },
  },
};

defineExpose<ViewExpose>({ self, id, actions });
</script>
<template>
  <div
    v-if="field"
    ref="fieldRef"
    class="flex items-center gap-x-1.5 transition-colors duration-75"
    :class="[
      !isMinimal ? 'w-fit border' : '',
      orientation != Orientation.HORIZONTAL_REVERSED ? 'flex-row' : 'flex-row-reverse',
      isSelected ? 'border-gray-400 bg-orange-400/20' : 'border-gray-200 hover:bg-gray-100',
      !isSelected && (isInspected || isHighlighted) ? 'bg-gray-100' : '',
      field.type == FieldType.RESOURCE
        ? !isMinimal
          ? 'rounded-2xl pr-2.5'
          : 'rounded'
        : '',
      field.type == FieldType.INPUT || field.type == FieldType.OUTPUT || field.type == FieldType.MEMBER
        ? 'rounded pr-2'
        : '',
    ]"
    data-suppress-drag="select"
  >
    <NodeReference
      ref="nameRef"
      :class="[!isMinimal ? 'px-1 py-[3px]' : '']"
      :node="field"
      :tx="() => connection.tx"
      size="sm"
      is-light
      is-input
      is-minimal
    />
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :connection="connection" />
</template>
