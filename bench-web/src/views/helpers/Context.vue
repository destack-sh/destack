<script lang="ts" setup>
import { isInlineSourceNode, isSourceNode, isStateNode, toCamelName } from "@/language/core/const";
import { makeAndConditional, makeExpression } from "@/language/core/expression";
import { packSubnode, useSubnodeProperty } from "@/language/core/node";
import { isRunnable } from "@/language/runtime/run";
import {
  BlockType,
  ContextAspect,
  ExpressionType,
  IconData,
  NodeType,
  Orientation,
  PageData,
  PROPERTY_ENUM_BY_TYPE,
  RunData,
  RunProperty,
  RunType,
  ViewData,
  ViewType,
} from "@/proto/wire";
import { isNode, propertyReference, toNodeRef, TypedNodeReferenceData } from "@/proto/wiring";
import { CLEAR_RUN_ACTION, getRunActions, runtime } from "@/runtime/runtime";
import { supergraph } from "@/system/connection";
import { canvas, inspectionPtr } from "@/system/space";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { IconInline, makeIcon } from "@/ui/icon";
import { VIEW_DEFAULT_HEADER_HEIGHT, VIEW_DEFAULT_ROOT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RunStatus from "@/views/builtins/RunStatus.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import List from "@/views/collections/List.vue";
import { type ViewEmits, type ViewExpose } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Text from "@/views/content/Text.vue";
import Chat from "@/views/helpers/Chat.vue";
import Run from "@/views/nodes/Run.vue";
import SomeObject from "@/views/objects/Object.vue";
import { useElementSize } from "@vueuse/core";
import { computed, nextTick, ref, Ref, toRef } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_ROOT_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const FOOTER_HEIGHT = 0;

const ICON_BY_CONTEXT_ASPECT: Record<ContextAspect, IconData> = {
  [ContextAspect.UNSPECIFIED]: makeIcon("fas fa-question"),
  [ContextAspect.DETAIL]: makeIcon("fas fa-eye"),
  [ContextAspect.RUN]: makeIcon("fas fa-play"),
  [ContextAspect.CHAT]: makeIcon("fas fa-message"),
  [ContextAspect.LOG]: makeIcon("fas fa-file-lines"),
};

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
const target = computed(() => {
  if (delegate.value != null) {
    return delegate.value;
  } else if (isNode(inspection.value, NodeType.BLOCK) && inspection.value.type >= BlockType.PARAGRAPH) {
    return parent.value;
  } else {
    return inspection.value;
  }
});
const targetPtr = computed(() => (target.value != null ? toNodeRef(target.value) : undefined));
const targetType = computed(() => targetPtr.value?.nodeType);
const targetPropertiesEnum = computed(() =>
  targetType.value != null ? (PROPERTY_ENUM_BY_TYPE[targetType.value] ?? {}) : {},
);
const scope = computed(() => {
  if (delegate.value != null) {
    return delegate.value;
  } else if (
    isStateNode(inspection.value) ||
    (isSourceNode(inspection.value) && !isInlineSourceNode(inspection.value))
  ) {
    return parent.value;
  } else {
    return inspection.value;
  }
});
const threadPtr = computed(() => {
  if (isInlineSourceNode(scope.value) && scope.value.threadPtr != null) {
    // current thread for scope (if in same channel)
    return scope.value.threadPtr;
  } else if (isNode(scope.value, NodeType.CHANNEL) || isNode(scope.value, NodeType.THREAD)) {
    // don't show the same thread twice
    return undefined;
  } else if (isNode(scope.value, NodeType.MESSAGE) && scope.value.createdThreadPtr != null) {
    // current thread for message
    return scope.value.createdThreadPtr;
  } else {
    return scope.value != null ? toNodeRef(scope.value) : undefined;
  }
});
const thread = supergraph.getRef(threadPtr);

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
    setAspect(ContextAspect.RUN);
    nextTick(() => startRef.value?.start());
  }
}

// view :DefaultViewAspect
const aspect = useSubnodeProperty(NodeType.VIEW, ViewType.CONTEXT, toRef(props, "subnodePacked"), "aspect");
function setAspect(aspect: ContextAspect) {
  state.update({ metatype: NodeType.VIEW, type: ViewType.CONTEXT, subnode: { aspect } });
}
const visibleAspects = [ContextAspect.DETAIL, ContextAspect.CHAT, ContextAspect.RUN];
function selectAspect(aspect: ContextAspect) {
  state.update({ metatype: NodeType.VIEW, type: ViewType.CONTEXT, subnode: { aspect } });
  if (aspect == ContextAspect.CHAT) {
    nextTick(() => chatRef.value?.focus?.());
  }
}

// interaction
const bodyRef = ref<HTMLElement | null>(null);
const scrollRef: Ref<InstanceType<typeof Scroll> | null> = ref(null);
const startRef: Ref<InstanceType<typeof Run> | null> = ref(null);
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
      class="mx-2 flex flex-shrink-0 flex-row items-center gap-x-1 rounded py-1.5 pl-2.5 pr-2.5"
      :style="{
        height: `${BAR_HEADER_HEIGHT}px`,
      }"
    >
      <!-- Node (path) -->
      <NodeReference v-if="target" :node="target" :tx="() => inspectionConnection!.tx" size="regular" is-input />
      <span v-else class="text-gray-400">Nothing</span>
      <template v-if="scope != null && scope.id != target?.id">
        <span class="mx-0.5 text-gray-400">in</span>
        <NodeReference :node="scope" :tx="() => inspectionConnection!.tx" is-light size="regular" />
      </template>

      <!-- Tabs -->
      <div class="ml-auto flex flex-row items-center">
        <button
          v-for="a in visibleAspects"
          :key="a"
          v-tooltip="{ title: toCamelName(ContextAspect, a), small: true, group: 'context.tabs' }"
          class="flex flex-shrink-0 cursor-pointer flex-row items-center rounded px-2 py-1 transition-colors duration-75"
          :class="[
            a == aspect
              ? 'bg-gray-100 font-medium text-gray-700'
              : 'text-gray-400 hover:bg-gray-100 hover:text-gray-700',
          ]"
          @click="selectAspect(a)"
        >
          <IconInline v-bind="ICON_BY_CONTEXT_ASPECT[a]" />
        </button>
      </div>
    </div>

    <!-- Header -->
    <div ref="headerRef" class="mx-3 flex flex-row items-center gap-x-2" :style="{}">
      <!-- NOTE :UX: the Context header thing is ugly -->
      <template v-if="aspect == ContextAspect.DETAIL">
        <!-- Detail text -->
        <Text
          v-if="target != null && 'text' in targetPropertiesEnum"
          id="text"
          is-small
          is-input
          placeholder="Text"
          class="mx-1 my-1 w-full"
          :model-value="(target as any).text"
          @update:model-value="
            inspectionConnection?.tx.update(target as PageData, { text: $event }, { debounce: 'long' })
          "
        />
        <span v-else class="mx-1 text-gray-400">No text available.</span>
      </template>
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
          v-if="aspect == ContextAspect.DETAIL"
          id="detail"
          :node-ptr="targetPtr"
          is-input
          v-bind="state.getChildState('scroll.detail', { nodePtr: targetPtr, isInput: true, isMinimal: false })"
          data-contextmenu="ignore"
        />
        <!-- Chat -->
        <Chat
          v-else-if="aspect == ContextAspect.CHAT"
          id="chat"
          ref="chatRef"
          :key="threadPtr?.id"
          :node-ptr="threadPtr"
          v-bind="state.getChildState('scroll.chat', { nodePtr: threadPtr })"
          :size="{ width: size?.width, height: bodyHeight }"
          data-contextmenu="ignore"
        />
        <!-- Run -->
        <div v-else-if="aspect == ContextAspect.RUN">
          <Run
            id="run.run"
            ref="startRef"
            :node-ptr="targetPtr"
            v-bind="state.getChildState('scroll.start', { nodePtr: targetPtr })"
            data-contextmenu="ignore"
          />
          <!-- Prior -->
          <div class="mx-4 mt-2 flex flex-row items-center gap-x-2" :style="{ height: `${HEADER_HEIGHT}px` }">
            <span class="font-medium">Prior Runs</span>
          </div>
          <List
            id="run.recent"
            :subnode-packed="
              packSubnode(NodeType.VIEW, ViewType.LIST, {
                queryNodeType: NodeType.RUN,
                filter: makeAndConditional([
                  makeExpression({
                    type: ExpressionType.EQUALS,
                    propertyPtr: propertyReference(NodeType.RUN, RunProperty.type),
                    value: RunType.FLOW,
                  }),
                  makeExpression({
                    type: ExpressionType.EQUALS,
                    propertyPtr: propertyReference(NodeType.RUN, RunProperty.flowPtr),
                    value: scope != null ? toNodeRef(scope) : undefined,
                  }),
                ]),
                sort: [
                  makeExpression({
                    type: ExpressionType.DESCENDING,
                    propertyPtr: propertyReference(NodeType.RUN, RunProperty.createdAt),
                  }),
                ],
              })
            "
          />
        </div>
      </div>

      <!-- Selection overlay -->
      <SelectionOverlay ref="selectionOverlayRef" :zone="selectionZone" />
    </Scroll>
  </div>
</template>
