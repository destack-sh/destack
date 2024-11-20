<script lang="ts" setup>
import { ViewData, NodeType, ViewType, HelpAspect } from "@/proto/wire";
import { viewEmits, type ViewExposed } from "@/views/common";
import { canvas, inspectionPtr, pkgConnection, pkgGraph } from "@/system/space";
import { computed, toRef } from "vue";
import { TypedNodeReferenceData, unwrapProtoOneOf } from "@/proto/wiring";
import { useExistingConnection } from "@/system/connection";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { useSubnodeProperty } from "@/language/node";
import { computedValue } from "@/utils/ref";
import NodeReference from "@/views/builtins/NodeReference.vue";
import { toCamelName } from "@/language/const";
import Inspect from "@/views/system/Inspect.vue";
import { isRunnable } from "@/language/session";
import Start from "@/views/system/Start.vue";

const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

const props = defineProps<
  { self: TypedNodeReferenceData<NodeType.VIEW>; id: string } & Pick<
    ViewData,
    "name" | "title" | "icon" | "nodePtr" | "subnodePacked"
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self);
const children = spaceGraph.getChildrenRef(self, NodeType.VIEW, { ignoreAncestors: true });
const nodePtr = computedValue(() => unwrapProtoOneOf(props.nodePtr) ?? inspectionPtr.value);
const node = pkgGraph.getRef(nodePtr);
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

defineExpose<ViewExposed>({ self });
</script>
<template>
  <div class="flex h-full w-full flex-col">
    <!-- Bench Header -->
    <div
      class="mx-2 my-1.5 flex cursor-pointer flex-row items-center rounded pl-2.5 pr-1"
      :style="{
        height: `${VIEW_DEFAULT_BAR_HEADER_HEIGHT - 12}px`,
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
    <!-- nocheckin: Help content -->
    <div v-if="aspect == HelpAspect.INSPECT">
      <Inspect id="inspect" :node-ptr="props.nodePtr" />
    </div>
    <div v-else-if="aspect == HelpAspect.RUN">
      <Start id="start" :node-ptr="props.nodePtr" />
    </div>
    <div v-else class="mx-5">
      <span class="text-red-600">{{ toCamelName(HelpAspect, aspect) }}</span>
    </div>
  </div>
</template>
