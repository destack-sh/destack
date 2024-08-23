<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { createStep, FLOW_GRID_STEP_X, FLOW_GRID_STEP_Y } from "@/language/flow";
import { NodeType, StepData, StepType, StructType, Variant, ViewData } from "@/proto/wire";
import { makeStruct, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { fireAction, getAction, type ActionBuiltinId, type ActionMapImplementation } from "@/ui/action";
import { ICON_BY_STEP_TYPE, IconInline } from "@/ui/icon";
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
const steps = pkgGraph.getChildrenRef(block, NodeType.STEP);
const pipes = pkgGraph.getChildrenRef(block, NodeType.PIPE);

//
// Interaction
//

const DOT_SIZE = 4;
const SCALE_MIN = 0.5;
const SCALE_MAX = 2.0;
const SCALE_STEP = 0.1;

const transform = computed(
  () =>
    selfView.value?.transform ??
    makeStruct({ metatype: StructType.TRANSFORM, translateX: FLOW_GRID_STEP_X / 2, translateY: FLOW_GRID_STEP_Y / 2 }),
);
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
    action: () => panCanvas({ x: -FLOW_GRID_STEP_X, y: 0 }),
  },
  "common.navigate.right": {
    action: () => panCanvas({ x: FLOW_GRID_STEP_X, y: 0 }),
  },
  "common.navigate.up": {
    action: () => panCanvas({ x: 0, y: -FLOW_GRID_STEP_Y }),
  },
  "common.navigate.down": {
    action: () => panCanvas({ x: 0, y: FLOW_GRID_STEP_Y }),
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
    class="group/flow relative h-full w-full"
    :class="[variant == Variant.COMPACT ? 'rounded border border-gray-200' : '', dragging ? 'cursor-grabbing' : '']"
    @mousedown="dragging = 'canvas'"
    @mousemove="(e: MouseEvent) => onDragging(e)"
    @mouseup.stop="dragging = null"
    @mouseleave.stop="dragging = null"
    @dragstart.stop.prevent="false"
    @wheel.prevent="onWheel"
  >
    <!-- nocheckin: flow UI -->
    <!-- Background grid (infinitely repeated) -->
    <div class="absolute h-full w-full overflow-hidden" :style="{}">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-full w-full transition-colors duration-75"
        :class="[
          isFocusAbsolute || variant != Variant.COMPACT
            ? 'text-gray-200'
            : 'text-gray-100 group-hover/flow:text-gray-200',
        ]"
        :style="{
          // extra spacing for smooth infinite scrolling
          width: `${100 / scale}%`,
          height: `${100 / scale}%`,
          transformOrigin: '0 0',
          transform: `scale(${scale}, ${scale}) translate(${((transform?.translateX ?? 0) % FLOW_GRID_STEP_X) - DOT_SIZE / 2}px, ${((transform?.translateY ?? 0) % FLOW_GRID_STEP_Y) - DOT_SIZE / 2}px)`,
        }"
      >
        <defs>
          <pattern
            id="dot-pattern"
            :x="0"
            :y="0"
            :width="FLOW_GRID_STEP_X"
            :height="FLOW_GRID_STEP_Y"
            patternUnits="userSpaceOnUse"
          >
            <circle
              :cx="DOT_SIZE / 2"
              :cy="DOT_SIZE / 2"
              :r="DOT_SIZE / 2"
              fill="currentColor"
              class="hover:text-primary-900"
            />
          </pattern>
        </defs>
        <rect width="100%" height="100%" fill="url(#dot-pattern)" />
      </svg>
    </div>

    <!-- Contents -->
    <div class="z-0 h-full w-full overflow-hidden">
      <div
        class="relative"
        :style="{
          transformOrigin: '0 0',
          transform: `scale(${scale}, ${scale}) translate(${transform?.translateX ?? 0}px, ${transform?.translateY ?? 0}px) `,
        }"
      >
        <!-- nocheckin: steps and pipes and stuff -->
        steps:{{ steps.length }} pipes:{{ pipes.length }}

        <!-- <div
          class="absolute bg-red-500 font-bold text-white opacity-50"
          :style="{
            left: `${GRID_SCALE_X * 0}px`,
            top: `${GRID_SCALE_Y * 0}px`,
            width: `${GRID_SCALE_X * 4}px`,
            height: `${GRID_SCALE_Y * 4}px`,
          }"
        /> -->
      </div>
    </div>

    <!-- Overlay -->
    <div class="absolute left-0 top-0 h-full w-full transform">
      <!-- Menu -->
      <div
        class="absolute right-2 top-2 z-20 flex w-fit flex-row items-center divide-x divide-gray-200 border border-gray-200 bg-white px-2 py-1"
      >
        <!-- Create -->
        <div v-if="variant != Variant.COMPACT" class="flex flex-row items-center gap-x-1 pr-1.5">
          <button
            v-for="stepType in [StepType.START, StepType.COMPLETE, StepType.CODE, StepType.TEXT, StepType.BLOCK]"
            :key="stepType"
            v-tooltip="{
              title: `${toCamelName(StepType, stepType)}`,
              showDelay: 200,
              hideDelay: 100,
              small: true,
              referenceMargin: 8,
              group: 'page.footer',
            }"
            class="rounded px-0.5 hover:bg-gray-100 hover:text-primary-900"
            :class="isFocusAbsolute ? 'text-gray-700' : 'text-gray-400'"
            @click="block && createStep(pkgConnection.tx, pkgGraph, { step: { type: stepType }, parent: block })"
          >
            <IconInline v-bind="ICON_BY_STEP_TYPE[stepType]" class="w-5 text-center" />
          </button>
        </div>

        <!-- Actions -->
        <div class="flex flex-row items-center gap-x-1 pl-1.5">
          <button
            v-for="action of (
              ['common.navigate.zoomIn', 'common.navigate.zoomOut', 'common.navigate.reset'] as ActionBuiltinId[]
            ).map(getAction)"
            :key="action.id"
            v-tooltip="{
              title: action.title,
              showDelay: 200,
              hideDelay: 100,
              small: true,
              referenceMargin: 8,
              group: 'page.footer',
            }"
            class="rounded px-0.5 hover:bg-gray-100 hover:text-primary-900"
            :class="isFocusAbsolute ? 'text-gray-700' : 'text-gray-400'"
            @click="fireAction(action)"
          >
            <IconInline v-bind="action.icon" class="w-5 text-center" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
