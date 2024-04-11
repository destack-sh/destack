<script lang="tsx" setup>
import { ViewData, NodeReferenceData, NodeType } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef, type Ref } from "vue";
import { makeViewId } from "@/views";
import { useExistingConnection } from "@/system/connection";
import type { TypedNodeReferenceData } from "@/proto/wiring";

const props = defineProps<{ self?: NodeReferenceData } & Pick<ViewData, "nodePtr">>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.BLOCK>>;
const { graph: blockGraph, connection: blockConnection } = useExistingConnection(nodePtr);
const block = blockGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

// nocheckin :Incomplete: Block

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div>Block {{ block?.name }}</div>
</template>
