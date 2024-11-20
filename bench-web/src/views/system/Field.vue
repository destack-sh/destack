<script lang="ts" setup>
import { FieldType, NodeType, Orientation, ViewData } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import type { ActionMapImplementation } from "@/ui/action";
import { focusInElement } from "@/ui/view";
import IconName from "@/views/builtins/IconName.vue";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import NativeInput from "@/views/content/NativeInput.vue";
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

const fieldRef = ref<HTMLElement | null>(null);
const nameRef = ref<InstanceType<typeof NativeInput> | null>(null);

const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.FIELD>);
const { graph: pkgGraph, connection: pkgConnection } = props.preparedConnection ?? useExistingConnection(nodePtr);
const field = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });
const isInspected = computed(() => canvas.isInspected(nodePtr.value));
const isHighlighted = computed(() => canvas.isHighlighted(nodePtr.value));

// actions
const actions: Partial<ActionMapImplementation<"common">> & ActionMapImplementation<"type"> = {
  // common
  "common.edit.rename": {
    action: () => {
      nextTick(() => focusInElement(nameRef.value!));
    },
  },
  // type
  "type.edit.isList": {
    isChecked: () => field.value?.isList ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isList: !field.value!.isList });
    },
  },
  "type.edit.isRequired": {
    isChecked: () => field.value?.isRequired ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isRequired: !field.value!.isRequired });
    },
  },
  "type.edit.isSecret": {
    isChecked: () => field.value?.isSecret ?? false,
    action: () => {
      if (field.value == null) return;
      pkgConnection.tx.update(field.value!, { isSecret: !field.value!.isSecret });
    },
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    v-if="field"
    ref="fieldRef"
    class="flex w-fit items-center gap-x-1.5 border border-gray-200 pl-0.5 transition-colors duration-75"
    :class="[
      orientation != Orientation.HORIZONTAL_REVERSED ? 'flex-row' : 'flex-row-reverse',
      isInspected || isHighlighted ? 'bg-gray-100' : 'hover:bg-gray-100',
      field.type == FieldType.OPTION ? 'rounded-2xl pr-2.5' : '',
      field.type == FieldType.INPUT || field.type == FieldType.OUTPUT || field.type == FieldType.MEMBER
        ? 'rounded pr-2'
        : '',
      field.type == FieldType.VARIABLE ? 'rounded-2xl pr-2.5' : '',
    ]"
  >
    <!-- Icon -->
    <IconName class="px-1 py-[3px]" :node="field" :tx="() => pkgConnection.tx" size="regular" light is-input />
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :connection="pkgConnection" />
</template>
