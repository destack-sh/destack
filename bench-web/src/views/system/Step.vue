<script lang="ts" setup>
import { ViewData, NodeType } from "@/proto/wire";
import { unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { canvas } from "@/system/space";
import { computed, ref, toRef, type Ref } from "vue";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { DEFAULT_HEADER_HEIGHT } from "@/ui/canvas";
import { useElementSize } from "@vueuse/core";
import { FLOW_GRID_STEP_Y } from "@/language/flow";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "transform" | "variant" | "size">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self, { isRequired: false });
const selfView = spaceGraph.getRef(self);
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.STEP>);
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;
const step = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

const containerRef: Ref<HTMLElement | null> = ref(null);
const contentRef: Ref<HTMLElement | null> = ref(null);
const headerRef: Ref<HTMLElement | null> = ref(null);
const contentSize = useElementSize(contentRef);
const paddingHeight = computed(() => FLOW_GRID_STEP_Y - ((contentSize.height.value + DEFAULT_HEADER_HEIGHT) % FLOW_GRID_STEP_Y));

canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id });
</script>
<template>
  <div ref="containerRef" class="rounded border border-gray-200 bg-white">
    <!-- nocheckin: Step view -->
    <!-- Header -->
    <div
      ref="headerRef"
      class="flex w-full flex-row items-center border-b border-gray-200 px-2"
      :style="{ height: DEFAULT_HEADER_HEIGHT + 'px' }"
    >
      <!-- Icon/Name -->
      {{ step?.name ?? "???" }}
      <!-- Controls -->
      <!-- ... -->
    </div>
    <!-- Content -->
    <div ref="contentRef" class="p-2 h-28">
      <!-- ... -->
      step content {{ step?.type }}
    </div>
    <!-- Padding (to ensure height is a multiple of the grid) -->
    <div class="" :style="{ height: paddingHeight + 'px' }"></div>
  </div>
</template>
