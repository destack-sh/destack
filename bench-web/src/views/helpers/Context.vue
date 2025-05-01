<script lang="ts" setup>
import { BENCH_BENCH_AGENT_PTR } from "@/language/core/builtin";
import { createDefaultThread, createThread } from "@/language/source/thread";
import { ContextMode, NodeType, Orientation, ViewData } from "@/proto/wire";
import { toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import {
  benchConnection,
  benchGraph,
  canvas,
  containerPtr,
  inspectionPtr,
  pagePtr,
  pkg,
  threadPtr,
} from "@/system/space";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Thread from "@/views/nodes/Thread.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { computed, nextTick, ref, Ref, toRef } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_ROOT_HEADER_HEIGHT;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "size"
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
canvas.registerView(self, id);

// node
const { node: inspection, connection: inspectionConnection } = supergraph.getLinkRef(inspectionPtr);
const { node: container, connection: containerConnection } = supergraph.getLinkRef(containerPtr);
const { node: page, connection: pageConnection } = supergraph.getLinkRef(pagePtr);
const target = computed(() => {
  return container.value ?? inspection.value;
});
const targetPtr = computed(() => (target.value != null ? toNodeRef(target.value) : undefined));
const scope = computed(() => container.value);
const { node: thread, connection: threadConnection } = supergraph.getLinkRef(threadPtr, { excludeSearch: true });

// state
const mode: Ref<ContextMode> = ref(ContextMode.CHAT);

// interaction
const bodyRef = ref<HTMLElement | null>(null);
const threadRef = ref<InstanceType<typeof Thread> | null>(null);
const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const bodyHeight = computed(() => (props.size?.height ?? 0) - BAR_HEADER_HEIGHT);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayRef });

function focus() {
  if (threadRef.value != null) {
    threadRef.value.focus?.();
  }
}

defineExpose<ViewExpose>({ self, focus });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Bench Header -->
    <div
      class="mx-2 flex shrink-0 flex-row items-center gap-x-1 rounded-sm py-1.5 pr-2.5 pl-2.5"
      :style="{
        height: `${BAR_HEADER_HEIGHT}px`,
      }"
    >
      <!-- Node (path) -->
      <NodeReference
        v-if="target"
        :node="target"
        :tx="() => inspectionConnection!.tx"
        size="sm"
        class="max-w-[300px] truncate"
        is-input
      />
      <span v-else class="text-gray-400">Nothing</span>
      <template v-if="scope != null && scope.id != target?.id">
        <span class="mx-0.5 text-gray-400">in</span>
        <NodeReference :node="scope" :tx="() => inspectionConnection!.tx" is-light size="sm" />
      </template>

      <!-- Thread -->
      <div class="ml-auto flex flex-row items-center gap-x-1.5">
        <!-- New/reset thread -->
        <button
          v-tooltip="{ title: 'New Thread', small: true, group: 'context.meta' }"
          class="cursor-pointer rounded-sm border-gray-200 px-1 py-0.5 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-900"
          @click="
            () => {
              const thread = createDefaultThread(benchConnection.tx, benchGraph);
              canvas.tx().update(canvas.space.value!, { threadPtr: toNodeRef(thread) });
              nextTick(() => threadRef?.focus?.());
            }
          "
        >
          <span class="fas fa-rotate-left" />
        </button>
        <!-- Thread -->
        <NodeReference
          v-if="thread != null && thread.id != target?.id"
          :node="thread"
          :tx="() => inspectionConnection!.tx"
          size="sm"
          class="max-w-[300px] truncate rounded-sm transition-colors duration-150 hover:cursor-pointer hover:bg-gray-100"
          @click="() => canvas.goToNode(threadPtr!)"
        />
      </div>
    </div>

    <!-- Content -->
    <Scroll
      id="scroll"
      ref="scrollRef"
      :orientation="Orientation.VERTICAL"
      size-is-dynamic
      :size="{ width: size?.width, height: bodyHeight }"
      @mousedown="(e: MouseEvent) => startSelectingIfAllowed(selectionZone, e)"
    >
      <div
        ref="bodyRef"
        class="w-full"
        :style="{
          minHeight: `${bodyHeight - 10 /* mystery offset? */}px`,
        }"
      >
        <!-- Detail -->
        <Thread
          v-if="mode == ContextMode.CHAT && targetPtr?.id != threadPtr?.id"
          id="thread"
          ref="threadRef"
          is-minimal
          :node-ptr="threadPtr"
          :size="{
            width: size?.width ?? 0,
            height: bodyHeight,
          }"
        />
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </Scroll>
  </div>
</template>
