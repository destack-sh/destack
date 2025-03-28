<script lang="ts" setup>
import { ViewData, NodeType, ComputerData } from "@/proto/wire";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, Ref, toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";
import RootHeader from "@/views/builtins/RootHeader.vue";
import { useAutoConnection } from "@/system/connection";
import Vnc from "@/views/builtins/Vnc.vue";
import { useElementSize } from "@vueuse/core";

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

const containerRef: Ref<HTMLDivElement | null> = ref(null);
const vncRef: Ref<InstanceType<typeof Vnc> | null> = ref(null);

const containerSize = useElementSize(containerRef);

function focus() {
  vncRef.value?.focus();
}

defineExpose<ViewExpose>({ self, id, focus });
</script>
<template>
  <div class="flex h-full w-full flex-col bg-white text-gray-900">
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus="props.focus" :graph="graph" />
    <div ref="containerRef" class="relative flex w-full flex-1 items-center justify-center p-[32px]">
      <!-- VNC view -->
      <div
        v-if="computer?.vncUri"
        class="h-auto w-full rounded-2xl border overflow-hidden border-gray-200 bg-gray-100"
        :style="{
          aspectRatio: computer?.width && computer?.height ? `${computer.width} / ${computer.height}` : 'auto',
          maxHeight: '100%',
        }"
      >
        <Vnc ref="vncRef" class="h-full w-full" :url="computer?.vncUri" auto-connect scale-viewport />
      </div>
      <!-- nocheckin: Computer VNC/streaming view -->
      <div v-else>
        <!-- ... -->
      </div>
    </div>
  </div>
</template>
