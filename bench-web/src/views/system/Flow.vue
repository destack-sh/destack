<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import {
	createStep,
	FLOW_CANVAS_DOT_SIZE,
	FLOW_GRID_STEP_X,
	FLOW_GRID_STEP_Y,
	FlowContext,
	STEP_WIDTH
} from "@/language/flow";
import { NodeType, StepType, Variant, ViewData } from "@/proto/wire";
import { toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import { fireAction, getAction, type ActionBuiltinId, type ActionMapImplementation } from "@/ui/action";
import { ICON_BY_STEP_TYPE, IconInline } from "@/ui/icon";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Step from "@/views/system/Step.vue";
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

const containerRef: Ref<HTMLElement | null> = ref(null);
const stepRefs: Ref<Record<string, InstanceType<typeof Step>>> = ref({});
const ctx = new FlowContext({
  spaceGraph: spaceGraph,
  spaceTx: () => spaceConnection.tx,
  graph: pkgGraph,
  tx: () => pkgConnection.tx,
  view: selfView,
	containerRef,
  stepRefs: stepRefs,
  flowPtr: nodePtr,
});
const flow = ctx.flow;
const scale = ctx.scale;
const steps = ctx.steps;
const pipes = ctx.pipes;

// actions
const actions: Partial<ActionMapImplementation<"common">> = {
  "common.navigate.left": {
    action: () => ctx.panCanvas({ x: -FLOW_GRID_STEP_X, y: 0 }),
  },
  "common.navigate.right": {
    action: () => ctx.panCanvas({ x: FLOW_GRID_STEP_X, y: 0 }),
  },
  "common.navigate.up": {
    action: () => ctx.panCanvas({ x: 0, y: -FLOW_GRID_STEP_Y }),
  },
  "common.navigate.down": {
    action: () => ctx.panCanvas({ x: 0, y: FLOW_GRID_STEP_Y }),
  },
  "common.navigate.zoomIn": {
    action: () => ctx.zoomCanvas("in", "center", 15),
  },
  "common.navigate.zoomOut": {
    action: () => ctx.zoomCanvas("out", "center", 15),
  },
  "common.navigate.reset": {
    action: () => ctx.resetViewport(),
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
    :class="[
      variant == Variant.COMPACT ? 'rounded border border-gray-200' : '',
      ctx.dragging.value ? 'cursor-grabbing' : 'cursor-grab',
    ]"
    @mousedown="(e) => ctx.startDragging(e, 'canvas')"
    @mousemove="(e: MouseEvent) => ctx.onDragging(e)"
    @mouseup.stop="ctx.dragging.value = null"
    @mouseleave.stop="ctx.dragging.value = null"
    @dragstart.stop.prevent="false"
    @wheel.prevent="(e) => ctx.onWheel(e)"
  >
    <!-- NOTE :UX: handle multitouch gestures -->
    <!-- nocheckin: flow UI: actions, contextmenus, pipes, everything.. -->
    <!-- Background grid (infinitely repeated) -->
    <div class="absolute h-full w-full overflow-hidden" :style="{}">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-full w-full transition-colors duration-75"
        :class="[
          isFocusAbsolute || variant != Variant.COMPACT
            ? 'text-gray-200 '
            : 'text-gray-100  group-hover/flow:text-gray-200',
        ]"
        :style="{
          // extra spacing for smooth infinite scrolling
          width: `${100 / scale}%`,
          height: `${100 / scale}%`,
          transformOrigin: '0 0',
          transform: `scale(${scale}, ${scale}) translate(${((transform?.translateX ?? 0) % FLOW_GRID_STEP_X) - FLOW_CANVAS_DOT_SIZE / 2}px, ${((transform?.translateY ?? 0) % FLOW_GRID_STEP_Y) - FLOW_CANVAS_DOT_SIZE / 2}px)`,
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
              :cx="FLOW_CANVAS_DOT_SIZE / 2"
              :cy="FLOW_CANVAS_DOT_SIZE / 2"
              :r="FLOW_CANVAS_DOT_SIZE / 2"
              fill="currentColor"
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
          @mousedown="(e) => ctx.startDragging(e, step)"
        />

        <!-- nocheckin: pipes and stuff -->
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
            @click="flow && createStep(pkgConnection.tx, pkgGraph, { step: { type: stepType }, parent: flow })"
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
