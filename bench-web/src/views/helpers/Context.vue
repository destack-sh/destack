<script lang="ts" setup>
import { NodeType, Orientation, ViewData } from "@/proto/wire";
import { toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import { canvas, containerPtr, inspectionPtr, pagePtr, threadPtr } from "@/system/space";
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

// node
const { node: inspection, connection: inspectionConnection } = supergraph.getLinkRef(inspectionPtr);
const { node: container, connection: containerConnection } = supergraph.getLinkRef(containerPtr);
const { node: page, connection: pageConnection } = supergraph.getLinkRef(pagePtr);
const target = computed(() => {
  return container.value ?? inspection.value;
});
const targetPtr = computed(() => (target.value != null ? toNodeRef(target.value) : undefined));
const scope = computed(() => container.value);

// interaction
const bodyRef = ref<HTMLElement | null>(null);
const threadRef = ref<InstanceType<typeof Thread> | null>(null);
const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const bodyHeight = computed(() => (props.size?.height ?? 0) - BAR_HEADER_HEIGHT);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayRef });

defineExpose<ViewExpose>({ self });
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

      <!-- Meta -->
      <div class="ml-auto flex flex-row items-center gap-x-1.5">
        <!-- Focus thread -->
        <button
          v-tooltip="{ title: 'Focus Thread', small: true, group: 'context.meta' }"
          class="cursor-pointer rounded-sm border-gray-200 px-1 py-0.5 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-900"
          @click="() => canvas.goToNode(threadPtr!)"
        >
          <span class="fas fa-arrow-up-left" />
        </button>
        <!-- New/reset thread -->
        <button
          v-tooltip="{ title: 'New Thread', small: true, group: 'context.meta' }"
          class="cursor-pointer rounded-sm border-gray-200 px-1 py-0.5 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-900"
          @click="
            () => {
              canvas.tx().update(canvas.space.value!, { threadPtr: undefined });
              nextTick(() => threadRef?.focus?.());
            }
          "
        >
          <span class="fas fa-rotate-left" />
        </button>
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
          id="thread"
          ref="threadRef"
          is-minimal
          :node-ptr="threadPtr"
          :size="{
            width: size?.width,
            height: bodyHeight,
          }"
        />
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </Scroll>
  </div>
</template>
