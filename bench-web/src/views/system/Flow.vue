<script lang="ts" setup>
import { toCamelName } from "@/language/const";
import {
  createStep,
  FLOW_CANVAS_DOT_SIZE,
  FLOW_CONTEXT_KEY,
  FLOW_GRID_STEP_X,
  FLOW_GRID_STEP_Y,
  FlowContext,
  PIPE_CONTEXT_ACTIONS,
  PIPE_WIDTH,
  STEP_CONTEXT_ACTIONS,
} from "@/language/flow";
import { cloneNode } from "@/language/node";
import { NodeType, PipeData, StepData, StepType, Variant, ViewData } from "@/proto/wire";
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
import { computedValue } from "@/utils/ref";
import { makeViewId, viewEmits, type ViewExposed } from "@/views/common";
import Pipe from "@/views/system/Pipe.vue";
import Step from "@/views/system/Step.vue";
import { computed, nextTick, provide, ref, toRef, type Ref } from "vue";

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "name" | "title" | "text" | "icon" | "nodePtr" | "focus" | "transform" | "variant">
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
provide(FLOW_CONTEXT_KEY, ctx);
const flow = ctx.flow;
const scale = ctx.scale;
const steps = ctx.steps;
const pipes = ctx.pipes;
const things: Ref<(StepData | PipeData)[]> = computed(() => [...steps.value, ...pipes.value]);
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);

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
const actions: Partial<ActionMapImplementation<"common">> = {
  // move
  "common.move.up": {
    action: (action, context) => {
      const { thing } = getThingFromContext(context);
      if (thing == null) return false;
      ctx.moveThing(thing, { x: 0, y: -FLOW_GRID_STEP_Y });
    },
  },
  "common.move.down": {
    action: (action, context) => {
      const { thing } = getThingFromContext(context);
      if (thing == null) return false;
      ctx.moveThing(thing, { x: 0, y: FLOW_GRID_STEP_Y });
    },
  },
  "common.move.left": {
    action: (action, context) => {
      const { thing } = getThingFromContext(context);
      if (thing == null) return false;
      ctx.moveThing(thing, { x: -FLOW_GRID_STEP_X, y: 0 });
    },
  },
  "common.move.right": {
    action: (action, context) => {
      const { thing } = getThingFromContext(context);
      if (thing == null) return false;
      ctx.moveThing(thing, { x: FLOW_GRID_STEP_X, y: 0 });
    },
  },
  // edit
  "common.edit.duplicate": {
    action: (action, context) => {
      const { thing } = getThingFromContext(context);
      if (thing == null) return false;
      const duplicate = cloneNode(pkgConnection.tx, pkgGraph, thing, { includeChildren: true });
    },
  },
  "common.edit.archive": {
    action: (action, context) => {
      const { thing } = getThingFromContext(context);
      if (thing == null) return false;
      pkgConnection.tx.archive(thing);
    },
  },
  "common.edit.delete": {
    action: (action, context) => {
      const { thing } = getThingFromContext(context);
      if (thing == null) return false;
      pkgConnection.tx.delete(thing);
    },
  },
  // navigate
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
    v-contextmenu="
      (context: PopoverContext): PopoverInfo => ({
        kind: 'menu',
        placement: 'bottom-right',
        items: menuActionsLike(['common.create.here', 'common.edit.paste', 'message.chat.message'], {
          context: { ...context, triggerNode: flow! },
        }),
      })
    "
    class="group/flow relative h-full w-full"
    :class="[
      variant == Variant.COMPACT ? 'rounded border border-gray-200' : '',
      ctx.dragging.value ? (ctx.isDraggingPort ? 'cursor-crosshair' : 'cursor-grabbing') : 'cursor-grab',
    ]"
    @mousedown="(e) => ctx.startDraggingIfAllowed(e, { kind: 'canvas' })"
    @mousemove="(e: MouseEvent) => ctx.onDragging(e)"
    @mouseup="(e) => ctx.endDragging(e, { kind: 'canvas' })"
    @mouseleave="(e) => ctx.cancelDragging()"
    @wheel.prevent="(e) => ctx.onWheel(e)"
  >
    <!-- NOTE :UX: handle multitouch gestures -->
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
        <!-- Pipes -->
        <!-- NOTE: we draw Pipes behind Steps -->
        <Pipe
          v-for="pipe in pipes"
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
        <!-- Steps -->
        <Step
          v-for="step in steps"
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
            width: ctx.getStepWidth(step) + 'px',
            left: (step.position?.x ?? 0) + 'px',
            top: (step.position?.y ?? 0) + 'px',
          }"
          :node-ptr="toNodeRefOneOf(step)"
          @mousedown="(e) => ctx.startDraggingIfAllowed(e, { kind: 'step', step })"
        />
        <!-- Pending Pipe (above Steps for clarity)-->
        <div v-if="ctx.draggable?.kind == 'step-port'" class="pointer-events-none absolute text-gray-700 opacity-50">
          <svg v-if="ctx.draggable.cursorWorldPos" class="overflow-visible">
            <path
              :stroke-width="PIPE_WIDTH"
              stroke-linecap="round"
              stroke-linejoin="bevel"
              stroke="currentColor"
              :d="
                ctx.computePathSvg(
                  ctx.getPortPosition(ctx.draggable.step, ctx.draggable.port)!,
                  ctx.draggable.cursorWorldPos!,
                )
              "
            />
          </svg>
        </div>
      </div>
    </div>

    <!-- Overlay -->
    <div class="absolute left-0 top-0 w-full">
      <!-- Menu -->
      <div
        v-if="self != null"
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
