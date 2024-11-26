<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { useSubnodeProperty } from "@/language/node";
import { getRunBasePtr, isRunnable } from "@/language/session";
import { HelpAspect, NodeType, Orientation, RunData, ViewData, ViewType } from "@/proto/wire";
import { TypedNodeReferenceData } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import { CLEAR_RUN_ACTION, getRunActions, runtime } from "@/system/runtime";
import { canvas, inspectionPtr, pkgConnection } from "@/system/space";
import { IconInline, makeIcon } from "@/ui/icon";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import RunStatus from "@/views/builtins/RunStatus.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Inspect from "@/views/system/Inspect.vue";
import Start from "@/views/system/Start.vue";
import { computed, nextTick, ref, Ref, toRef, watchEffect } from "vue";

const BAR_HEADER_HEIGHT = VIEW_DEFAULT_BAR_HEADER_HEIGHT;
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const FOOTER_HEIGHT = 0;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "size" | "subnodePacked"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// node
const nodePtr = computedValue(() => props.nodePtr ?? inspectionPtr.value);
const { node } = supergraph.getLinkRef(nodePtr);
const isNodeRunnable = computed(() => node.value != null && isRunnable(node.value));

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
const runBasePtr = computed(() => (containingRun.value != null ? getRunBasePtr(containingRun.value) : nodePtr.value));
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
const visibleAspects = computed(() => {
  const visibleAspects: HelpAspect[] = [HelpAspect.INSPECT];
  if (isNodeRunnable.value) {
    visibleAspects.push(HelpAspect.RUN);
  }
  return visibleAspects;
});
watchEffect(() => {
  // ensure aspect is visible
  if (!visibleAspects.value.includes(aspect.value)) {
    setAspect(visibleAspects.value[0]);
  }
});

const startRef: Ref<InstanceType<typeof Start> | null> = ref(null);

defineExpose<ViewExposed>({ self });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Bench Header -->
    <div
      class="mx-2 my-1.5 flex flex-shrink-0 flex-row items-center gap-x-1 rounded pl-2.5 pr-2.5"
      :style="{
        height: `${BAR_HEADER_HEIGHT - 12}px`,
      }"
    >
      <!-- Node -->
      <NodeReference v-if="node" :node="node" :connection="pkgConnection" is-input />
      <span v-else class="text-gray-400">Nothing</span>
      <!-- Run status -->
      <div v-if="containingRun != null" class="flex px-1.5 flex-row">
        <template v-if="selfRun != null && selfRun.id != containingRun.id">
          <RunStatus :run="selfRun" />
          <span class="ml-2 mr-1 text-gray-400">/</span>
        </template>
        <RunStatus :run="containingRun" />
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
            <span v-if="action.isPrimary" class="mr-1">{{ action.title }}</span>
            <IconInline class="w-5 text-center" v-bind="action.icon" />
          </button>
        </div>
      </div>
    </div>

    <!-- Header -->
    <div
      class="mx-3 mb-3 mt-1.5 flex flex-row items-center gap-x-2"
      :style="{
        height: `${HEADER_HEIGHT - 12}px`,
      }"
    >
      <button
        v-for="a in visibleAspects"
        :key="a"
        class="flex flex-shrink-0 cursor-pointer flex-row items-center rounded px-2 py-1 transition-colors duration-75"
        :class="[
          a == aspect ? 'bg-gray-100 font-medium text-gray-900' : 'text-gray-400 hover:bg-gray-100 hover:text-gray-700',
        ]"
        @click="
          state.update({ metatype: NodeType.VIEW, type: ViewType.HELP, subnode: { aspect: a } }, { debounce: 'short' })
        "
      >
        <span>{{ toCamelName(HelpAspect, a) }} </span>
      </button>
    </div>

    <!-- Content -->
    <div v-if="aspect == HelpAspect.INSPECT">
      <Scroll
        id="scroll"
        ref="scrollRef"
        :orientation="Orientation.VERTICAL"
        size-is-dynamic
        :size="{ width: size?.width, height: (size?.height ?? 0) - BAR_HEADER_HEIGHT - HEADER_HEIGHT - FOOTER_HEIGHT }"
      >
        <Inspect id="inspect" :node-ptr="nodePtr" v-bind="state.getChildState('scroll.inspect', { nodePtr })" />
      </Scroll>
    </div>
    <div v-else-if="aspect == HelpAspect.RUN">
      <Start id="start" ref="startRef" :node-ptr="nodePtr" v-bind="state.getChildState('start', { nodePtr })" />
    </div>
    <div v-else class="mx-5">
      <span class="text-red-600">{{ toCamelName(HelpAspect, aspect) }}</span>
    </div>
  </div>
</template>
