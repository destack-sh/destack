<script lang="ts" setup>
import { ViewData, NodeType, Variant } from "@/proto/wire";
import { type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { toRef, type Ref } from "vue";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import Text from "@/views/content/Text.vue";

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
const message = pkgGraph.getRef(nodePtr);

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, variants: [Variant.PRIMARY, Variant.STEALTH] });
</script>
<template>
  <div v-if="message" class="flex max-w-full flex-row gap-x-2">
    <!-- nocheckin: message -->
    <!-- Author -->
    <div class="flex-shrink-0">
			{{ message.createdByPtr!.type }}
			{{ message.createdAt!.seconds }}
    </div>
    <!-- Content -->
    <div>
      <Text :model-value="message.text" :variant="Variant.STEALTH" />
    </div>
  </div>
  <Inaccessible v-else class="h-full w-full bg-white" :node="nodePtr" :is-connected="pkgConnection.isConnected.value" />
</template>
