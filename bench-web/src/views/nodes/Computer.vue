<script lang="ts" setup>
import { ViewData, NodeType, ComputerData, ResourceStatus, ResourceStatusOptionInfo, ColorShade } from "@/proto/wire";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, Ref, toRef } from "vue";
import { TypedNodeReferenceData } from "@/proto/wiring";
import RootHeader from "@/views/builtin/RootHeader.vue";
import { useAutoConnection } from "@/system/connection";
import Vnc from "@/views/builtin/Vnc.vue";
import { useElementSize } from "@vueuse/core";
import { getColorHex } from "@/ui/style";
import { toCamelName } from "@/language/core/const";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; isRoot?: boolean } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "focusPtr" | "isMinimal">
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
    <RootHeader v-if="isRoot" :self="self" :node-ptr="nodePtr" :focus-ptr="focusPtr" :graph="graph" />
    <div ref="containerRef" class="relative flex w-full flex-1 items-center justify-center p-[32px]">
      <!-- VNC view -->
      <div
        class="relative h-auto w-full overflow-hidden rounded-2xl border border-gray-200 bg-gray-100"
        :style="{
          aspectRatio: computer?.width && computer?.height ? `${computer.width} / ${computer.height}` : 'auto',
          maxHeight: '100%',
        }"
      >
        <!-- Live VNC view -->
        <Vnc
          v-if="computer?.status == ResourceStatus.AVAILABLE && computer?.vncUrl"
          ref="vncRef"
          class="h-full w-full"
          :url="computer?.vncUrl"
          auto-connect
          scale-viewport
        />
        <!-- TODO :Incomplete: proper Computer VNC/streaming view -->
        <div v-else>
          <!-- ... -->
        </div>

        <!-- Loading -->
        <Transition
          enter-from-class="opacity-0"
          enter-active-class="transition-opacity duration-200"
          enter-to-class="opacity-100"
          leave-from-class="opacity-100"
          leave-active-class="transition-opacity duration-200"
          leave-to-class="opacity-0"
          appear
          mode="out-in"
        >
          <div
            v-if="computer == null || computer?.status == ResourceStatus.PENDING || vncRef?.isLoading"
            class="absolute left-0 top-0 flex h-full w-full items-center justify-center"
          >
            <i class="fas fa-spinner-third animate-spin text-lg font-bold text-gray-400" />
          </div>
        </Transition>

        <Transition
          enter-from-class="opacity-0"
          enter-active-class="transition-opacity duration-200"
          enter-to-class="opacity-100"
          leave-from-class="opacity-100"
          leave-active-class="transition-opacity duration-200"
          leave-to-class="opacity-0"
          appear
          mode="out-in"
        >
          <div v-if="computer != null && !vncRef?.isConnected" class="absolute left-0 top-0 flex w-full justify-center">
            <div
              class="flex flex-row items-center rounded rounded-t-none border border-t-0 px-2 py-1"
              :style="{
                backgroundColor: getColorHex(ResourceStatusOptionInfo[computer.status]!.color!, ColorShade.S100),
                borderColor: getColorHex(ResourceStatusOptionInfo[computer.status]!.color!, ColorShade.S200),
              }"
            >
              <span
                class="w-5 text-center"
                :class="ResourceStatusOptionInfo[computer.status]!.icon!"
                :style="{ color: getColorHex(ResourceStatusOptionInfo[computer.status]!.color!) }"
              />
              <span class="ml-1 font-semibold">{{ toCamelName(ResourceStatus, computer.status) }}</span>
            </div>
          </div>
        </Transition>
      </div>
    </div>
  </div>
</template>
