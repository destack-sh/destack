<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { makeViewId } from "@/views";
import { useGetConnection, type PreparedGetConnection } from "@/system/connection";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import { IconInline, getNodeIcon } from "@/system/icon";
import type { ActionMapImplementation } from "@/system/action";
import { startDragging } from "@/utils/drag";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Pick<
    ViewData,
    "variant" | "nodePtr"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const fieldRef = ref<HTMLElement | null>(null);
const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.FIELD>>;
const { graph: pkgGraph, connection: pkgConnection } =
  props.preparedConnection ??
  useGetConnection(
    { name: `field.${nodePtr.value.id}` },
    computed(() => ({ roots: [nodePtr.value], isEnabled: nodePtr.value != null })),
  );
const field = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

// actions
// nocheckin: Field.actions
const actions: Partial<ActionMapImplementation<"common">> = {
  "common.edit.rename": {
    action: () => {},
  },
};

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div
    ref="fieldRef"
    v-if="field"
    class="flex w-fit flex-row items-center rounded border border-gray-300 bg-white px-1.5 py-[3px] hover:border-primary-900 hover:bg-primary-100"
  >
    <!-- nocheckin: Field -->
    <IconInline v-bind="getNodeIcon(field)" class="mr-1 w-5" />
    <span>{{ field.name }}</span>
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
