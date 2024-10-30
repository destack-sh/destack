<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import {
  createStep,
  FLOW_CANVAS_DOT_SIZE,
  FLOW_CONTEXT_KEY,
  FLOW_GRID_STEP,
  FlowContext,
  getStepWidth,
  pathToSvg,
  PIPE_CONTEXT_ACTIONS,
  PIPE_WIDTH,
  STEP_CONTEXT_ACTIONS,
} from "@/language/flow";
import { cloneNode } from "@/language/node";
import {
  ChangeCategory,
  ColorShade,
  ColorType,
  NodeReferenceData,
  NodeType,
  PipeData,
  PortSide,
  StepData,
  StepType,
  Variant,
  ViewData,
} from "@/proto/wire";
import { toNodeRefOneOf, unwrapProtoOneOf, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import {
  fireAction,
  getAction,
  type ActionBuiltinId,
  type ActionContext,
  type ActionMapImplementation,
} from "@/ui/action";
import { ICON_BY_STEP_TYPE, IconInline } from "@/ui/icon";
import { menuActionsLike, type PopoverContext, type PopoverInfo } from "@/ui/popover";
import { COLOR_BY_STEP_TYPE, getColorHex } from "@/ui/style";
import { computedValue } from "@/utils/ref";
import NodePath from "@/views/builtins/NodePath.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Pipe from "@/views/system/Pipe.vue";
import Step from "@/views/system/Step.vue";
import { computed, provide, ref, toRef, watchEffect, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "icon" | "nodePtr" | "focus" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self, { isRequired: false });
const selfView = spaceGraph.getRef(self);
const nodePtr = computed(() => unwrapProtoOneOf(props.nodePtr) as TypedNodeReferenceData<NodeType.BLOCK>);
const pkgGetConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph: pkgGraph, connection: pkgConnection } = pkgGetConnection;

const containerRef: Ref<HTMLElement | null> = ref(null);
const stepRefs: Ref<Record<string, InstanceType<typeof Step>>> = ref({});
const pipeRefs: Ref<Record<string, InstanceType<typeof Pipe>>> = ref({});
const flowCtx = new FlowContext({
  spaceGraph: spaceGraph,
  spaceTx: () => spaceConnection.tx.with({ category: ChangeCategory.SPACE }),
  graph: pkgGraph,
  tx: () => pkgConnection.tx,
  update: state.update,
  view: selfView,
  transform: toRef(props, "transform"),
  containerRef,
  stepRefs: stepRefs,
  flowPtr: nodePtr,
});
provide(FLOW_CONTEXT_KEY, flowCtx);
const flow = flowCtx.flow;
const scale = flowCtx.scale;
const steps = flowCtx.steps;
const pipes = flowCtx.pipes;
const things: Ref<(StepData | PipeData)[]> = computed(() => [...steps.value, ...pipes.value]);
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);
const pendingPath = computed(() => {
  if (flowCtx.draggable?.kind != "step-port") return null;
  return flowCtx.computePath(
    "manhattan",
    flowCtx.getPortPosition(flowCtx.draggable.step, flowCtx.draggable.port.side)!,
    flowCtx.draggable.port.side,
    flowCtx.draggable.cursorWorldPos!,
    flowCtx.draggable.port.side == PortSide.OUTGOING ? PortSide.INCOMING : PortSide.OUTGOING,
  );
});

//
// Interaction
//

// actions
const getThingFromContext = (ctx: ActionContext | undefined): { thing: StepData | PipeData | null; idx: number } => {
  let thingIdx: number | undefined = undefined;
  if (thingIdx === undefined && ctx?.triggerNode?.id != null)
    thingIdx = things.value.findIndex((thing) => thing.id == ctx!.triggerNode!.id);
  if (thingIdx === undefined && focusedNodePtr.value?.id != null)
    thingIdx = things.value.findIndex((thing) => thing.id == focusedNodePtr.value!.id);
  if (thingIdx === undefined) return { thing: null, idx: -1 };
  const thing = things.value[thingIdx];
  return { thing, idx: thingIdx };
};
const actions: Partial<ActionMapImplementation<"common" | "session">> = {
  // create
  "common.create.step": (action, context) => {
    if (flow.value == null) return false;
    createStep(pkgConnection.tx, pkgGraph, {
      parent: flow.value,
      step: { type: StepType.ACTION },
      near: flowCtx.centerVec,
    });
  },
  // move
  "common.move.up": (action, context) => {
    const { thing } = getThingFromContext(context);
    if (thing == null) return false;
    flowCtx.moveThing(thing, { x: 0, y: -FLOW_GRID_STEP });
  },
  "common.move.down": (action, context) => {
    const { thing } = getThingFromContext(context);
    if (thing == null) return false;
    flowCtx.moveThing(thing, { x: 0, y: FLOW_GRID_STEP });
  },
  "common.move.left": (action, context) => {
    const { thing } = getThingFromContext(context);
    if (thing == null) return false;
    flowCtx.moveThing(thing, { x: -FLOW_GRID_STEP, y: 0 });
  },
  "common.move.right": (action, context) => {
    const { thing } = getThingFromContext(context);
    if (thing == null) return false;
    flowCtx.moveThing(thing, { x: FLOW_GRID_STEP, y: 0 });
  },
  // edit
  "common.edit.duplicate": (action, context) => {
    const { thing } = getThingFromContext(context);
    if (thing == null) return false;
    const duplicate = cloneNode(pkgConnection.tx, pkgGraph, thing, { includeChildren: true });
  },
  "common.edit.delete": (action, context) => {
    const { thing } = getThingFromContext(context);
    if (thing == null) return false;
    pkgConnection.tx.delete(thing);
  },
  // navigate
  "common.navigate.left": () => flowCtx.pan({ x: -FLOW_GRID_STEP, y: 0 }),
  "common.navigate.right": () => flowCtx.pan({ x: FLOW_GRID_STEP, y: 0 }),
  "common.navigate.up": () => flowCtx.pan({ x: 0, y: -FLOW_GRID_STEP }),
  "common.navigate.down": () => flowCtx.pan({ x: 0, y: FLOW_GRID_STEP }),
  "common.navigate.zoomIn": () => flowCtx.zoom("in", "center", 15),
  "common.navigate.zoomOut": () => flowCtx.zoom("out", "center", 15),
  "common.navigate.reset": () => flowCtx.resetViewport(),
};

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  if (typeof anchor == "object") {
    if (stepRefs.value[anchor.id!] != null) {
      const stepState = flowCtx.stepsStates.value[anchor.id!];
      if (stepState.step.value != null && !flowCtx.isInViewport({ kind: "step", step: stepState.step.value })) {
        flowCtx.panToCenter({ kind: "step", step: stepState.step.value! });
      }
      return stepRefs.value[anchor.id!].$el;
    } else if (pipeRefs.value[anchor.id!] != null) {
      const pipeState = flowCtx.pipesStates.value[anchor.id!];
      if (pipeState.pipe.value != null && !flowCtx.isInViewport({ kind: "pipe", pipe: pipeState.pipe.value })) {
        flowCtx.panToCenter({ kind: "pipe", pipe: pipeState.pipe.value! });
      }
      return pipeRefs.value[anchor.id!].$el;
    }
  }
  return false;
}

const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);
defineExpose<ViewExposed>({ self, id, actions, focus });
</script>
<template>
  <div
    ref="containerRef"
    v-contextmenu="
      (context: PopoverContext): PopoverInfo => ({
        kind: 'menu',
        placement: 'bottom-right',
        items: menuActionsLike(
          ['common.create.step', 'common.create.pipe', 'common.edit.paste', 'message.chat.message'],
          { context: { ...context, triggerNode: flow! } },
        ),
      })
    "
    class="group/flow relative w-full select-none"
    :class="[
      variant == Variant.COMPACT || variant == Variant.STEALTH ? '' : 'h-full',
      flowCtx.dragging.value ? (flowCtx.isDraggingPort ? 'cursor-crosshair' : 'cursor-grabbing') : 'cursor-grab',
    ]"
    @mousedown="(e) => flowCtx.startDraggingIfAllowed(e, { kind: 'canvas' })"
    @mousemove="(e: MouseEvent) => flowCtx.onDragging(e)"
    @mouseup="(e) => flowCtx.endDragging(e, { kind: 'canvas' })"
    @mouseleave="(e) => flowCtx.cancelDragging()"
    @wheel.prevent="(e) => flowCtx.onWheel(e)"
  >
    <!-- NOTE :UX: handle multitouch gestures -->
    <!-- Background grid (infinitely repeated) -->
    <div class="absolute h-full w-full overflow-hidden" :style="{}">
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="h-full w-full text-gray-200"
        :style="{
          // extra spacing for smooth infinite scrolling
          width: `${100 / scale}%`,
          height: `${100 / scale}%`,
          transformOrigin: '0 0',
          transform: `scale(${scale}, ${scale}) translate(${((transform?.translateX ?? 0) % FLOW_GRID_STEP) - FLOW_CANVAS_DOT_SIZE / 2}px, ${((transform?.translateY ?? 0) % FLOW_GRID_STEP) - FLOW_CANVAS_DOT_SIZE / 2}px)`,
        }"
      >
        <defs>
          <pattern
            id="dot-pattern"
            :x="0"
            :y="0"
            :width="FLOW_GRID_STEP"
            :height="FLOW_GRID_STEP"
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
        <!-- Pipes -->
        <Pipe
          v-for="pipe in pipes"
          :id="pipe.id"
          :ref="(ref: any) => (ref != null ? (pipeRefs[pipe.id] = ref) : delete pipeRefs[pipe.id])"
          :key="pipe.id"
          v-contextmenu="
            (context: PopoverContext): PopoverInfo => ({
              kind: 'menu',
              placement: 'bottom-right',
              items: menuActionsLike(PIPE_CONTEXT_ACTIONS, { context: { ...context, triggerNode: pipe } }),
            })
          "
          :node-ptr="toNodeRefOneOf(pipe)"
          class="absolute"
        />
        <!-- Pending Pipe (above Steps for clarity)-->
        <div
          v-if="flowCtx.draggable?.kind == 'step-port'"
          class="pointer-events-none absolute text-gray-700 opacity-50"
        >
          <svg v-if="pendingPath" class="overflow-visible">
            <path
              :stroke-width="PIPE_WIDTH"
              stroke-linecap="round"
              stroke-linejoin="bevel"
              stroke="currentColor"
              fill="none"
              :d="pathToSvg(pendingPath.points)"
            />
          </svg>
        </div>
        <!-- Steps -->
        <Step
          v-for="step in steps"
          :id="step.id"
          :ref="(ref: any) => (ref ? (stepRefs[step.id] = ref) : delete stepRefs[step.id])"
          :key="step.id"
          v-contextmenu="
            (context: PopoverContext): PopoverInfo => ({
              kind: 'menu',
              placement: 'bottom-right',
              items: menuActionsLike(STEP_CONTEXT_ACTIONS, { context: { ...context, triggerNode: step } }),
            })
          "
          class="absolute"
          :style="{
            width: getStepWidth(step) + 'px',
            left: (step.position?.x ?? 0) + 'px',
            top: (step.position?.y ?? 0) + 'px',
          }"
          :node-ptr="toNodeRefOneOf(step)"
          @mousedown="(e) => flowCtx.startDraggingIfAllowed(e, { kind: 'step', step })"
        />
      </div>
    </div>

    <!-- Overlay -->
    <div
      class="pointer-events-none absolute left-0 top-0 flex w-full flex-row flex-wrap justify-between gap-y-3 overflow-hidden p-1"
    >
      <!-- Node path -->
      <NodePath
        v-if="variant != Variant.STEALTH && variant != Variant.COMPACT && flowCtx.flow.value != null"
        class="pointer-events-auto flex-shrink-0 border border-gray-300 bg-white px-2"
        :container="nodePtr"
        :focus="props.focus?.nodesPtr[0]"
        :graph="pkgGraph"
      />
      <!-- Menu (Create) -->
      <div
        v-if="variant != Variant.STEALTH"
        class="pointer-events-auto z-20 flex w-fit flex-row items-center gap-x-1 rounded-sm border border-gray-300 bg-white px-2 py-1.5"
        :class="variant != Variant.COMPACT ? 'absolute left-1/2 -translate-x-1/2' : ''"
      >
        <!-- Create -->
        <button
          v-for="stepType in [StepType.START, StepType.TRIGGER, StepType.ACTION, StepType.TEXT]"
          :key="stepType"
          v-tooltip="{
            title: `${toCamelName(StepType, stepType)}`,
            showDelay: 200,
            hideDelay: 100,
            small: true,
            referenceMargin: 8,
            group: 'flow.create',
          }"
          class="rounded-sm px-0.5 text-gray-700 hover:bg-gray-100 hover:text-primary-700"
          @click="
            flow &&
              createStep(pkgConnection.tx, pkgGraph, {
                step: { type: stepType },
                parent: flow,
                near: flowCtx.centerVec,
              })
          "
        >
          <IconInline v-bind="ICON_BY_STEP_TYPE[stepType]" class="w-5 text-center" />
        </button>
      </div>
      <!-- Menu (Actions) -->
      <div
        class="pointer-events-auto z-20 flex w-fit flex-row items-center gap-x-1 rounded-sm border border-gray-300 bg-white px-2 py-1.5"
      >
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
            group: 'flow.actions',
          }"
          class="rounded-sm px-0.5 text-gray-700 hover:bg-gray-100 hover:text-primary-700"
          @click="fireAction(action)"
        >
          <IconInline v-bind="action.icon" class="w-5 text-center" />
        </button>
      </div>
    </div>
  </div>
</template>
