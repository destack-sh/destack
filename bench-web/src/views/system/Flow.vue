<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import { createStep, FLOW_GRID_STEP_X, FLOW_GRID_STEP_Y, STEP_WIDTH } from "@/language/flow";
import { NodeType, StepData, StepType, StructType, Variant, Vector2Data, ViewData } from "@/proto/wire";
import { isNode, makeStruct, toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { fireAction, getAction, type ActionBuiltinId, type ActionMapImplementation } from "@/ui/action";
import { ICON_BY_STEP_TYPE, IconInline } from "@/ui/icon";
import { addTransform } from "@/ui/view";
import { assertNever, roundToStep } from "@/utils/functools";
import { log } from "@/utils/log";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Step from "@/views/system/Step.vue";
import { useElementSize } from "@vueuse/core";
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

const containerRef: Ref<HTMLElement | null> = ref(null);
const stepRefs: Ref<Record<string, InstanceType<typeof Step>>> = ref({});
const dragging: Ref<{ thing: StepData | "canvas"; viewOffsetToThing: { x: number; y: number } } | null> = ref(null);
const transform = computed(
  () =>
    selfView.value?.transform ??
    makeStruct({ metatype: StructType.TRANSFORM, translateX: FLOW_GRID_STEP_X / 2, translateY: FLOW_GRID_STEP_Y / 2 }),
);
const scale = computed(() => transform.value.scaleX ?? transform.value.scaleY ?? 1);

function getStepComponent(step: StepData): InstanceType<typeof Step> | null {
  const stepRef = stepRefs.value[step.id!];
  return stepRef != null ? stepRef : null;
}

function getStepBounding(step: StepData): DOMRect | null {
  const stepComponent = getStepComponent(step);
  if (stepComponent == null) return null;
  return stepComponent.$el.getBoundingClientRect();
}

//
// Interaction
//

const DOT_SIZE = 4;
const SCALE_MIN = 0.5;
const SCALE_MAX = 2.0;
const SCALE_STEP = 0.1;

/** Pan around the canvas */
function panCanvas(move: { x: number; y: number }) {
  if (props.self == null) return; // not a real view
  const self = spaceGraph.getOrError(props.self);
  spaceConnection.tx.update(
    self,
    { transform: addTransform(transform.value, { translateX: move.x, translateY: move.y }) },
    { debounce: "long" },
  );
}

function snapVec(vec: { x: number; y: number }): { x: number; y: number } {
  return {
    x: Math.round(vec.x / FLOW_GRID_STEP_X) * FLOW_GRID_STEP_X,
    y: Math.round(vec.y / FLOW_GRID_STEP_Y) * FLOW_GRID_STEP_Y,
  };
}

/** Convert viewport coordinates to view coordinates */
function viewportToViewVec(viewportVec: { x: number; y: number }): { x: number; y: number } {
  const canvasBounding = containerRef.value?.getBoundingClientRect()!;
  return {
    x: viewportVec.x - canvasBounding.left,
    y: viewportVec.y - canvasBounding.top,
  };
}

/** Convert world coordinates to view coordinates */
function worldToViewVec(worldVec: { x: number; y: number }): { x: number; y: number } {
  let viewVec = {
    x: worldVec.x + (transform.value.translateX ?? 0),
    y: worldVec.y + (transform.value.translateY ?? 0),
  };
  viewVec = { x: viewVec.x * scale.value, y: viewVec.y * scale.value };
  return viewVec;
}

/** Convert screen coordinates to world coordinates */
function viewToWorldVec(viewVec: { x: number; y: number }): { x: number; y: number } {
  let worldVec = { x: viewVec.x / scale.value, y: viewVec.y / scale.value };
  worldVec = { x: worldVec.x - (transform.value.translateX ?? 0), y: worldVec.y - (transform.value.translateY ?? 0) };
  return worldVec;
}

/** Zoom the convas around the given origin (pan to keep the same point in view in screen space) */
function zoomCanvas(direction: "in" | "out", viewCenterVec: { x: number; y: number } | "center") {
  if (props.self == null) return; // not a real view
  const containerBounding = containerRef.value?.getBoundingClientRect()!;
  if (viewCenterVec == "center") {
    viewCenterVec = { x: containerBounding.width / 2, y: containerBounding.height / 2 };
  }
  const self = spaceGraph.getOrError(props.self);

  // figure out new zoom
  const currentZoom = roundToStep(transform.value?.scaleX ?? 1, SCALE_STEP);
  const newZoom =
    direction == "in" ? Math.min(SCALE_MAX, currentZoom + SCALE_STEP) : Math.max(SCALE_MIN, currentZoom - SCALE_STEP);

  // pan to move towards the origin
	const currentViewCenter = { x: containerBounding.width / 2, y: containerBounding.height / 2 };
	// nocheckin

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

function onDragging(e: MouseEvent) {
  if (dragging.value == null) return;
  if (props.self == null) return; // not a real view
  if (dragging.value.thing == "canvas") {
    // pan canvas
    const self = spaceGraph.getOrError(props.self);
    const translateX = e.movementX / (transform.value?.scaleX ?? 1);
    const translateY = e.movementY / (transform.value?.scaleY ?? 1);
    spaceConnection.tx.update(
      self,
      { transform: addTransform(transform.value, { translateX, translateY }) },
      { debounce: "long" },
    );
  } else if (isNode(dragging.value.thing, NodeType.STEP)) {
    // move step (snap to grid)
    const screenVec = viewportToViewVec({
      x: e.clientX - dragging.value.viewOffsetToThing.x,
      y: e.clientY - dragging.value.viewOffsetToThing.y,
    });
    const worldVec = snapVec(viewToWorldVec(screenVec));
    pkgConnection.tx.update(
      dragging.value.thing,
      { position: makeStruct({ metatype: StructType.VECTOR2, x: worldVec.x, y: worldVec.y }) },
      { debounce: "long" },
    );
  } else {
    assertNever(dragging.value.thing);
  }
}

function startDragging(e: MouseEvent, thing: StepData | "canvas") {
  if (dragging.value != null) return; // already dragging
  if (thing == "canvas") {
    // start panning canvas
    dragging.value = { thing: "canvas", viewOffsetToThing: viewportToViewVec({ x: e.clientX, y: e.clientY }) };
  } else if (isNode(thing, NodeType.STEP)) {
    // start dragging step
    const stepBounding = getStepBounding(thing);
    if (stepBounding == null) return; // not mounted yet
    dragging.value = {
      thing,
      viewOffsetToThing: { x: e.clientX - stepBounding.left, y: e.clientY - stepBounding.top },
    };
  } else {
    assertNever(thing);
  }
  log.trace("flow.drag.start", dragging.value);
}

function onWheel(e: WheelEvent) {
	const viewCenterVec = viewportToViewVec({ x: e.clientX, y: e.clientY });
  zoomCanvas(e.deltaY < 0 ? "in" : "out", viewCenterVec);
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
    action: () => zoomCanvas("in", "center"),
  },
  "common.navigate.zoomOut": {
    action: () => zoomCanvas("out", "center"),
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
    ref="containerRef"
    class="group/flow relative h-full w-full"
    :class="[variant == Variant.COMPACT ? 'rounded border border-gray-200' : '', dragging ? 'cursor-grabbing' : '']"
    @mousedown="(e) => startDragging(e, 'canvas')"
    @mousemove="(e: MouseEvent) => onDragging(e)"
    @mouseup.stop="dragging = null"
    @mouseleave.stop="dragging = null"
    @dragstart.stop.prevent="false"
    @wheel.prevent="onWheel"
  >
    <!-- nocheckin: flow UI: actions, contextmenus, pipes, everything.. -->
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
        class="relative h-full w-full"
        :style="{
          transformOrigin: '0 0',
          transform: `scale(${scale}, ${scale}) translate(${transform?.translateX ?? 0}px, ${transform?.translateY ?? 0}px) `,
        }"
      >
        <!-- Steps -->
        <Step
          v-for="step in steps"
          :ref="(ref: any) => (ref ? (stepRefs[step.id] = ref) : delete stepRefs[step.id])"
          :key="step.id"
          class="absolute"
          :style="{
            width: STEP_WIDTH + 'px',
            left: (step.position?.x ?? 0) + 'px',
            top: (step.position?.y ?? 0) + 'px',
          }"
          :node-ptr="toNodeRefOneOf(step)"
          @mousedown="(e) => startDragging(e, step)"
        />

        <!-- nocheckin: pipes and stuff -->

        <!-- Placeholder for spacing check -->
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
    <div class="absolute left-0 top-0 w-full">
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
