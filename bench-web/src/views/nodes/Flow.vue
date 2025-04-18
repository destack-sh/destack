<script lang="ts" setup>
import { packSubnode } from "@/language/core/node";
import { makeType } from "@/language/core/type";
import { newChangeId } from "@/language/core/transaction";
import {
  ActionData,
  AnyNodeData,
  BenchType,
  ChangeCategory,
  TransitionData,
  NodeReferenceData,
  NodeType,
  PickerVariant,
  PortSide,
  TypeKind,
  ViewData,
  ViewType
} from "@/proto/wire";
import { isNode, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { useAutoConnection, type PreparedNodeConnection } from "@/system/connection";
import { canvas, spaceGraph } from "@/system/space";
import { ACTION_CONTEXT_COMMANDS, TRANSITION_CONTEXT_COMMANDS, type CommandMapKit } from "@/ui/command";
import { startSelectingIfAllowed, useSelectionZone } from "@/ui/drag";
import {
  ACTION_SIZE,
  ACTION_SIZE_HALF,
  FLOW_CANVAS_DOT_SIZE,
  FLOW_CONTEXT_KEY,
  FLOW_GRID_STEP,
  FlowContext,
  TRANSITION_WIDTH,
  TransitionPath,
  pathToSvg,
  SELF_TRANSITION_CONNECTION_DISTANCE,
} from "@/ui/flow";
import { PopoverInfoIn } from "@/ui/popover";
import { lengthVector2, subVector2, VIEW_DEFAULT_HEADER_HEIGHT } from "@/ui/view";
import Inaccessible from "@/views/builtin/Inaccessible.vue";
import InlineHeader from "@/views/builtin/InlineHeader.vue";
import NodeReference from "@/views/builtin/NodeReference.vue";
import { NavigationDirection, type FocusAnchor, type ViewEmits, type ViewExpose } from "@/views/common";
import Action from "@/views/nodes/Action.vue";
import Transition from "@/views/nodes/Transition.vue";
import SelectionOverlay from "@/views/overlays/SelectionOverlay.vue";
import { MaybeElement, useElementSize } from "@vueuse/core";
import { computed, provide, ref, toRef, type Ref } from "vue";

const BACKGROUND_STYLE: "checker" | "dots" = "dots";
const HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;
const GUTTER_WIDTH = 60;

const props = defineProps<
  {
    self?: TypedNodeReferenceData<NodeType.VIEW>;
    id: string;
    preparedConnection?: PreparedNodeConnection;
    isRoot?: boolean;
  } & Partial<Pick<ViewData, "icon" | "nodePtr" | "focusPtr" | "transform" | "isInline" | "isMinimal">>
>();
const emit = defineEmits<ViewEmits>();
const self = toRef(props, "self");
const id = toRef(props, "id");
const state = canvas.registerView(self, id);

// view
const containerRef = ref<HTMLElement | null>(null);
const headerRef: Ref<InstanceType<typeof InlineHeader> | null> = ref(null);
const bodyRef: Ref<HTMLElement | null> = ref(null);
const nameRef: Ref<InstanceType<typeof NodeReference> | null> = ref(null);
const createActionRef: Ref<HTMLButtonElement | null> = ref(null);
const actionRefs: Ref<Record<string, InstanceType<typeof Action>>> = ref({});
const transitionRefs: Ref<Record<string, InstanceType<typeof Transition>>> = ref({});
const containerSize = useElementSize(containerRef);
const headerSize = useElementSize(headerRef as Ref<MaybeElement>);

// flow
const nodePtr = computed(() => props.nodePtr as TypedNodeReferenceData<NodeType.FLOW>);
const preparedConnection = props.preparedConnection ?? useAutoConnection(nodePtr);
const { graph, connection } = preparedConnection;
const flowCtx = new FlowContext({
  spaceGraph: spaceGraph,
  spaceTx: () => canvas.tx().with({ category: ChangeCategory.SPACE }),
  graph: graph,
  tx: () => connection.tx,
  update: state.update,
  view: state.baseViewRef,
  transform: toRef(props, "transform"),
  containerRef: bodyRef,
  actionRefs: actionRefs,
  flowPtr: nodePtr,
});
provide(FLOW_CONTEXT_KEY, flowCtx);
const flow = flowCtx.flow;
const actions = flowCtx.actions;
const transitions = flowCtx.transitions;
const fields = flowCtx.fields;
const actionsAndTransitions: Ref<(ActionData | TransitionData)[]> = computed(() => [...actions.value, ...transitions.value]);

const viewport = flowCtx.viewport;

//
// Interaction
//

// pending
const pendingPath: Ref<TransitionPath | null> = computed(() => {
  // preview path between current dragged port and action (or point in canvas if nothing)
  if (flowCtx.draggable?.kind != "port") return null;
  const sourcePort = flowCtx.draggable;
  const sourceAction = sourcePort.action;
  const sourceBounding = flowCtx.getActionBoundingBox(sourceAction);
  if (sourceBounding == null) return null;
  const cursor = flowCtx.cursorWorldPos.value;
  const sourcePos = {
    x: sourceBounding.x1 + (flowCtx.dragging.value?.viewOffsetByThing[sourceAction.id]?.x ?? 0),
    y: sourceBounding.y1 + (flowCtx.dragging.value?.viewOffsetByThing[sourceAction.id]?.y ?? 0),
  };
  const targetAction = flowCtx.getActionAt(cursor);
  const distance = lengthVector2(subVector2(sourcePos, cursor));
  if (targetAction != null && targetAction.id === sourceAction.id && distance < SELF_TRANSITION_CONNECTION_DISTANCE) {
    return null; // ignore self-connections that are too close to starting point
  }

  // make path
  const isValid =
    targetAction != null &&
    flowCtx.canPortsConnect(
      { side: PortSide.OUTGOING, parent: sourceAction },
      { side: PortSide.INCOMING, parent: targetAction },
    ) === true;
  if (isValid) {
    // real path preview
    const target = flowCtx.getActionBoundingBox(targetAction);
    if (target == null) return null;
    return flowCtx.computePath(sourceBounding, target);
  } else {
    // just direct path, can't actually connect these
    const midpoint = { x: (sourcePos.x + cursor.x) / 2, y: (sourcePos.y + cursor.y) / 2 };
    return { start: sourcePos, end: cursor, midpoint };
  }
});

// selection
const selectionOverlayContainerRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionOverlayBodyRef = ref<InstanceType<typeof SelectionOverlay> | null>(null);
const selectionZoneContainer = useSelectionZone({ containerEl: containerRef, overlayEl: selectionOverlayContainerRef });
const selectionZoneBody = useSelectionZone({ containerEl: bodyRef, overlayEl: selectionOverlayBodyRef });

// actions
function moveNodes(nodes: AnyNodeData[], move: { x: number; y: number }) {
  const tx = flowCtx.tx.with({ change: { key: newChangeId(), title: "Move" } });
  for (const thing of nodes) {
    if (isNode(thing, NodeType.ACTION)) {
      flowCtx.move(thing, move, { tx });
    }
  }
}
function pushPopover(popover: PopoverInfoIn, e?: MouseEvent | KeyboardEvent) {
  canvas.pushPopover({
    generation: "new",
    trigger: (e?.target ?? bodyRef.value!) as HTMLElement,
    reference: e instanceof MouseEvent ? { x: e.clientX, y: e.clientY } : bodyRef.value!,
    placement: e instanceof MouseEvent ? undefined : "inside-top",
    ...popover,
  });
}
const implementedActions: Partial<CommandMapKit<"flow" | "space" | "runtime">> = {
  // edit
  "space.edit.rename": () => {
    nameRef.value?.focusIdentifier();
  },
  "flow.edit.createAction": (action, context) => {
    pushPopover(
      {
        kind: "view",
        component: ViewType.PICKER,
        placement: "inside-top",
        title: "Add Action",
        props: {
          valueType: makeType({ kind: TypeKind.ENUM, benchType: BenchType.ACTION_TYPE }),
          subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, { variant: PickerVariant.DROPDOWN_LARGE }),
        },
        onApply: (value) => {
          flowCtx.createAction({ parent: flow.value!, action: { type: value } });
        },
      },
      context.event,
    );
  },
  "flow.edit.splitTransition": (action, context) => {
    if (context.nodes?.length != 1) return;
    const transition = context.nodes[0];
    if (!isNode(transition, NodeType.TRANSITION)) return;
    const oldTarget = graph.getOrError(transition.targetPtr!) as ActionData;
    pushPopover(
      {
        kind: "view",
        component: ViewType.PICKER,
        placement: "bottom-right",
        title: "Add Action",
        props: {
          valueType: makeType({ kind: TypeKind.ENUM, benchType: BenchType.ACTION_TYPE }),
          subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, { variant: PickerVariant.DROPDOWN_LARGE }),
        },
        onApply: (value) => {
          const tx = flowCtx.tx.with({ change: { key: newChangeId(), title: "Split Transition" } });
          // find position
          const transitionMidpoint = flowCtx.transitionsStates.value[transition.id!].path.value?.midpoint;
          if (transitionMidpoint == null) throw new Error("no transition midpoint");
          const position = subVector2(transitionMidpoint, { x: ACTION_SIZE_HALF.width, y: ACTION_SIZE_HALF.height });
          // create new action
          const newAction = flowCtx.createAction({ parent: flow.value!, action: { type: value, position }, tx });
          // create new transition from new action to old transition target
          const newTransition = flowCtx.createTransition({
            parent: flow.value!,
            transition: {},
            source: { parent: newAction, side: PortSide.OUTGOING },
            target: { parent: oldTarget, side: PortSide.INCOMING },
            tx,
          });
          // reconnect old transition to new action
          tx.update(transition, { targetPtr: toNodeRef(newAction) }, { debounce: "long" });
          // and go to
          canvas.inspect({ node: newAction });
        },
      },
      context.event,
    );
  },
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
  "space.navigate.left": () => flowCtx.pan({ x: -FLOW_GRID_STEP, y: 0 }),
  "space.navigate.right": () => flowCtx.pan({ x: FLOW_GRID_STEP, y: 0 }),
  "space.navigate.up": () => flowCtx.pan({ x: 0, y: -FLOW_GRID_STEP }),
  "space.navigate.down": () => flowCtx.pan({ x: 0, y: FLOW_GRID_STEP }),
  "space.navigate.zoomIn": () => flowCtx.zoom("in", "center", 10),
  "space.navigate.zoomOut": () => flowCtx.zoom("out", "center", 10),
  "space.navigate.reset": () => flowCtx.resetViewport(),
  // select
  "space.select.all": () => canvas.select(actionsAndTransitions.value),
};

// focus
function focus(anchor?: FocusAnchor | NodeReferenceData) {
  if (typeof anchor == "object") {
    if (actionRefs.value[anchor.id!] != null) {
      const actionState = flowCtx.actionsStates.value[anchor.id!];
      if (
        actionState.action.value != null &&
        !flowCtx.isInViewport({ kind: "action", action: actionState.action.value })
      ) {
        flowCtx.panToCenter({ kind: "action", action: actionState.action.value! });
      }
      return actionRefs.value[anchor.id!].$el;
    } else if (transitionRefs.value[anchor.id!] != null) {
      const transitionState = flowCtx.transitionsStates.value[anchor.id!];
      if (transitionState.transition.value != null && !flowCtx.isInViewport({ kind: "transition", transition: transitionState.transition.value })) {
        flowCtx.panToCenter({ kind: "transition", transition: transitionState.transition.value! });
      }
      return transitionRefs.value[anchor.id!].$el;
    }
  } else {
    headerRef.value?.focus?.(anchor ?? "top");
    return true;
  }

  return false;
}

const isFocusAbsolute = canvas.isFocusedAbsoluteRef(self);
defineExpose<ViewExpose>({ self, id, commands: implementedActions, focus });
</script>
<template>
  <div
    ref="containerRef"
    :class="[isMinimal ? '' : 'h-full']"
    data-contextmenu-items="flow.edit.create*"
    @mousedown="(e) => startSelectingIfAllowed(selectionZoneContainer, e)"
  >
    <!-- Header -->
    <InlineHeader
      ref="headerRef"
      :self="self"
      :node="flow"
      :node-ptr="nodePtr"
      :prepared-connection="preparedConnection"
      :is-root="isRoot"
      :is-inline="isInline"
      :is-minimal="isMinimal"
      :focus-ptr="focusPtr"
      :width="containerSize.width.value - GUTTER_WIDTH * 2"
      @navigate="(direction: NavigationDirection) => emit('navigate', direction)"
    >
      <!-- Signature -->
      <template #left="{ style }">
        <!-- Identity/Roles -->
        <!-- ... -->
      </template>
      <!-- Meta -->
      <template #right="{ style }">
        <!-- Add action -->
        <button
          ref="createActionRef"
          class="group/button rounded-sm px-1 py-0.5 text-gray-400 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-700"
          :class="style == 'block' ? 'opacity-0 group-hover/block:opacity-100 group-hover/header:opacity-100' : ''"
          @click="
            (e) =>
              canvas.pushPopover({
                kind: 'view',
                trigger: e.target as HTMLElement,
                reference: e.target as HTMLElement,
                component: ViewType.PICKER,
                title: 'Add Action',
                placement: 'bottom-left',
                offset: 'referenceWidth',
                props: {
                  valueType: makeType({ kind: TypeKind.ENUM, benchType: BenchType.ACTION_TYPE }),
                  subnodePacked: packSubnode(NodeType.VIEW, ViewType.PICKER, {
                    variant: PickerVariant.DROPDOWN_LARGE,
                  }),
                },
                onApply: (value) => {
                  flowCtx.createAction({ parent: flow!, action: { type: value } });
                },
              })
          "
        >
          <i class="fas fa-plus mr-1.5 text-center" />
          <span class="">Action</span>
        </button>
        <div v-if="style == 'page'" class="flex flex-row items-center gap-x-0.5">
          <!-- Zoom -->
          <button
            class="rounded-sm py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="() => flowCtx.zoom('out', flowCtx.centerVec!, 10)"
          >
            <i class="fas fa-minus w-5 text-center" />
          </button>
          <button
            class="rounded-sm py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="flowCtx.zoom(1.0, flowCtx.centerVec!, 1)"
          >
            {{ Math.round(viewport.scale * 100) }}%
          </button>
          <button
            class="rounded-sm py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="flowCtx.zoom('in', flowCtx.centerVec!, 10)"
          >
            <i class="fas fa-plus w-5 text-center" />
          </button>
          <!-- Auto/Reset -->
          <button
            class="rounded-sm px-0.5 py-0.5 text-gray-400 hover:bg-gray-100 hover:text-gray-700"
            @click="flowCtx.resetViewport()"
          >
            <i class="fas fa-arrows-to-dot w-5 text-center" />
          </button>
        </div>
      </template>
    </InlineHeader>

    <!-- Canvas -->
    <div
      v-if="flow"
      ref="bodyRef"
      class="group/flow relative w-full select-none"
      :class="[flowCtx.dragging.value ? (flowCtx.isDraggingPort ? 'cursor-crosshair' : 'cursor-grabbing') : '']"
      :style="{
        height: `calc(100% - ${headerSize.height.value}px)`,
      }"
      @mousedown="(e) => startSelectingIfAllowed(selectionZoneBody, e)"
      @mousemove="(e) => flowCtx.onDragging(e)"
      @mouseleave="flowCtx.cancelDragging()"
      @mouseup="(e) => flowCtx.endDragging(e, { kind: 'canvas' })"
      @wheel="(e) => (!isMinimal ? flowCtx.onWheel(e) : undefined)"
      @keydown.esc="flowCtx.cancelDragging()"
    >
      <!-- Background grid (infinitely repeated) -->
      <div class="absolute h-full w-full overflow-hidden" :style="{}">
        <svg
          v-if="BACKGROUND_STYLE == 'dots'"
          xmlns="http://www.w3.org/2000/svg"
          class="h-full w-full text-gray-200"
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
          :style="{
            transformOrigin: '0 0',
            transform: `scale(${viewport.scale}, ${viewport.scale}) translate(${viewport.transform.translateX}px, ${viewport.transform.translateY}px) `,
          }"
        >
          <!-- Transitions -->
          <Transition
            v-for="transition in transitions"
            :id="transition.id"
            :ref="(ref: any) => (ref != null ? (transitionRefs[transition.id] = ref) : delete transitionRefs[transition.id])"
            :key="transition.id"
            :prepared-connection="preparedConnection"
            :node-ptr="toNodeRef(transition)"
            :data-contextmenu-items="TRANSITION_CONTEXT_COMMANDS.join(',')"
            class="absolute"
          />
          <!-- Commands -->
          <Action
            v-for="action in actions"
            :id="action.id"
            :ref="(ref: any) => (ref ? (actionRefs[action.id] = ref) : delete actionRefs[action.id])"
            :key="action.id"
            class="absolute"
            :class="[flowCtx?.isDraggingAction(action) ? 'cursor-grabbing' : flowCtx.isDragging ? '' : 'cursor-grab']"
            :style="{
              width: ACTION_SIZE.width + 'px',
              height: ACTION_SIZE.height + 'px',
              left: (action.position?.x ?? 0) + 'px',
              top: (action.position?.y ?? 0) + 'px',
            }"
            :prepared-connection="preparedConnection"
            :node-ptr="toNodeRef(action)"
            :data-contextmenu-items="ACTION_CONTEXT_COMMANDS.join(',')"
            data-suppress-drag="select"
            @mousedown="(e: MouseEvent) => flowCtx.startDraggingIfAllowed(e, { kind: 'action', action: action! })"
          />
          <!-- Pending Transition (above Actions for clarity)-->
          <div v-if="flowCtx.draggable?.kind == 'port'" class="pointer-events-none absolute text-gray-700 opacity-50">
            <svg v-if="pendingPath" class="overflow-visible">
              <path
                :stroke-width="TRANSITION_WIDTH * 2"
                stroke-linecap="round"
                stroke-linejoin="bevel"
                stroke="currentColor"
                fill="none"
                :d="pathToSvg(pendingPath)"
              />
            </svg>
          </div>
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
