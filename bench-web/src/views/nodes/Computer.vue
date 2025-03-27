<script lang="ts" setup>
import { ViewData, NodeType, ComputerData } from "@/proto/wire";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, Ref, toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";
import RootHeader from "@/views/builtins/RootHeader.vue";
import { useAutoConnection } from "@/system/connection";
import Vnc from "@/views/builtins/Vnc.vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; isRoot?: boolean } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "focus" | "isMinimal">
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const nodePtr = computed(() => props.nodePtr);
const state = canvas.registerView(self, id);

const { graph, connection } = useAutoConnection(nodePtr);
const computer = graph.getRef(nodePtr) as Ref<ComputerData | undefined>;

defineExpose<ViewExpose>({ self, id });
</script>
<template>
  <div class="flex h-full w-full select-none flex-col bg-white text-gray-900">
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus="props.focus" :graph="graph" />
    <!-- nocheckin: Computer VNC view: novnc? guacamole? -->
    <Vnc url="ws://localhost:6080" :rfb-options="{ credentials: { password: 'bench' } }" />
  </div>
</template>
