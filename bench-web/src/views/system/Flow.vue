<script lang="ts" setup>
import { SOURCE_STEP_TYPES, toCamelName } from "@/language/const";
import {
  createStep,
  FLOW_CANVAS_DOT_SIZE,
  FLOW_CONTEXT_KEY,
  FLOW_GRID_STEP,
  FlowContext,
  pathToSvg,
  PIPE_WIDTH,
  PipePath,
  STEP_SIZE,
} from "@/language/flow";
import { newChangeId } from "@/language/transaction";
import {
  AnyNodeData,
  ChangeCategory,
  FieldType,
  NodeReferenceData,
  NodeType,
  PipeData,
  StepData,
  StepType,
  Variant,
  ViewData,
} from "@/proto/wire";
import { isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { useExistingConnection, type PreparedGetConnection } from "@/system/connection";
import { canvas } from "@/system/space";
import {
  PIPE_CONTEXT_ACTIONS,
  STEP_CONTEXT_ACTIONS,
  type ActionContext,
  type ActionMapImplementation,
} from "@/ui/action";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import { ICON_BY_STEP_TYPE, IconInline } from "@/ui/icon";
import { VIEW_DEFAULT_BAR_HEADER_HEIGHT, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import { computedValue } from "@/utils/ref";
import HistoryNavigator from "@/views/builtins/HistoryNavigator.vue";
import Inaccessible from "@/views/builtins/Inaccessible.vue";
import NodePath from "@/views/builtins/NodePath.vue";
import NodeReference from "@/views/builtins/NodeReference.vue";
import SelectionOverlay from "@/views/builtins/SelectionOverlay.vue";
import { viewEmits, type FocusAnchor, type ViewExposed } from "@/views/common";
import Pipe from "@/views/system/Pipe.vue";
import Step from "@/views/system/Step.vue";
import Type from "@/views/system/Type.vue";
import { useElementSize, useScroll } from "@vueuse/core";
import { computed, onMounted, provide, ref, toRef, type Ref } from "vue";

const BACKGROUND_STYLE: "checker" | "dots" = "dots";
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const GUTTER_WIDTH = 60;

const props = defineProps<
  { self?: TypedNodeReferenceData<NodeType.VIEW>; id: string; preparedConnection?: PreparedGetConnection } & Partial<
    Pick<ViewData, "icon" | "nodePtr" | "focus" | "transform" | "variant">
  >
>();
const emit = defineEmits(viewEmits());
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

const { graph: spaceGraph, connection: spaceConnection } = useExistingConnection(self, { isRequired: false });
const selfView = spaceGraph.getRef(self);
const nodePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.BLOCK>);
const preparedConnection = props.preparedConnection ?? useExistingConnection(nodePtr);
const { graph, connection } = preparedConnection;

const containerRef = ref<HTMLElement | null>(null);
const historyRef: Ref<InstanceType<typeof HistoryNavigator> | null> = ref(null);
const headerRef: Ref<HTMLElement | null> = ref(null);
const bodyRef: Ref<HTMLElement | null> = ref(null);
const stepRefs: Ref<Record<string, InstanceType<typeof Step>>> = ref({});
const pipeRefs: Ref<Record<string, InstanceType<typeof Pipe>>> = ref({});
const headerSize = useElementSize(headerRef);
const containerScroll = useScroll(bodyRef);

const flowContext = new FlowContext({
  spaceGraph: spaceGraph,
  spaceTx: () => spaceConnection.tx.with({ category: ChangeCategory.SPACE }),
  graph: graph,
  tx: () => connection.tx,
  update: state.update,
  view: selfView,
  transform: toRef(props, "transform"),
  containerRef: bodyRef,
  stepRefs: stepRefs,
  flowPtr: nodePtr,
});
provide(FLOW_CONTEXT_KEY, flowContext);
const flow = flowContext.flow;
const steps = flowContext.steps;
const pipes = flowContext.pipes;
const fields = flowContext.fields;
const stepsAndPipes: Ref<(StepData | PipeData)[]> = computed(() => [...steps.value, ...pipes.value]);

const viewport = flowContext.viewport;
const focusedNodePtr = computedValue(() => props.focus?.nodesPtr[0]);
const pendingPath: Ref<PipePath | null> = computed(() => {
  if (flowContext.draggable?.kind != "port") return null;
  // preview path between current dragged port and step (or point in canvas if nothing)
  const sourceStep = flowContext.draggable.step;
  const sourceBounding = flowContext.getStepBoundingBox(sourceStep);
  if (sourceBounding == null) return null;
  const cursor = flowContext.cursorWorldPos.value;
  const targetStep = flowContext.getStepAt(cursor);
  const isValid =
    targetStep != null &&
    !SOURCE_STEP_TYPES.includes(targetStep.type) &&
    !flowContext.pipes.value.some((pipe) => pipe.sourcePtr?.ck == sourceStep.ck && pipe.targetPtr?.ck == targetStep.ck);
  if (isValid) {
    // real path preview
    const target = flowContext.getStepBoundingBox(targetStep);
    if (target == null) return null;
    return flowContext.computePath(sourceBounding, target);
  } else {
    // just direct path
    const sourceMidpoint = {
      x: sourceBounding.x1 + sourceBounding.width / 2,
      y: sourceBounding.y1 + sourceBounding.height / 2,
    };
    const midpoint = { x: (sourceMidpoint.x + cursor.x) / 2, y: (sourceMidpoint.y + cursor.y) / 2 };
    return { start: sourceMidpoint, end: cursor, midpoint };
  }
});

//
// Interaction
//

// wait for initial render to complete so the transition-all doesn't look glitchy on mount
const isInitialRender = ref(true);
onMounted(() => {
  setTimeout(() => {
    isInitialRender.value = false;
  }, 100);
});
const shouldAnimateTransform = computed(
  // () => !isInitialRender.value && !flowContext.isDragging && !containerScroll.isScrolling.value,
  () =>
    false /* NOTE :UX: animating Flow transform (offset/zoom) sometimes looks good, sometimes janky, so it's disabled */,
);

// selection
const selectionOverlayContainerRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionOverlayBodyRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZoneContainer = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayContainerRef });
const selectionZoneBody = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayBodyRef });

// actions
function moveNodes(nodes: AnyNodeData[], move: { x: number; y: number }) {
  const tx = flowContext.tx.with({ change: { key: newChangeId(), title: "Move" } });
  for (const thing of nodes) {
    if (isNode(thing, NodeType.STEP)) {
      flowContext.move(thing, move, { tx });
    }
  }
}
const actions: Partial<ActionMapImplementation<"space" | "session">> = {
  // move
  "space.move.up": (action, context) => {
    moveNodes(context.nodes ?? [], { x: 0, y: -FLOW_GRID_STEP });
  },
  "space.move.down": (action, context) => {
    moveNodes(context.nodes ?? [], { x: 0, y: FLOW_GRID_STEP });
  },
  "space.move.left": (action, context) => {
    moveNodes(context.nodes ?? [], { x: -FLOW_GRID_STEP, y: 0 });
  },
  "space.move.right": (action, context) => {
    moveNodes(context.nodes ?? [], { x: FLOW_GRID_STEP, y: 0 });
  },
  // navigate
  "space.navigate.left": () => flowContext.pan({ x: -FLOW_GRID_STEP, y: 0 }),
  "space.navigate.right": () => flowContext.pan({ x: FLOW_GRID_STEP, y: 0 }),
  "space.navigate.up": () => flowContext.pan({ x: 0, y: -FLOW_GRID_STEP }),
  "space.navigate.down": () => flowContext.pan({ x: 0, y: FLOW_GRID_STEP }),
  "space.navigate.zoomIn": () => flowContext.zoom("in", "center", 10),
  "space.navigate.zoomOut": () => flowContext.zoom("out", "center", 10),
  "space.navigate.reset": () => flowContext.resetViewport(),
  // select
  "space.select.all": () => canvas.select(stepsAndPipes.value),
};

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  if (typeof anchor == "object") {
    if (stepRefs.value[anchor.id!] != null) {
      const stepState = flowContext.stepsStates.value[anchor.id!];
      if (stepState.step.value != null && !flowContext.isInViewport({ kind: "step", step: stepState.step.value })) {
        flowContext.panToCenter({ kind: "step", step: stepState.step.value! });
      }
      return stepRefs.value[anchor.id!].$el;
    } else if (pipeRefs.value[anchor.id!] != null) {
      const pipeState = flowContext.pipesStates.value[anchor.id!];
      if (pipeState.pipe.value != null && !flowContext.isInViewport({ kind: "pipe", pipe: pipeState.pipe.value })) {
        flowContext.panToCenter({ kind: "pipe", pipe: pipeState.pipe.value! });
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
    :class="[variant == Variant.COMPACT ? '' : 'h-full']"
    @mousedown="(e) => startSelectingIfAllowed(selectionZoneContainer, e)"
  >
    <!-- Meta header -->
    <div
      v-if="variant != Variant.COMPACT"
      data-keep-inspection-in-base-view="true"
      class="group flex w-full max-w-full flex-row items-center px-2"
      :style="{ height: (historyRef?.isActive ? VIEW_DEFAULT_BAR_HEADER_HEIGHT : HEADER_HEIGHT) + 'px' }"
    >
      <!-- History -->
      <HistoryNavigator ref="historyRef" :self="self" />
      <!-- Breadcrumb -->
      <NodePath :container="nodePtr" :self="nodePtr" :graph="graph" />
      <!-- Meta & Controls -->
      <div class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-1.5 pl-1">
        <!-- ... -->
      </div>
    </div>
    <!-- Header -->
    <div
      v-if="flow && variant != Variant.COMPACT"
      ref="columnHeaderRef"
      :style="{
        marginLeft: `${GUTTER_WIDTH}px`,
        marginRight: `${GUTTER_WIDTH}px`,
      }"
    >
      <!-- Title -->
      <NodeReference v-if="flow" class="mb-2 mt-5" size="title" is-input :node="flow" :tx="() => connection.tx" />
      <div class="flex flex-row flex-wrap items-center">
        <!-- Signature -->
        <div class="flex flex-row flex-wrap items-center gap-x-2 gap-y-1 border-gray-200">
          <Type
            id="type.input"
            class=""
            :node="flow"
            :prepared-connection="preparedConnection"
            :node-ptr="props.nodePtr"
            :field-type="FieldType.INPUT"
          />
          <i
            v-if="fields?.some((f) => f.type == FieldType.INPUT || f.type == FieldType.OUTPUT)"
            class="fas fa-arrow-right-long text-base text-gray-400"
          />
          <Type
            id="type.output"
            class=""
            :node="flow"
            :prepared-connection="preparedConnection"
            :node-ptr="props.nodePtr"
            :field-type="FieldType.OUTPUT"
          />
          <span v-if="fields?.some((f) => f.type == FieldType.VARIABLE)" class="text-xs text-gray-400">◆</span>
          <Type
            id="type.input"
            :node="flow"
            :prepared-connection="preparedConnection"
            :node-ptr="props.nodePtr"
            :field-type="FieldType.VARIABLE"
          />
        </div>
        <!-- Canvas controls -->
        <div class="ml-auto flex flex-shrink-0 flex-row items-center gap-x-0.5">
          <!-- Zoom -->
          <button
            class="rounded px-0.5 py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="flowContext.zoom('out', flowContext.centerVec!, 10)"
          >
            <i class="fas fa-minus w-5 text-center" />
          </button>
          <button
            class="rounded px-1 py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="flowContext.zoom(1.0, flowContext.centerVec!, 1)"
          >
            {{ Math.round(viewport.scale * 100) }}%
          </button>
          <button
            class="rounded px-0.5 py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="flowContext.zoom('in', flowContext.centerVec!, 10)"
          >
            <i class="fas fa-plus w-5 text-center" />
          </button>
          <!-- Auto/Reset -->
          <button
            class="rounded px-0.5 py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="flowContext.resetViewport()"
          >
            <i class="fas fa-arrows-to-dot w-5 text-center" />
          </button>
        </div>
      </div>
    </div>
    <!-- Canvas body -->
    <div
      v-if="flow"
      ref="bodyRef"
      class="group/flow relative w-full select-none"
      :class="[
        variant == Variant.COMPACT ? 'h-full' : '',
        flowContext.dragging.value ? (flowContext.isDraggingPort ? 'cursor-crosshair' : 'cursor-grabbing') : '',
      ]"
      :style="{
        height:
          variant != Variant.COMPACT
            ? `calc(100% - ${HEADER_HEIGHT + headerSize.height.value + 28 /* headerRef margin*/}px)`
            : undefined,
      }"
      @mousedown="(e) => startSelectingIfAllowed(selectionZoneBody, e)"
      @mousemove="(e) => flowContext.onDragging(e)"
      @mouseleave="flowContext.cancelDragging()"
      @mouseup="flowContext.cancelDragging()"
      @wheel="(e) => (variant != Variant.COMPACT ? flowContext.onWheel(e) : undefined)"
    >
      <!-- Background grid (infinitely repeated) -->
      <div class="absolute h-full w-full overflow-hidden" :style="{}">
        <svg
          v-if="BACKGROUND_STYLE == 'dots'"
          xmlns="http://www.w3.org/2000/svg"
          class="h-full w-full text-gray-200"
          :class="shouldAnimateTransform ? 'transition duration-150' : ''"
          :style="{
            // extra spacing for smooth infinite scrolling
            width: `${100 / viewport.scale}%`,
            height: `${100 / viewport.scale}%`,
            transformOrigin: '0 0',
            transform: `scale(${viewport.scale}, ${viewport.scale}) translate(${(viewport.transform.translateX % FLOW_GRID_STEP) - FLOW_CANVAS_DOT_SIZE / 2}px, ${(viewport.transform.translateY % FLOW_GRID_STEP) - FLOW_CANVAS_DOT_SIZE / 2}px)`,
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
        <svg
          v-else-if="BACKGROUND_STYLE == 'checker'"
          xmlns="http://www.w3.org/2000/svg"
          class="h-full w-full text-gray-200"
          :class="shouldAnimateTransform ? 'transition duration-150' : ''"
          :style="{
            width: `${105 / viewport.scale}%`,
            height: `${105 / viewport.scale}%`,
            transformOrigin: '0 0',
            transform: `scale(${viewport.scale}, ${viewport.scale}) translate(${(viewport.transform.translateX % FLOW_GRID_STEP) - FLOW_CANVAS_DOT_SIZE / 2}px, ${(viewport.transform.translateY % FLOW_GRID_STEP) - FLOW_CANVAS_DOT_SIZE / 2}px)`,
          }"
        >
          <defs>
            <pattern
              id="checker-pattern"
              :x="0"
              :y="0"
              :width="FLOW_GRID_STEP"
              :height="FLOW_GRID_STEP"
              patternUnits="userSpaceOnUse"
            >
              <rect x="0" y="0" :width="FLOW_GRID_STEP / 2" :height="FLOW_GRID_STEP / 2" fill="#f5f5f4" />
              <rect
                :x="FLOW_GRID_STEP / 2"
                y="0"
                :width="FLOW_GRID_STEP / 2"
                :height="FLOW_GRID_STEP / 2"
                fill="white"
              />
              <rect
                x="0"
                :y="FLOW_GRID_STEP / 2"
                :width="FLOW_GRID_STEP / 2"
                :height="FLOW_GRID_STEP / 2"
                fill="white"
              />
              <rect
                :x="FLOW_GRID_STEP / 2"
                :y="FLOW_GRID_STEP / 2"
                :width="FLOW_GRID_STEP / 2"
                :height="FLOW_GRID_STEP / 2"
                fill="#f5f5f4"
              />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#checker-pattern)" />
        </svg>
      </div>

      <!-- Contents -->
      <div class="z-0 h-full w-full overflow-hidden">
        <div
          class="relative h-full w-full"
          :class="shouldAnimateTransform ? 'transition duration-150' : ''"
          :style="{
            transformOrigin: '0 0',
            transform: `scale(${viewport.scale}, ${viewport.scale}) translate(${viewport.transform.translateX}px, ${viewport.transform.translateY}px) `,
          }"
        >
          <!-- Pipes -->
          <Pipe
            v-for="pipe in pipes"
            :id="pipe.id"
            :ref="(ref: any) => (ref != null ? (pipeRefs[pipe.id] = ref) : delete pipeRefs[pipe.id])"
            :key="pipe.id"
            :data-contextmenu-items="PIPE_CONTEXT_ACTIONS.join(',')"
            :node-ptr="toNodeRef(pipe)"
            class="absolute"
          />
          <!-- Pending Pipe (above Steps for clarity)-->
          <div
            v-if="flowContext.draggable?.kind == 'port'"
            class="pointer-events-none absolute text-gray-700 opacity-50"
          >
            <svg v-if="pendingPath" class="overflow-visible">
              <path
                :stroke-width="PIPE_WIDTH * 2"
                stroke-linecap="round"
                stroke-linejoin="bevel"
                stroke="currentColor"
                fill="none"
                :d="pathToSvg(pendingPath)"
              />
            </svg>
          </div>
          <!-- Steps -->
          <Step
            v-for="step in steps"
            :id="step.id"
            :ref="(ref: any) => (ref ? (stepRefs[step.id] = ref) : delete stepRefs[step.id])"
            :key="step.id"
            class="absolute"
            :class="[flowContext?.isDraggingStep(step) ? 'cursor-grabbing' : 'cursor-grab']"
            :style="{
              width: STEP_SIZE.width + 'px',
              left: (step.position?.x ?? 0) + 'px',
              top: (step.position?.y ?? 0) + 'px',
            }"
            :node-ptr="toNodeRef(step)"
            :data-contextmenu-items="STEP_CONTEXT_ACTIONS.join(',')"
            data-suppress-drag="select"
            @mousedown="(e) => flowContext.startDraggingIfAllowed(e, { kind: 'step', step: step! })"
          />
        </div>
      </div>

      <!-- Overlay -->
      <div
        v-if="flow"
        class="pointer-events-none absolute bottom-0 left-0 flex w-full flex-row items-center justify-center"
      >
        <!-- Menu -->
        <div
          class="rounded-b-0 pointer-events-auto z-20 flex w-fit flex-row items-center gap-x-1 rounded-t-2xl border-x border-t border-gray-200 bg-white px-2.5 py-1.5"
          data-suppress-drag="both"
          :class="
            variant != Variant.COMPACT ? '' : 'opacity-0 transition-colors duration-150 group-hover/flow:opacity-100'
          "
        >
          <!-- Create -->
          <button
            v-for="stepType in [
              StepType.START,
              StepType.COMPLETE,
              StepType.FAIL,
              StepType.ACTION,
              StepType.CREATE,
              StepType.YIELD,
              StepType.TEXT,
            ]"
            :key="stepType"
            v-tooltip="{
              title: `${toCamelName(StepType, stepType)}`,
              showDelay: 200,
              hideDelay: 100,
              small: true,
              referenceMargin: 8,
              group: 'flow.overlay',
            }"
            class="rounded px-1"
            :class="
              variant != Variant.COMPACT
                ? 'text-base text-gray-700 hover:bg-gray-100'
                : 'text-gray-400 hover:bg-gray-100 hover:text-gray-700'
            "
            @click="
              flow &&
                createStep(connection.tx, graph, {
                  step: { type: stepType },
                  parent: flow,
                  near: flowContext.centerVec,
                })
            "
          >
            <IconInline v-bind="ICON_BY_STEP_TYPE[stepType]" class="w-5 text-center" />
          </button>
          <!-- Reset Viewport (if compact) -->
          <button
            v-if="variant == Variant.COMPACT"
            class="rounded px-1 text-gray-400 hover:text-gray-700"
            @click="flowContext.resetViewport()"
          >
            <i class="fas fa-arrows-to-dot w-5 text-center" />
          </button>
        </div>
      </div>

      <!-- Selection -->
      <SelectionOverlay ref="selectionOverlayBodyRef" :zone="selectionZoneBody" />
    </div>
    <Inaccessible v-else :connection="connection" :node="nodePtr" class="h-full w-full" />

    <!-- Selection -->
    <SelectionOverlay ref="selectionOverlayBodyRef" :zone="selectionZoneContainer" />
  </div>
</template>
