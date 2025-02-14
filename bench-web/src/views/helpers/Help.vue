<script lang="ts" setup>
import { getBaseFromNode, toCamelName } from "@/language/core/const";
import { useSubnodeProperty } from "@/language/core/node";
import { isRunnable } from "@/language/runtime/run";
import { BlockType, HelpAspect, NodeType, Orientation, RunData, ViewData, ViewType } from "@/proto/wire";
import { isNode, TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import { CLEAR_RUN_ACTION, getRunActions, runtime } from "@/runtime/runtime";
import { canvas, inspectionPtr } from "@/system/space";
import { IconInline, makeIcon } from "@/ui/icon";
import { VIEW_DEFAULT_ROOT_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RunStatus from "@/views/builtins/RunStatus.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import SomeObject from "@/views/objects/Object.vue";
import Run from "@/views/nodes/Run.vue";
import { computed, nextTick, ref, Ref, toRef } from "vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import Chat from "@/views/helpers/Chat.vue";

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

// node
const nodePtr = computedValue(() => props.nodePtr ?? inspectionPtr.value);
const { node: inspection, connection: inspectionConnection } = supergraph.getLinkRef(nodePtr);
const isNodeRunnable = computed(() => inspection.value != null && isRunnable(inspection.value));
const parentPtr = computed(() => inspection.value?.parentPtr);
const { node: parent, connection: parentConnection } = supergraph.getLinkRef(parentPtr);
const delegatePtr = computed(() => {
  if (isNode(inspection.value, NodeType.BLOCK)) {
    return inspection.value.nodePtr;
  } else {
    return null;
  }
});
const { node: delegate, connection: delegateConnection } = supergraph.getLinkRef(delegatePtr);
const scope = computed(() => {
  if (delegate.value != null) {
    return delegate.value;
  } else if (isNode(inspection.value, NodeType.BLOCK) && inspection.value.type >= BlockType.PARAGRAPH) {
    return parent.value;
  } else {
    return inspection.value;
  }
});

// run
const containingRun: Ref<RunData | null> = computed(() => {
  if (nodePtr.value != null && runtime.focusedRun != null && runtime.focusedRunTree.hasBase(nodePtr.value)) {
    return runtime.focusedRun;
  } else {
    return null;
  }
});
const selfRun: Ref<RunData | null> = computed(() => {
  if (nodePtr.value != null && runtime.focusedRun != null && runtime.focusedRunTree.hasBase(nodePtr.value)) {
    return runtime.focusedRunTree.getLastActiveRun({ ck: nodePtr.value.ck });
  } else {
    return null;
  }
});
function start() {
  if (startRef.value != null) {
    startRef.value.start();
  } else {
    setAspect(HelpAspect.RUN);
    nextTick(() => startRef.value?.start());
  }
}

// view
const aspect = useSubnodeProperty(NodeType.VIEW, ViewType.HELP, toRef(props, "subnodePacked"), "aspect");
function setAspect(aspect: HelpAspect) {
  state.update({ metatype: NodeType.VIEW, type: ViewType.HELP, subnode: { aspect } });
}
const visibleAspects = [HelpAspect.DETAIL, HelpAspect.RUN, HelpAspect.CHAT];
function selectAspect(aspect: HelpAspect) {
  state.update({ metatype: NodeType.VIEW, type: ViewType.HELP, subnode: { aspect } });
  if (aspect == HelpAspect.CHAT) {
    nextTick(() => chatRef.value?.focus?.());
  }
}

// interaction
const bodyRef = ref<HTMLElement | null>(null);
const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const startRef: Ref<InstanceType<typeof Run> | null> = ref(null);
const bodyHeight = computed(() => (props.size?.height ?? 0) - BAR_HEADER_HEIGHT - HEADER_HEIGHT - FOOTER_HEIGHT);
const selectionOverlayRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZone = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayRef });
const chatRef: Ref<InstanceType<typeof Chat> | null> = ref(null);

defineExpose<ViewExpose>({ self });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Bench Header -->
    <div
      class="mx-2 flex flex-shrink-0 flex-row items-center gap-x-1 rounded py-1.5 pl-2.5 pr-2.5"
      :style="{
        height: `${BAR_HEADER_HEIGHT}px`,
      }"
    >
      <!-- Node (path) -->
      <NodeReference v-if="scope" :node="scope" :tx="() => inspectionConnection!.tx" size="regular" is-input />
      <span v-else class="text-gray-400">Nothing</span>
      <!-- Run status -->
      <div v-if="containingRun != null" class="flex flex-row px-1.5">
        <template v-if="selfRun != null && selfRun.id != containingRun.id">
          <RunStatus :run="selfRun" icon="dot" />
          <span class="ml-2 mr-2 text-gray-400">/</span>
        </template>
        <RunStatus :run="containingRun" icon="dot" />
      </div>

      <!-- Meta/Controls -->
      <div class="ml-auto flex flex-row items-center">
        <!-- Controls -->
        <div
          class="flex flex-row items-center gap-x-0.5"
          :style="{
            height: `${HEADER_HEIGHT}px`,
          }"
        >
          <!-- Run controls -->
          <button
            v-for="action in containingRun != null
              ? [...getRunActions(containingRun), CLEAR_RUN_ACTION]
              : [{ title: 'Start', isPrimary: true, icon: makeIcon('fas fa-play'), action: () => start() }]"
            v-if="isNodeRunnable"
            :key="action.title"
            v-tooltip="{ title: action.title, small: true, group: 'run' }"
            class="rounded px-0.5 py-0.5 text-gray-700 transition-colors duration-75 hover:bg-gray-100 hover:text-gray-900"
            @click.stop="action.action()"
          >
            <IconInline class="w-5 text-center" v-bind="action.icon" />
            <span v-if="action.isPrimary" class="ml-1">{{ action.title }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Header -->
    <div
      class="mx-3 flex flex-row items-center gap-x-2 pb-3 pt-1.5"
      :style="{
        height: `${HEADER_HEIGHT}px`,
      }"
    >
      <button
        v-for="a in visibleAspects"
        :key="a"
        class="flex flex-shrink-0 cursor-pointer flex-row items-center rounded px-2 py-1 transition-colors duration-75"
        :class="[
          a == aspect ? 'bg-gray-100 font-medium text-gray-900' : 'text-gray-400 hover:bg-gray-100 hover:text-gray-700',
        ]"
        @click="selectAspect(a)"
      >
        <span>{{ toCamelName(HelpAspect, a) }} </span>
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
          v-if="aspect == HelpAspect.DETAIL"
          id="detail"
          :node-ptr="delegatePtr ?? nodePtr"
          is-input
          v-bind="state.getChildState('scroll.detail', { nodePtr, isInput: true, isMinimal: false })"
          data-contextmenu="ignore"
        />
        <!-- Run -->
        <Run
          v-else-if="aspect == HelpAspect.RUN"
          id="start"
          ref="startRef"
          :node-ptr="delegatePtr ?? nodePtr"
          v-bind="state.getChildState('scroll.start', { nodePtr })"
          data-contextmenu="ignore"
        />
        <!-- Chat -->
        <Chat
          v-else-if="aspect == HelpAspect.CHAT"
          id="chat"
          ref="chatRef"
          :node-ptr="delegatePtr ?? nodePtr"
          v-bind="state.getChildState('scroll.chat', { nodePtr })"
          :size="{ width: size?.width, height: bodyHeight }"
          data-contextmenu="ignore"
        />
        <!-- ... -->
        <div v-else class="mx-5">
          <span class="text-red-600">{{ toCamelName(HelpAspect, aspect) }}</span>
        </div>
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </Scroll>
  </div>
</template>
