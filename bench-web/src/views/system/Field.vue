<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { makeViewId } from "@/views";
import { useGetConnection, type PreparedGetConnection } from "@/system/connection";
import Inaccessible from "@/views/builtins/Inaccessible.vue";

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

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div v-if="field">
    {{ field.name }}
  </div>
  <Inaccessible v-else class="bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
