<script lang="ts" setup>
import { ViewData, NodeType, Variant } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef, type Ref } from "vue";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "variant" | "nodePtr">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const nodePtr = toRef(props, "nodePtr") as Ref<TypedNodeReferenceData<NodeType.MESSAGE>>;
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.STEALTH] });
</script>
<template>
  <div>Message! nocheckin</div>
</template>
