<script lang="ts" setup>
import { ViewData, NodeType, Variant, StepData, ObjectType, StructType } from "@/proto/wire";
import { makeStruct, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import {
  declareActionMap,
  fireAction,
  getAction,
  type ActionBuiltinId,
  type ActionMapImplementation,
} from "@/ui/action";
import { IconInline } from "@/ui/icon";
import { addTransform } from "@/ui/view";
import { roundToStep } from "@/utils/functools";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import { computed, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = makeViewId(props);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self, { isRequired: false });
const selfView = spaceGraph.getRef(self);
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;
const block = pkgGraph.getRef(nodePtr, { ignoreAncestors: props.self == null });

//
// Interaction
//

const GRID_SCALE_X = 32;
const GRID_SCALE_Y = 24;
const DOT_SIZE = 2;
const SCALE_MIN = 0.5;
const SCALE_MAX = 1.5;
const SCALE_STEP = 0.1;

const transform = computed(() => selfView.value?.transform ?? makeStruct({ metatype: StructType.TRANSFORM }));
const scale = computed(() => transform.value.scaleX ?? transform.value.scaleY ?? 1);

function panCanvas(move: { x: number; y: number }) {
  if (props.self == null) return; // not a real view
  const self = spaceGraph.getOrError(props.self);
  spaceConnection.tx.update(
    self,
    { transform: addTransform(transform.value, { translateX: move.x, translateY: move.y }) },
    { debounce: "long" },
  );
}

function zoomCanvas(direction: "in" | "out") {
  if (props.self == null) return; // not a real view
  const currentZoom = roundToStep(transform.value?.scaleX ?? 1, SCALE_STEP);
  const newZoom =
    direction == "in" ? Math.min(SCALE_MAX, currentZoom + SCALE_STEP) : Math.max(SCALE_MIN, currentZoom - SCALE_STEP);
  const self = spaceGraph.getOrError(props.self);
  spaceConnection.tx.update(
    self,
    { transform: { ...transform.value, scaleX: newZoom, scaleY: newZoom } },
    { debounce: "long" },
  );
}

function resetCanvas() {
  if (props.self == null) return; // not a real view
  const self = spaceGraph.getOrError(props.self);
  spaceConnection.tx.update(self, { transform: makeStruct({ metatype: StructType.TRANSFORM }) }, { debounce: "short" });
}

const dragging: Ref<StepData | "canvas" | null> = ref(null);
function onDragging(e: MouseEvent) {
  if (dragging.value == null) return;
  if (props.self == null) return; // not a real view
  if (dragging.value == "canvas") {
    const self = spaceGraph.getOrError(props.self);
    const translateX = e.movementX / (transform.value?.scaleX ?? 1);
    const translateY = e.movementY / (transform.value?.scaleY ?? 1);
    spaceConnection.tx.update(
      self,
      { transform: addTransform(transform.value, { translateX, translateY }) },
      { debounce: "long" },
    );
  } else {
    throw new Error("nocheckin: move step");
  }
}

function onWheel(e: WheelEvent) {
  zoomCanvas(e.deltaY < 0 ? "in" : "out");
}

// actions
const actions: Partial<ActionMapImplementation<"common">> = {
  "common.navigate.left": {
    action: () => panCanvas({ x: -GRID_SCALE_X, y: 0 }),
  },
  "common.navigate.right": {
    action: () => panCanvas({ x: GRID_SCALE_X, y: 0 }),
  },
  "common.navigate.up": {
    action: () => panCanvas({ x: 0, y: -GRID_SCALE_Y }),
  },
  "common.navigate.down": {
    action: () => panCanvas({ x: 0, y: GRID_SCALE_Y }),
  },
  "common.navigate.zoomIn": {
    action: () => zoomCanvas("in"),
  },
  "common.navigate.zoomOut": {
    action: () => zoomCanvas("out"),
  },
  "common.navigate.reset": {
    action: () => resetCanvas(),
  },
};

const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);
canvas.registerView(self, id);
defineExpose<ViewExposed>({ self, id, actions });
</script>
<template>
  <div
    class="relative h-full w-full"
    :class="[variant == Variant.COMPACT ? 'rounded border border-gray-200' : '', dragging ? 'cursor-grabbing' : '']"
    @mousedown.stop="dragging = 'canvas'"
    @mousemove="(e: MouseEvent) => onDragging(e)"
    @mouseup.stop="dragging = null"
    @mouseleave.stop="dragging = null"
    @dragstart.stop.prevent="false"
    @wheel.prevent="onWheel"
  >
    <!-- nocheckin: flow UI -->
    <!-- Background grid -->
    <div class="absolute h-full w-full overflow-hidden" :style="{}">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-full w-full text-gray-200"
        :style="{
          // extra spacing for smooth infinite scrolling
          width: `${100 / scale}%`,
          height: `${100 / scale}%`,
          transformOrigin: '0 0',
          transform: `scale(${scale}, ${scale})  translate(${(transform?.translateX ?? 0) % GRID_SCALE_X}px, ${(transform?.translateY ?? 0) % GRID_SCALE_Y}px)`,
        }"
      >
        <defs>
          <pattern
            id="dot-pattern"
            x="0"
            y="0"
            :width="GRID_SCALE_X"
            :height="GRID_SCALE_Y"
            patternUnits="userSpaceOnUse"
          >
            <circle cx="2" cy="2" :r="DOT_SIZE" fill="currentColor" />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#dot-pattern)" />
      </svg>
    </div>

    <!-- Contents -->
    <div class="z-0 h-full w-full overflow-hidden">
      <div
        class="h-full w-full"
        :style="{
          transform: `scale(${scale}, ${scale}) translate(${transform?.translateX ?? 0}px, ${transform?.translateY ?? 0}px) `,
        }"
      >
        <!-- nocheckin: steps and pipes and stuff -->
      </div>
    </div>

    <!-- Overlay -->
    <div class="absolute left-0 top-0 h-full w-full transform">
      <!-- Controls -->
      <div
        class="absolute right-2 top-2 z-20 flex w-fit flex-row items-center gap-x-1 border border-gray-200 bg-white px-2 py-1"
      >
        <button
          v-for="action of (
            ['common.navigate.zoomIn', 'common.navigate.zoomOut', 'common.navigate.reset'] as ActionBuiltinId[]
          ).map(getAction)"
          :key="action.id"
          class="rounded px-0.5 hover:bg-gray-100 hover:text-primary-900"
          :class="isFocusAbsolute ? 'text-gray-700' : 'text-gray-400'"
          @click="fireAction(action)"
        >
          <IconInline v-bind="action.icon" class="w-5 text-center" />
        </button>
      </div>
    </div>
  </div>
</template>
