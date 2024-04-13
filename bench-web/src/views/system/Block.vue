<script lang="tsx" setup>
import { NodeReferenceData, NodeType, ViewData } from "@/proto/wire";
import type { TypedNodeReferenceData } from "@/proto/wiring";
import { useGetConnection, type PreparedGetConnection } from "@/system/connection";
import { IconInline } from "@/system/icon";
import { getNodeIcon } from "@/system/lang";
import { onMouseNotPressedOnce } from "@/utils/layout";
import { canvas } from "@/system/space";
import { makeViewId } from "@/views";
import { viewEmits, type ViewExposed } from "@/views/common";
import Inaccessible from "@/views/private/Inaccessible.vue";
import { computed, toRef, type Ref, ref } from "vue";

const props = defineProps<
  { self?: NodeReferenceData; preparedConnection?: PreparedGetConnection } & Pick<ViewData, "nodePtr">
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const blockRef = ref<HTMLElement | null>(null);
const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.BLOCK>>;
const { graph: pkgGraph, connection: pkgConnection } =
  props.preparedConnection ??
  useGetConnection(
    { name: `block.${nodePtr.value.id}` },
    computed(() => ({
      roots: [nodePtr.value],
      options: { descendantTypes: [NodeType.FIELD, NodeType.VIEW, NodeType.STEP, NodeType.TRIGGER] },
      enabled: nodePtr.value != null,
    })),
  );
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

// nocheckin :Incomplete: Block

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div ref="blockRef" v-if="block" class="group/block">
    <!-- Icon/Name -->
    <span @mousedown="() => (blockRef!.draggable = true, onMouseNotPressedOnce(() => blockRef!.draggable = false))">
      <IconInline v-bind="getNodeIcon(block)" class="mr-1 text-gray-600" />
      <span>{{ block?.name }}</span>
    </span>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
