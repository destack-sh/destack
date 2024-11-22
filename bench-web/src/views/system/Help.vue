<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { useSubnodeProperty } from "@/language/node";
import { isRunnable } from "@/language/session";
import { HelpAspect, NodeType, Orientation, ViewData, ViewType } from "@/proto/wire";
import { toNodeRefOneOf, TypedNodeReferenceData, unwrapProtoOneOf, wrapProtoOneOf } from "@/proto/wiring";
import { supergraph } from "@/system/connection";
import { canvas, inspectionPtr, pkgConnection } from "@/system/space";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { viewEmits, type ViewExposed } from "@/views/common";
import Scroll from "@/views/containers/Scroll.vue";
import Inspect from "@/views/system/Inspect.vue";
import Start from "@/views/system/Start.vue";
import { computed, Ref, toRef, watchEffect } from "vue";

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

const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr) ?? inspectionPtr.value);
const nodePtrOneOf: Ref<ViewData["nodePtr"]> = computedValue(() =>
  nodePtr.value != null ? toNodeRefOneOf(nodePtr.value) : { oneofKind: undefined },
);
const { node } = supergraph.getLinkRef(nodePtr);
const aspect = useSubnodeProperty(NodeType.VIEW, ViewType.HELP, toRef(props, "subnodePacked"), "aspect");
const visibleAspects = computed(() => {
  const visibleAspects: HelpAspect[] = [HelpAspect.INSPECT];
  if (node.value != null) {
    if (isRunnable(node.value)) {
      visibleAspects.push(HelpAspect.RUN);
    }
  }
  return visibleAspects;
});
watchEffect(() => {
  // ensure aspect is visible
  if (!visibleAspects.value.includes(aspect.value)) {
    state.update({ metatype: NodeType.VIEW, type: ViewType.HELP, subnode: { aspect: visibleAspects.value[0] } });
  }
});

defineExpose<ViewExposed>({ self });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Bench Header -->
    <div
      class="mx-2 my-1.5 flex flex-shrink-0 cursor-pointer flex-row items-center rounded pl-2.5 pr-1"
      :style="{
        height: `${BAR_HEADER_HEIGHT - 12}px`,
      }"
    >
      <!-- Node -->
      <NodeReference v-if="node" :node="node" :connection="pkgConnection" is-input />
      <span v-else class="text-gray-400">Nothing</span>
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
        <Inspect
          id="inspect"
          :node-ptr="nodePtrOneOf"
          v-bind="state.getChildState('scroll.inspect', { nodePtr: nodePtrOneOf })"
        />
      </Scroll>
    </div>
    <div v-else-if="aspect == HelpAspect.RUN">
      <Start id="start" :node-ptr="nodePtrOneOf" v-bind="state.getChildState('start', { nodePtr: nodePtrOneOf })" />
    </div>
    <div v-else class="mx-5">
      <span class="text-red-600">{{ toCamelName(HelpAspect, aspect) }}</span>
    </div>
  </div>
</template>
