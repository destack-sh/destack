<script lang="ts" setup>
import { toCamelName } from "@/language/core/const";
import { getEnumOptions } from "@/language/core/enum";
import { useSubnodeProperty } from "@/language/core/node";
import { EnumType, ContextMode, NodeType, Orientation, ViewData, ViewType } from "@/proto/wire";
import { toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import { canvas, containerPtr, inspectionPtr, pagePtr, threadPtr } from "@/system/space";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { getNodeIcon, IconInline } from "@/ui/icon";
import { VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Chat from "@/views/helpers/Chat.vue";
import Thread from "@/views/nodes/Thread.vue";
import SomeObject from "@/views/objects/Object.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { useElementSize } from "@vueuse/core";
import { computed, ref, Ref, toRef } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_ROOT_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const FOOTER_HEIGHT = 0;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "size" | "subnodePacked"
  >
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);
const subnodePacked = toRef(props, "subnodePacked");

// node
const { node: inspection, connection: inspectionConnection } = supergraph.getLinkRef(inspectionPtr);
const { node: container, connection: containerConnection } = supergraph.getLinkRef(containerPtr);
const { node: page, connection: pageConnection } = supergraph.getLinkRef(pagePtr);
const target = computed(() => {
  return container.value ?? inspection.value;
});
const targetPtr = computed(() => (target.value != null ? toNodeRef(target.value) : undefined));
const scope = computed(() => container.value);

// state (should be in ContextView?)
const mode = useSubnodeProperty(NodeType.VIEW, ViewType.CONTEXT, subnodePacked, "contextMode");

// interaction
const bodyRef = ref<HTMLElement | null>(null);
const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const headerRef: Ref<HTMLDivElement | null> = ref(null);
const headerSize = useElementSize(headerRef);
const bodyHeight = computed(
  () => (props.size?.height ?? 0) - BAR_HEADER_HEIGHT - headerSize.height.value - FOOTER_HEIGHT,
);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayRef });
const chatRef: Ref<InstanceType<typeof Chat> | null> = ref(null);

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
      <div class="ml-auto flex flex-row items-center">
        <!-- ... -->
        <div v-if="target != null" class="flex flex-row items-center text-gray-400">
          <IconInline v-bind="getNodeIcon({ metatype: target.metatype })" class="w-5 text-center" />
          <span class="ml-1">{{ toCamelName(NodeType, target.metatype) }}</span>
        </div>
      </div>
    </div>

    <!-- Header -->
    <div ref="headerRef" class="mx-3 flex flex-row items-center gap-x-1 pt-1 pb-2" :style="{}">
      <!-- ... -->
      <button
        v-for="m in getEnumOptions(EnumType.CONTEXT_MODE)"
        :key="m.value"
        class="cursor-pointer rounded-sm px-1.5 py-0.5 transition-colors duration-75 hover:bg-gray-100"
        :class="[mode == m.value ? 'bg-gray-100 text-gray-900' : 'text-gray-400']"
        @click="
          state.update({
            metatype: NodeType.VIEW,
            type: ViewType.CONTEXT,
            subnode: { contextMode: m.value },
          })
        "
      >
        <span>{{ m.title }}</span>
      </button>
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
        :style="{
          minHeight: `${bodyHeight - 10 /* not entirely sure why, the Scroll component seems to have some padding/border? */}px`,
        }"
      >
        <!-- Detail -->
        <SomeObject
          v-if="mode == ContextMode.DETAIL"
          id="detail"
          :node-ptr="targetPtr"
          is-input
          v-bind="state.getChildState('scroll.detail', { nodePtr: targetPtr, isInput: true, isMinimal: false })"
          data-contextmenu="ignore"
        />
        <!-- Chat -->
        <Thread
          v-else-if="mode == ContextMode.CHAT"
          id="chat"
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
