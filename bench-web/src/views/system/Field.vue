<script lang="ts" setup>
import { FieldType, NodeType, Orientation, Variant, ViewData } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import { focusInElement } from "@/ui/view";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
import { MaybeElement } from "@vueuse/core";
import { computed, nextTick, ref, toRef } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedGetConnection } & Pick<
    ViewData,
    "variant" | "nodePtr" | "orientation"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const fieldRef = ref<HTMLElement | null>(null);
const nameRef = ref<InstanceType<typeof NativeInput> | null>(null);

const nodePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.FIELD>);
const { graph: graph, connection: connection } = props.preparedConnection ?? useExistingConnection(nodePtr);
const field = graph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const isInspected = computed(() => canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(nodePtr.value));
const isSelected = computed(() => state.isSelected(nodePtr.value));

// actions
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"type"> = {
  // common
  "common.edit.rename": {
    action: () => {
      nextTick(() => focusInElement(nameRef.value as MaybeElement));
    },
  },
  // type
  "type.edit.isList": {
    isChecked: () => field.value?.isList ?? false,
    action: () => {
      if (field.value == null) return;
      connection.tx.update(field.value!, { isList: !field.value!.isList });
    },
  },
  "type.edit.isRequired": {
    isChecked: () => field.value?.isRequired ?? false,
    action: () => {
      if (field.value == null) return;
      connection.tx.update(field.value!, { isRequired: !field.value!.isRequired });
    },
  },
  "type.edit.isSecret": {
    isChecked: () => field.value?.isSecret ?? false,
    action: () => {
      if (field.value == null) return;
      connection.tx.update(field.value!, { isSecret: !field.value!.isSecret });
    },
  },
};

defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="field"
    ref="fieldRef"
    class="flex items-center gap-x-1.5 transition-colors duration-75"
    :class="[
      variant != Variant.STEALTH ? 'w-fit border border-gray-200' : '',
      orientation != Orientation.HORIZONTAL_REVERSED ? 'flex-row' : 'flex-row-reverse',
      isInspected || isHighlighted || isSelected ? 'bg-gray-100' : 'hover:bg-gray-100',
      field.type == FieldType.OPTION ? 'rounded-2xl pr-2.5' : '',
      field.type == FieldType.INPUT || field.type == FieldType.OUTPUT || field.type == FieldType.MEMBER
        ? 'rounded pr-2'
        : '',
      field.type == FieldType.VARIABLE ? 'rounded-2xl pr-2.5' : '',
    ]"
  >
    <!-- Icon -->
    <NodeReference class="px-1 py-[3px]" :node="field" :tx="() => connection.tx" size="regular" light is-input />
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :connection="connection" />
</template>
