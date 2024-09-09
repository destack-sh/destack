import { estimateCodeHeight } from "@/language/code";
import { BOUNDARY_STEP_TYPES, INCOMING_STEP_TYPES, OUTGOING_STEP_TYPES } from "@/language/const";
import type { ReadNodeGraph } from "@/language/graph";
import { makeNodeName } from "@/language/node";
import { estimateTextHeight } from "@/language/text";
import type { Transaction } from "@/language/transaction";
import {
  BlockData,
  BlockType,
  ColorShade,
  FieldData,
  FieldZone,
  NodeType,
  ObjectType,
  PipeData,
  PipeType,
  PortKeyData,
  PortSide,
  PortType,
  StepType,
  StructType,
  TransformData,
  Vector2Data,
  ViewData,
  type AnyNodeData,
  type StepData,
} from "@/proto/wire";
import { describeNode, isNode, makeStruct, toPlainNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import type { ActionBuiltinId } from "@/ui/action";
import { isDraggingAllowed } from "@/ui/drag";
import { getColorHex } from "@/ui/style";
import { toaster } from "@/ui/toast";
import { addTransform, addVector2, VIEW_DEFAULT_HEADER_HEIGHT, type Vector2 } from "@/ui/view";
import { generateOrderKey } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";
import { log } from "@/utils/log";
import { computedValue } from "@/utils/ref";
import type Step from "@/views/system/Step.vue";
import { computed, inject, ref, shallowRef, triggerRef, watch, type Ref } from "vue";

export const STEP_CONTEXT_ACTIONS: ActionBuiltinId[] = [
  "common.edit.rename",
  "common.edit.morph",
  "common.edit.duplicate",
  "common.edit.archive",
  "common.edit.delete",
  "session.run.start",
  "message.chat.message",
];
export const PIPE_CONTEXT_ACTIONS: ActionBuiltinId[] = [
  "common.edit.rename",
  "common.edit.morph",
  "common.edit.duplicate",
  "common.edit.archive",
  "common.edit.delete",
];

export const FLOW_GRID_STEP = 28;
export const FLOW_PORT_SIZE = 12;

export const FLOW_CANVAS_DOT_SIZE = 4;
export const FLOW_SCALE_MIN = 0.5;
export const FLOW_SCALE_MAX = 4.0;
export const FLOW_SCALE_SPEED = 0.01;

export const PIPE_WIDTH = 4;
export const STEP_HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

/** Rounds the given vector to the nearest grid position (in world coordinates). */
export function snapVec(vec: { x: number; y: number }): { x: number; y: number } {
  return {
    x: Math.round(vec.x / FLOW_GRID_STEP) * FLOW_GRID_STEP,
    y: Math.round(vec.y / FLOW_GRID_STEP) * FLOW_GRID_STEP,
  };
}
export function snapScalar(x: number): number {
  return Math.round(x / FLOW_GRID_STEP) * FLOW_GRID_STEP;
}

export type PortId = Pick<PortKeyData, "type" | "side"> & Partial<Pick<PortKeyData, "fieldPtr">>;
export type Port = PortId & {
  parent: StepData | PipeData;
  idx: number;
  field?: FieldData;
  fieldParent?: BlockData | StepData; // if different from step
};

export function portIdEquals(a: PortId, b: PortId): boolean {
  return a.type == b.type && a.side == b.side && a.fieldPtr?.id == b.fieldPtr?.id;
}

export function getPortKey(port: PortId): PortKeyData {
  return {
    metatype: ObjectType.PORT_KEY,
    type: port.type,
    side: port.side,
    fieldPtr: port.fieldPtr,
  };
}

export type PipePath = {
  points: Vector2[];
};

export type BoundingBox = { x1: number; y1: number; x2: number; y2: number; width: number; height: number };

export type FlowThing =
  | { kind: "canvas" }
  | { kind: "step"; step: StepData }
  | { kind: "pipe"; pipe: PipeData }
  | { kind: "step-port"; step: StepData; port: Port; cursorWorldPos?: { x: number; y: number } };
// | { // TODO :UX: reconnect pipes (support pipe-port)
//     kind: "pipe-port";
//     pipe: PipeData;
//     port: Port;
//   };

/** Step state in a Flow. */
export class StepState {
  // self
  flow: FlowContext;
  stepPtr: TypedNodeReferenceData<NodeType.STEP>;
  step: Ref<StepData | null>;
  fields: Ref<FieldData[]>;
  nodePtr: Ref<TypedNodeReferenceData<NodeType.BLOCK | NodeType.STEP> | null>;
  node: Ref<BlockData | StepData | null>;
  nodeFields: Ref<FieldData[]>;
  // derived
  ports: Ref<{ incoming: Port[]; outgoing: Port[] }>;
  // layout
  boundingBox: Ref<BoundingBox | null> = shallowRef(null);

  constructor(flow: FlowContext, step: StepData) {
    this.flow = flow;
    this.stepPtr = toPlainNodeRef(step);
    this.step = flow.graph.getRef(this.stepPtr, { ignoreAncestors: true });
    this.nodePtr = computedValue(
      () => this.step.value?.nodePtr as TypedNodeReferenceData<NodeType.BLOCK | NodeType.STEP> | null,
    );
    this.node = flow.graph.getRef(this.nodePtr);
    this.nodeFields = flow.graph.getChildrenRef(this.nodePtr, NodeType.FIELD);
    this.fields = flow.graph.getChildrenRef(step, NodeType.FIELD);
    // derived
    this.ports = computed(() =>
      this.step.value != null ? this.flow.getPorts(this.step.value) : { incoming: [], outgoing: [] },
    );
    // layout
    this.boundingBox = computedValue(() =>
      this.step.value != null ? this.flow.getStepBoundingBox(this.step.value) : null,
    );
  }
}

/** Pipe state in a Flow. */
export class PipeState {
  // self
  flow: FlowContext;
  pipePtr: TypedNodeReferenceData<NodeType.PIPE>;
  pipe: Ref<PipeData | null>;
  source: Ref<StepData | null>;
  target: Ref<StepData | null>;
  // derived
  sourcePort: Ref<Port | null>;
  targetPort: Ref<Port | null>;
  // layout
  path: Ref<PipePath | null>;
  boundingBox: Ref<BoundingBox | null>;

  constructor(flow: FlowContext, pipe: PipeData) {
    this.flow = flow;
    this.pipePtr = toPlainNodeRef(pipe);
    this.pipe = flow.graph.getRef(this.pipePtr);
    this.source = flow.graph.getRef(
      computed(() => this.pipe.value?.sourcePtr as TypedNodeReferenceData<NodeType.STEP> | null),
    );
    this.target = flow.graph.getRef(
      computed(() => this.pipe.value?.targetPtr as TypedNodeReferenceData<NodeType.STEP> | null),
    );

    // derived
    this.sourcePort = computed(() => {
      if (this.pipe.value?.sourcePort == null || this.source.value == null) return null;
      const sourceStepState = this.flow.stepsStates.value[this.source.value.id!];
      const port = sourceStepState?.ports.value.outgoing.find((p) => portIdEquals(p, this.pipe.value!.sourcePort!));
      return port ?? null;
    });
    this.targetPort = computed(() => {
      if (this.pipe.value?.targetPort == null || this.target.value == null) return null;
      const targetStepState = this.flow.stepsStates.value[this.target.value.id!];
      const port = targetStepState?.ports.value.incoming.find((p) => portIdEquals(p, this.pipe.value!.targetPort!));
      return port ?? null;
    });

    // layout
    this.path = computed(() => {
      if (this.sourcePort.value == null || this.targetPort.value == null) return null;
      const sourcePortPosition = this.flow.getPortPosition(this.source.value!, this.sourcePort.value);
      const targetPortPosition = this.flow.getPortPosition(this.target.value!, this.targetPort.value);
      if (sourcePortPosition == null || targetPortPosition == null) return null;
      const path = this.flow.computePath(
        sourcePortPosition,
        this.sourcePort.value.side,
        targetPortPosition,
        this.targetPort.value.side,
      );
      return path;
    });
    this.boundingBox = computedValue(() => {
      if (this.path.value == null) return null;
      return this.flow.computePathBoundingBox(this.path.value);
    });
  }
}

/** An entire flow canvas (including steps, sub-steps, pipes, etc.) */
export class FlowContext {
  spaceGraph: ReadNodeGraph;
  graph: ReadNodeGraph;
  private spaceTxFactory: () => Transaction;
  private txFactory: () => Transaction;

  view: Ref<ViewData | null>;
  stepRefs: Ref<Record<string, InstanceType<typeof Step>>>;
  containerRef: Ref<HTMLElement | null>;
  dragging: Ref<{ thing: FlowThing; viewOffsetToThing: { x: number; y: number } } | null> = ref(null);

  flowPtr: Ref<TypedNodeReferenceData<NodeType.BLOCK> | null>;
  flow: Ref<BlockData | null>;
  fields: Ref<FieldData[]>;
  transform: Ref<TransformData>;
  scale: Ref<number>;
  steps: Ref<StepData[]>;
  pipes: Ref<PipeData[]>;

  stepsStates: Ref<Record<string, StepState>> = shallowRef({});
  pipesStates: Ref<Record<string, PipeState>> = shallowRef({});
  contentBoundingBox: Ref<BoundingBox | null>;

  constructor(context: {
    spaceGraph: ReadNodeGraph;
    spaceTx: () => Transaction;
    graph: ReadNodeGraph;
    tx: () => Transaction;
    view: Ref<ViewData | null>;
    containerRef: Ref<HTMLElement | null>;
    stepRefs: Ref<Record<string, InstanceType<typeof Step>>>;
    flowPtr: Ref<TypedNodeReferenceData<NodeType.BLOCK>>;
  }) {
    this.spaceGraph = context.spaceGraph;
    this.spaceTxFactory = context.spaceTx;
    this.graph = context.graph;
    this.txFactory = context.tx;

    // view
    this.view = context.view;
    this.containerRef = context.containerRef;
    this.stepRefs = context.stepRefs;

    // flow
    this.flowPtr = context.flowPtr;
    this.flow = this.graph.getRef(context.flowPtr);
    this.fields = this.graph.getChildrenRef(this.flow, NodeType.FIELD);
    this.transform = computed(() => this.view.value?.transform ?? makeStruct({ metatype: StructType.TRANSFORM }));
    this.scale = computed(() => this.transform.value.scaleX ?? this.transform.value.scaleY ?? 1);
    this.steps = this.graph.getChildrenRef(this.flow, NodeType.STEP);
    this.pipes = this.graph.getChildrenRef(this.flow, NodeType.PIPE);

    // maintain step/pipe contexts
    watch(
      this.steps,
      () => {
        const stepsIds = this.steps.value.map((s) => s.id);
        this.steps.value
          .filter((step) => this.stepsStates.value[step.id] == null)
          .forEach(
            (step) => ((this.stepsStates.value[step.id] = new StepState(this, step)), triggerRef(this.stepsStates)),
          );
        Object.keys(this.stepsStates.value)
          .filter((stepId) => !stepsIds.includes(stepId))
          .forEach((stepId) => (delete this.stepsStates.value[stepId], triggerRef(this.stepsStates)));
      },
      { immediate: true },
    );
    watch(
      this.pipes,
      () => {
        const pipesIds = this.pipes.value.map((p) => p.id);
        this.pipes.value
          .filter((pipe) => this.pipesStates.value[pipe.id] == null)
          .forEach(
            (pipe) => ((this.pipesStates.value[pipe.id] = new PipeState(this, pipe)), triggerRef(this.pipesStates)),
          );
        Object.keys(this.pipesStates.value)
          .filter((pipeId) => !pipesIds.includes(pipeId))
          .forEach((pipeId) => (delete this.pipesStates.value[pipeId], triggerRef(this.pipesStates)));
      },
      { immediate: true },
    );

    // layout
    this.contentBoundingBox = computedValue(() => this.computeContentBoundingBox());
  }

  get tx() {
    return this.txFactory();
  }

  get spaceTx() {
    return this.spaceTxFactory();
  }

  get draggable(): FlowThing | null {
    return this.dragging.value?.thing ?? null;
  }

  get isDraggingPort(): boolean {
    return this.dragging.value?.thing.kind == "step-port";
  }

  getStepComponent(step: StepData): InstanceType<typeof Step> | null {
    const stepRef = this.stepRefs.value[step.id!];
    return stepRef != null ? stepRef : null;
  }

  /** Gets the (reactive) estimated boundng box for a Step (in world coordinates). */
  getStepBoundingBox(step: StepData): BoundingBox | null {
    const stepState = this.stepsStates.value[step.id!];
    if (stepState == null) return null;
    const verticalPorts = Math.max(stepState.ports.value.incoming.length, stepState.ports.value.outgoing.length);
    const size = estimateStepSize(step, verticalPorts);
    const x1 = step.position?.x ?? 0;
    const y1 = step.position?.y ?? 0;
    const x2 = x1 + size.width;
    const y2 = y1 + size.height;
    return { x1, y1, x2, y2, width: x2 - x1, height: y2 - y1 };
  }

  //
  // Canvas
  //

  /** Gets the current center of the canvas in world coordinates. */
  get centerVec(): Vector2 | null {
    if (this.containerRef.value == null) return null;
    const containerBounding = this.containerRef.value.getBoundingClientRect();
    const centerVec = this.viewToWorldVec({ x: containerBounding.width / 2, y: containerBounding.height / 2 });
    return centerVec;
  }

  /** Computes the bounding box for the viewport (in world coordinates). */
  computeViewportBoundingBox(): BoundingBox | null {
    if (this.containerRef.value == null) return null;
    const containerBounding = this.containerRef.value.getBoundingClientRect();
    const vec1 = this.viewToWorldVec({ x: 0, y: 0 });
    const vec2 = this.viewToWorldVec({ x: containerBounding.width, y: containerBounding.height });
    return {
      x1: Math.min(vec1.x, vec2.x),
      y1: Math.min(vec1.y, vec2.y),
      x2: Math.max(vec1.x, vec2.x),
      y2: Math.max(vec1.y, vec2.y),
      width: vec2.x - vec1.x,
      height: vec2.y - vec1.y,
    };
  }

  /** Computes the bounding box for all things in this flow (in world coordinates). */
  computeContentBoundingBox(): BoundingBox | null {
    const steps = this.steps.value;
    if (steps.length == 0) return null;
    let x1 = steps[0].position?.x ?? 0;
    let y1 = steps[0].position?.y ?? 0;
    let x2 = steps[0].position?.x ?? 0;
    let y2 = steps[0].position?.y ?? 0;
    for (const step of steps) {
      const state = this.stepsStates.value[step.id!];
      if (state == null) continue;
      const numPorts = Math.max(state.ports.value.incoming.length, state.ports.value.outgoing.length);
      const size = estimateStepSize(step, numPorts);
      x1 = Math.min(x1, step.position?.x ?? 0);
      y1 = Math.min(y1, step.position?.y ?? 0);
      x2 = Math.max(x2, (step.position?.x ?? 0) + size.width);
      y2 = Math.max(y2, (step.position?.y ?? 0) + size.height);
    }
    return { x1, y1, x2, y2, width: x2 - x1, height: y2 - y1 };
  }

  /** Gets the bounding box for a thing (in world coordinates). */
  getBoundingBox(thing: FlowThing): BoundingBox | null {
    if (thing.kind == "canvas") {
      return this.contentBoundingBox.value;
    } else if (thing.kind == "step") {
      return this.stepsStates.value[thing.step.id!]?.boundingBox.value;
    } else if (thing.kind == "step-port") {
      const position = this.getPortPosition(thing.step, thing.port);
      if (position == null) return null;
      return this.getPortBoundingBox(position);
    } else if (thing.kind == "pipe") {
      const pipeState = this.pipesStates.value[thing.pipe.id!];
      if (pipeState == null) return null;
      return pipeState.boundingBox.value;
    } else {
      assertNever(thing);
    }
  }

  isInViewport(thing: FlowThing): boolean {
    if (thing.kind == "canvas") return true;
    const thingBounding = this.getBoundingBox(thing);
    const canvasBounding = this.computeViewportBoundingBox();
    if (thingBounding == null || canvasBounding == null) return false;
    return (
      thingBounding.x1 >= canvasBounding.x1 &&
      thingBounding.x2 <= canvasBounding.x2 &&
      thingBounding.y1 >= canvasBounding.y1 &&
      thingBounding.y2 <= canvasBounding.y2
    );
  }

  /** Pan the canvas to center something (in world coordinates). */
  panToCenter(thing: FlowThing) {
    const boundingBox = this.getBoundingBox(thing);
    if (boundingBox == null) throw new Error(`no bounding box for ${thing.kind}`);
    const currentCenter = this.centerVec;
    if (currentCenter == null) throw new Error("no current center");
    this.pan({
      x: currentCenter.x - boundingBox.x1 - boundingBox.width / 2,
      y: currentCenter.y - boundingBox.y1 - boundingBox.height / 2,
    });
  }

  /** Pan the canvas (in world coordinates). */
  pan(move: { x: number; y: number }) {
    if (this.view.value == null) return; // not a real view
    this.spaceTx.update(
      this.view.value,
      { transform: addTransform(this.transform.value, { translateX: move.x, translateY: move.y }) },
      { debounce: "long" },
    );
  }

  /** Convert viewport coordinates to view coordinates */
  viewportToViewVec(viewportVec: { x: number; y: number }): { x: number; y: number } {
    const canvasBounding = this.containerRef.value?.getBoundingClientRect();
    if (canvasBounding == null) throw new Error("no canvas bounding");
    return {
      x: viewportVec.x - canvasBounding.left,
      y: viewportVec.y - canvasBounding.top,
    };
  }

  /** Convert view coordinates to viewport coordinates */
  viewToViewportVec(viewVec: { x: number; y: number }): { x: number; y: number } {
    const canvasBounding = this.containerRef.value?.getBoundingClientRect();
    if (canvasBounding == null) throw new Error("no canvas bounding");
    return {
      x: viewVec.x + canvasBounding.left,
      y: viewVec.y + canvasBounding.top,
    };
  }

  /** Convert viewport coordinates to view coordinates */
  viewportToWorldVec(viewportVec: { x: number; y: number }): { x: number; y: number } {
    const viewVec = this.viewportToViewVec(viewportVec);
    return this.viewToWorldVec(viewVec);
  }

  /** Convert world coordinates to view coordinates */
  worldToViewVec(worldVec: { x: number; y: number }): { x: number; y: number } {
    let viewVec = {
      x: worldVec.x + (this.transform.value.translateX ?? 0),
      y: worldVec.y + (this.transform.value.translateY ?? 0),
    };
    viewVec = { x: viewVec.x * this.scale.value, y: viewVec.y * this.scale.value };
    return viewVec;
  }

  /** Convert world coordinates to viewport coordinates */
  worldToViewportVec(worldVec: { x: number; y: number }): { x: number; y: number } {
    const viewVec = this.worldToViewVec(worldVec);
    return this.viewToViewportVec(viewVec);
  }

  /** Convert screen coordinates to world coordinates */
  viewToWorldVec(viewVec: { x: number; y: number }): { x: number; y: number } {
    let worldVec = { x: viewVec.x / this.scale.value, y: viewVec.y / this.scale.value };
    worldVec = {
      x: worldVec.x - (this.transform.value.translateX ?? 0),
      y: worldVec.y - (this.transform.value.translateY ?? 0),
    };
    return worldVec;
  }

  /** Zoom the convas around the given origin (panning as needed) */
  zoom(direction: "in" | "out" | number, originViewVec: { x: number; y: number } | "center", steps: number) {
    if (this.view.value == null) return; // not a real view
    const containerBounding = this.containerRef.value?.getBoundingClientRect();
    if (containerBounding == null) throw new Error("no container bounding");
    if (originViewVec == "center") {
      originViewVec = { x: containerBounding.width / 2, y: containerBounding.height / 2 };
    }
    // figure out new zoom
    const currentZoom = this.scale.value;
    let newZoom: number;
    if (direction == "in") {
      newZoom = Math.min(FLOW_SCALE_MAX, currentZoom + FLOW_SCALE_SPEED * steps);
    } else if (direction == "out") {
      newZoom = Math.max(FLOW_SCALE_MIN, currentZoom - FLOW_SCALE_SPEED * steps);
    } else {
      newZoom = direction;
    }
    if (newZoom == currentZoom) return; // no change
    const translateX = this.transform.value?.translateX ?? 0;
    const translateY = this.transform.value?.translateY ?? 0;

    // pan to keep the origin
    // (the viewport scales with (0, 0) at the origin, but we want the center of the viewport to stay in the same place)
    const currentCenterWorldVec = this.viewToWorldVec({
      x: containerBounding.width / 2,
      y: containerBounding.height / 2,
    });
    const newCenterWorldVec = {
      x: containerBounding.width / 2 / newZoom - translateX,
      y: containerBounding.height / 2 / newZoom - translateY,
    };
    const panVec = {
      x: newCenterWorldVec.x - currentCenterWorldVec.x,
      y: newCenterWorldVec.y - currentCenterWorldVec.y,
    };

    // pan to move towards the origin a bit
    const newOriginWorldVec = {
      x: originViewVec.x / newZoom - translateX,
      y: originViewVec.y / newZoom - translateY,
    };
    panVec.x += (newCenterWorldVec.x - newOriginWorldVec.x) * (newZoom - currentZoom);
    panVec.y += (newCenterWorldVec.y - newOriginWorldVec.y) * (newZoom - currentZoom);

    this.spaceTx.update(
      this.view.value,
      {
        transform: {
          ...this.transform.value,
          scaleX: newZoom,
          scaleY: newZoom,
          translateX: translateX + panVec.x,
          translateY: translateY + panVec.y,
        },
      },
      { debounce: "long" },
    );
  }

  /** Resets the viewport to the 'center' of the canvas */
  resetViewport() {
    if (this.view.value == null) return; // not a real view
    this.zoom(1, "center", 0);
    this.panToCenter({ kind: "canvas" });
  }

  /** Zooms the canvas in/out in response to a "wheel" event. */
  onWheel(e: WheelEvent) {
    const viewCenterVec = this.viewportToViewVec({ x: e.clientX, y: e.clientY });
    this.zoom(e.deltaY < 0 ? "in" : "out", viewCenterVec, Math.abs(e.deltaY * 0.5));
  }

  /** Starts dragging a thing if it's not a disallowed element (like an input). */
  startDraggingIfAllowed(e: MouseEvent, thing: FlowThing): boolean {
    const target = e.target as HTMLElement;
    if (!isDraggingAllowed(target)) return false;
    return this.startDragging(e, thing);
  }

  /** Starts dragging a thing. */
  startDragging(e: MouseEvent, thing: FlowThing): boolean {
    if (this.dragging.value != null) return false; // already dragging
    if (thing.kind == "canvas") {
      // start panning canvas
      this.dragging.value = {
        thing,
        viewOffsetToThing: this.viewportToViewVec({ x: e.clientX, y: e.clientY }),
      };
    } else if (thing.kind == "step") {
      // start moving step
      const positionViewportVec = this.worldToViewportVec({
        x: thing.step.position?.x ?? 0,
        y: thing.step.position?.y ?? 0,
      });
      this.dragging.value = {
        thing,
        viewOffsetToThing: { x: e.clientX - positionViewportVec.x, y: e.clientY - positionViewportVec.y },
      };
    } else if (thing.kind == "step-port") {
      // create pending pipe
      this.dragging.value = {
        thing: { ...thing, cursorWorldPos: this.viewportToWorldVec({ x: e.clientX, y: e.clientY }) },
        viewOffsetToThing: { x: 0, y: 0 },
      };
    } else if (thing.kind == "pipe") {
      throw new Error("cannot drag pipe");
    } else {
      assertNever(thing);
    }
    log.trace("flow.drag.start", this.dragging.value);
    return true;
  }

  /** Updates the position of a dragged thing in response to a "drag" event. */
  onDragging(e: MouseEvent) {
    if (this.dragging.value == null) return;
    if (this.view.value == null) return; // not a real view
    const thing = this.dragging.value.thing;
    if (thing.kind == "canvas") {
      // pan canvas
      const translateX = e.movementX / (this.transform.value?.scaleX ?? 1);
      const translateY = e.movementY / (this.transform.value?.scaleY ?? 1);
      this.spaceTx.update(
        this.view.value,
        { transform: addTransform(this.transform.value, { translateX, translateY }) },
        { debounce: "long" },
      );
    } else if (thing.kind == "step") {
      // move step (snap to grid)
      const screenVec = this.viewportToViewVec({
        x: e.clientX - this.dragging.value.viewOffsetToThing.x,
        y: e.clientY - this.dragging.value.viewOffsetToThing.y,
      });
      const worldVec = snapVec(this.viewToWorldVec(screenVec));
      this.tx.update(
        thing.step,
        { position: makeStruct({ metatype: StructType.VECTOR2, x: worldVec.x, y: worldVec.y }) },
        { debounce: "long" },
      );
    } else if (thing.kind == "step-port") {
      // update cursor position
      thing.cursorWorldPos = this.viewportToWorldVec({ x: e.clientX, y: e.clientY });
    } else if (thing.kind == "pipe") {
      throw new Error("cannot drag pipe");
    } else {
      assertNever(thing);
    }
  }

  /** Cancel dragging (and don't trigger any release events). */
  cancelDragging() {
    if (this.dragging.value == null) return;
    this.dragging.value = null;
    log.trace("flow.drag.cancel", this.dragging.value);
  }

  /** Stop dragging a thing (if any). Triggers a 'release' event to connect things. */
  endDragging(e: MouseEvent, at: FlowThing) {
    if (this.flow.value == null) throw new Error("no flow to connect");
    if (this.dragging.value == null) return;

    // (re-)connect ports
    try {
      if (this.draggable?.kind == "step-port" && at.kind == "step-port") {
        if (portIdEquals(this.draggable.port, at.port)) return; // no-op
        log.info("flow.drag.connect", { from: this.draggable.port, to: at.port });
        createPipe(this.tx, this.graph, {
          parent: this.flow.value,
          pipe: { type: PipeType.THEN },
          source: this.draggable.port,
          target: at.port,
        });
      }
    } catch (e) {
      toaster.error({ title: "Invalid Pipe", text: (e as any)?.message ?? "Cannot pipe like that." });
      log.error("flow.drag.connect.error", this.draggable, at, e);
    }

    log.trace("flow.drag.end", { from: this.dragging.value, to: at });
    this.dragging.value = null;
  }

  /** Get the snapped position of a port (in world coordinates). */
  getPortPosition(step: StepData, port: Port): Vector2 | null {
    // step position
    const basePosition = step.position != null ? { ...step.position } : { x: 0, y: 0 };
    // move to side
    if (port.side == PortSide.OUTGOING) {
      basePosition.x += getStepWidth(step);
    }
    // move down below header :FlowGrid
    basePosition.y += STEP_HEADER_HEIGHT + FLOW_GRID_STEP - (STEP_HEADER_HEIGHT % FLOW_GRID_STEP);
    // move down to port
    basePosition.y += port.idx * FLOW_GRID_STEP;
    return basePosition;
  }

  /** Gets the bounding box for a port (in world coordinates). */
  getPortBoundingBox(position: Vector2): BoundingBox | null {
    return {
      x1: position.x - FLOW_PORT_SIZE / 2,
      y1: position.y - FLOW_PORT_SIZE / 2,
      x2: position.x + FLOW_PORT_SIZE / 2,
      y2: position.y + FLOW_PORT_SIZE / 2,
      width: FLOW_PORT_SIZE,
      height: FLOW_PORT_SIZE,
    };
  }

  /**
   * Computes the manhattan path for a pipe (in world coordinates, without considering other pipes).
   * Pathfinding has a few key objectives:
   *  0. We start at the source port and want to reach the target port (if we can't, return null).
   *  1. Pipes are always exactly on the grid; ports are always connected horizontally.
   *  2. Unless directly at a port, we must stay at least one step away from any step.
   *  3. Pipes must be straight and should look reasonably clean, so try to break at halfway points.
   *  4. Path computation must be very fast (we're doing it on every mouse move and state change).
   * NOTE :UX: improve pipe paths (better pathfinding, coordinate pipe paths, ...)
   * */
  computePath(source: Vector2, sourceSide: PortSide, target: Vector2, targetSide: PortSide): PipePath | null {
    // swap it so that source is always outgoing
    if (sourceSide != PortSide.OUTGOING) {
      [source, target] = [target, source];
      [sourceSide, targetSide] = [targetSide, sourceSide];
    }

    // 'collision' detection
    const stepBoundingBoxes: BoundingBox[] = Object.values(this.stepsStates.value)
      .map((s) => s.boundingBox.value)
      .filter((s) => s != null) as BoundingBox[];
    function hitStep(vec: { x: number; y: number }): BoundingBox | undefined {
      for (const box of stepBoundingBoxes) {
        if (box.x1 <= vec.x && vec.x <= box.x2 && box.y1 <= vec.y && vec.y <= box.y2) {
          return box;
        }
      }
      return undefined;
    }

    // pathfind between point right next to port
    source = snapVec(source);
    target = snapVec(target);
    const innerPoints = pathfind(
      { x: source.x + FLOW_GRID_STEP, y: source.y },
      { x: target.x - FLOW_GRID_STEP, y: target.y },
      { step: FLOW_GRID_STEP, maxIterations: 1000, hit: hitStep },
    );
    if (innerPoints == null) return null; // no path found
    const points = [source, ...innerPoints, target]; // add source/target port back in

    const path: PipePath = { points };
    return path;
  }

  /** Gets the bounding box for a pipe path (in world coordinates). */
  computePathBoundingBox(path: PipePath): BoundingBox | null {
    const points = path.points;
    let x1 = points[0].x;
    let y1 = points[0].y;
    let x2 = points[0].x;
    let y2 = points[0].y;
    for (let i = 1; i < points.length; i++) {
      const p = points[i];
      if (p.x < x1) x1 = p.x;
      else if (p.x > x2) x2 = p.x;
      if (p.y < y1) y1 = p.y;
      else if (p.y > y2) y2 = p.y;
    }
    return { x1, y1, x2, y2, width: x2 - x1, height: y2 - y1 };
  }

  /** Gets the pipes connected to the given port. */
  getPipesAtPort(port: Port): PipeData[] {
    return this.pipes.value.filter((pipe) => {
      return (
        (port.parent?.ck == pipe.sourcePtr?.ck && portIdEquals(port, pipe.sourcePort!)) ||
        (port.parent?.ck == pipe.targetPtr?.ck && portIdEquals(port, pipe.targetPort!))
      );
    });
  }

  /** Gets the hex color of the given pipe. */
  getPipeColorHex(pipe: PipeData): string | undefined {
    if (pipe.color != null) {
      return getColorHex(pipe.color, pipe.color?.shade ?? ColorShade.S600);
    } else {
      return undefined;
    }
  }

  /** Gets the computed (reactive) fields for a given step (may be from the flow or related nodes). */
  getStepFields(step: StepData, side: PortSide) {
    const state = this.stepsStates.value[step.id!];
    if (state == null || this.flow.value == null) return null;
    return getStepFields(this.spaceGraph, step, side, {
      stepFields: state.fields.value,
      flow: this.flow.value,
      flowFields: this.fields.value,
      node: state.node.value,
      nodeFields: state.nodeFields.value,
    });
  }

  /** Gets the (reactive) ports for a given step. */
  getPorts(step: StepData): { incoming: Port[]; outgoing: Port[] } {
    const state = this.stepsStates.value[step.id!];
    if (state == null || this.flow.value == null) return { incoming: [], outgoing: [] };
    return getPorts(this.spaceGraph, step, {
      stepFields: state.fields.value,
      flow: this.flow.value,
      flowFields: this.fields.value,
      node: state.node.value,
      nodeFields: state.nodeFields.value,
    });
  }

  /** Moves the thing */
  moveThing(thing: StepData | PipeData, move: { x: number; y: number }) {
    if (isNode(thing, NodeType.STEP)) {
      this.tx.update(thing, { position: addVector2(thing.position, move) }, { debounce: "long" });
    } else if (isNode(thing, NodeType.PIPE)) {
      throw new Error(":Incomplete: move pipe");
    } else {
      assertNever(thing);
    }
  }
}
export const FLOW_CONTEXT_KEY = Symbol("flow");

export function useFlowContext(): FlowContext {
  const flowContext = inject<FlowContext | null>(FLOW_CONTEXT_KEY, null);
  if (flowContext == null) throw new Error("no flow context");
  return flowContext;
}

/** Gets the containing flow block. */
export function getContainingFlow(graph: ReadNodeGraph, node: AnyNodeData): BlockData | null {
  const ancestors = graph.getAncestors(node, { includeSelf: true });
  return ancestors.find((n) => isNode(n, NodeType.BLOCK) && n.type == BlockType.FLOW) as BlockData | null;
}

/** Gets the computed (reactive) fields for a given step (may be from the flow or related nodes). */
export function getStepFields(
  graph: ReadNodeGraph,
  step: StepData,
  side: PortSide,
  related?: {
    stepFields: FieldData[];
    flow: BlockData;
    flowFields: FieldData[];
    node: BlockData | StepData | undefined | null;
    nodeFields: FieldData[];
  },
): {
  zone: FieldZone;
  fields: FieldData[];
  fieldParent: BlockData | StepData;
} | null {
  if (related == null) {
    // get related nodes (not reactive)
    const flow = getContainingFlow(graph, step);
    if (flow == null) return null;
    const node = graph.getMaybe(step.nodePtr) as BlockData | StepData | undefined;
    related = {
      stepFields: graph.getChildren(step, NodeType.FIELD),
      flow,
      flowFields: graph.getChildren(flow, NodeType.FIELD),
      node,
      nodeFields: node != null ? graph.getChildren(node, NodeType.FIELD) : [],
    };
  }

  if (step.type == StepType.START) {
    // from flow's input fields
    if (related.flow == null) return null;
    return {
      zone: FieldZone.INPUT,
      fields: side == PortSide.OUTGOING ? related.flowFields.filter((f) => f.zone == FieldZone.INPUT) : [],
      fieldParent: related.flow!,
    };
  } else if (step.type == StepType.COMPLETE) {
    // from flow's output fields
    if (related.flow == null) return null;
    return {
      zone: FieldZone.OUTPUT,
      fields: side == PortSide.INCOMING ? related.flowFields.filter((f) => f.zone == FieldZone.OUTPUT) : [],
      fieldParent: related.flow,
    };
  } else if (step.type == StepType.BLOCK) {
    // from block
    if (related.node == null) return null;
    const zone = side == PortSide.INCOMING ? FieldZone.INPUT : FieldZone.OUTPUT;
    return { zone, fields: related.nodeFields.filter((f) => f.zone == zone), fieldParent: related.node };
  } else {
    // step itself
    const zone = side == PortSide.INCOMING ? FieldZone.INPUT : FieldZone.OUTPUT;
    return { zone, fields: related.stepFields.filter((f) => f.zone == zone), fieldParent: step };
  }
}

/** Gets the (reactive) ports for a given step. Pass in related to avoid re-fetching if already known. */
export function getPorts(
  graph: ReadNodeGraph,
  step: StepData,
  related?: {
    stepFields: FieldData[];
    flow: BlockData;
    flowFields: FieldData[];
    node: BlockData | StepData | undefined | null;
    nodeFields: FieldData[];
  },
): { incoming: Port[]; outgoing: Port[] } {
  const incoming: Port[] = [];
  const outgoing: Port[] = [];

  // NOTE :UX: we hide :ObjectPorts by default for now (not sure how/when to enable, always enabled is cluttery)
  if (!INCOMING_STEP_TYPES.includes(step.type)) {
    // incoming ports
    incoming.push({ parent: step, idx: 0, type: PortType.RUN, side: PortSide.INCOMING });
  }
  if (!OUTGOING_STEP_TYPES.includes(step.type)) {
    // outgoing ports
    outgoing.push({ parent: step, idx: 0, type: PortType.RUN, side: PortSide.OUTGOING });
  }

  const incomingFields = getStepFields(graph, step, PortSide.INCOMING, related);
  const outgoingFields = getStepFields(graph, step, PortSide.OUTGOING, related);
  for (const field of incomingFields?.fields ?? []) {
    incoming.push({
      parent: step,
      idx: incoming.length,
      type: PortType.FIELD,
      side: PortSide.INCOMING,
      field,
      fieldPtr: toPlainNodeRef(field),
    });
  }
  for (const field of outgoingFields?.fields ?? []) {
    outgoing.push({
      parent: step,
      idx: outgoing.length,
      type: PortType.FIELD,
      side: PortSide.OUTGOING,
      field,
      fieldPtr: toPlainNodeRef(field),
    });
  }

  return { incoming: incoming, outgoing: outgoing };
}

/** Computes the pipe path SVG path string. */
export function pathToSvg(path: Vector2[]): string {
  const pathParts: string[] = [];
  for (let i = 0; i < path.length; i++) {
    const p = path[i];
    pathParts.push(`L${p.x},${p.y}`);
  }
  return `M${path[0].x},${path[0].y} ${pathParts.join(" ")}`;
}

/** Gets the view width for a Step. */
export function getStepWidth(step: StepData): number {
  if (BOUNDARY_STEP_TYPES.includes(step.type)) return FLOW_GRID_STEP * 8;
  else if (step.type == StepType.TEXT || step.type == StepType.CODE) return FLOW_GRID_STEP * 10;
  else return FLOW_GRID_STEP * 10;
}

/** Estimate the view size of a Step. Width should be exact, but height is likely overestimated a bit. */
export function estimateStepSize(step: StepData, verticalPorts: number): { width: number; height: number } {
  const width = getStepWidth(step);
  // base height
  let height =
    STEP_HEADER_HEIGHT + // header
    (FLOW_GRID_STEP - (STEP_HEADER_HEIGHT % FLOW_GRID_STEP) - FLOW_GRID_STEP / 2) + // header padding to align with grid
    FLOW_GRID_STEP * verticalPorts; // ports
  // content
  if (step.type == StepType.TEXT) {
    height += (step.text != null ? estimateTextHeight(step.text, width) : 20) + 10;
  } else if (step.type == StepType.CODE) {
    height += (step.code != null ? estimateCodeHeight(step.code, width) : 20) + 10;
  }
  // snap height to grid
  height = Math.ceil(height / FLOW_GRID_STEP) * FLOW_GRID_STEP;
  return { width, height };
}

export function createStep(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    step: { type: StepType } & Partial<StepData>;
    parent: StepData | TypedNodeReferenceData<NodeType.STEP> | BlockData | TypedNodeReferenceData<NodeType.BLOCK>;
    near?: Vector2 | null; // in world coordinates
  },
): StepData {
  const parent = isNode(options.parent) ? options.parent : graph.getOrError(options.parent);
  const parentPtr = toPlainNodeRef(parent);
  const packagePtr = parent.packagePtr;
  const flow = getContainingFlow(graph, parent);
  if (flow == null) throw new Error(`no flow for ${describeNode(parent)}`);

  // position in graph
  const siblings = graph.getChildren(parent, NodeType.STEP);
  const orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);

  // TODO :UX: create step in empty space
  let position: Vector2Data | null = options.step.position ?? null;
  if (options.near != null) {
    position = makeStruct({ metatype: StructType.VECTOR2, x: options.near.x, y: options.near.y });
  }

  // create
  const step = tx.create({
    metatype: NodeType.STEP,
    name: makeNodeName(graph, { metatype: ObjectType.STEP, type: options.step.type, parentPtr }),
    orderKey,
    ...options.step,
    position: position ?? undefined,
    type: options.step.type,
    parentPtr,
    packagePtr,
  });
  return step;
}

export function createPipe(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    parent: StepData | TypedNodeReferenceData<NodeType.STEP> | BlockData | TypedNodeReferenceData<NodeType.BLOCK>;
    pipe: { type: PipeType } & Partial<PipeData>;
    source: Port;
    target: Port;
  },
) {
  const parent = isNode(options.parent) ? options.parent : graph.getOrError(options.parent);
  const parentPtr = toPlainNodeRef(parent);
  const packagePtr = parent.packagePtr;

  // swap source/target if needed
  if (options.source.side == PortSide.INCOMING && options.target.side == PortSide.OUTGOING) {
    [options.source, options.target] = [options.target, options.source];
  }
  const { source, target } = options;
  if (source.side != PortSide.OUTGOING) throw new Error(`cannot pipe from incoming port`);
  if (target.side != PortSide.INCOMING) throw new Error(`cannot pipe to outgoing port`);

  // position in graph
  const siblings = graph.getChildren(parent, NodeType.PIPE);
  const orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);

  // create
  const pipe = tx.create({
    metatype: NodeType.PIPE,
    name: makeNodeName(graph, { metatype: ObjectType.PIPE, parentPtr }),
    orderKey,
    ...options.pipe,
    parentPtr,
    packagePtr,
    sourcePtr: toPlainNodeRef(source.parent),
    sourcePort: getPortKey(source),
    targetPtr: toPlainNodeRef(target.parent),
    targetPort: getPortKey(target),
  });
  return pipe;
}

interface AStarNode {
  x: number;
  y: number;
  g: number; // cost from start
  h: number; // heuristic estimate of distance to target
  f: number; // total cost
  parent: AStarNode | null;
}

function manhattanDistance(a: Vector2, b: Vector2): number {
  return Math.abs(a.x - b.x) + Math.abs(a.y - b.y);
}

function euclideanDistance(a: Vector2, b: Vector2): number {
  return Math.sqrt(Math.pow(a.x - b.x, 2) + Math.pow(a.y - b.y, 2));
}

/** Finds the shortest path between two points on a grid using the A* algorithm. */
function pathfind(
  source: Vector2,
  target: Vector2,
  options: { step: number; maxIterations: number; hit: (vec: Vector2) => BoundingBox | undefined },
): Vector2[] | null {
  const openSet: AStarNode[] = [];
  const closedSet: Set<string> = new Set();
  const MANHATTEN_WEIGHT = 1.0;
  const EUCLIDEAN_WEIGHT = 0.5;
  const DIRECTION_CHANGE_WEIGHT = 1.0;

  const startNode: AStarNode = {
    x: source.x,
    y: source.y,
    g: 0,
    h: manhattanDistance(source, target),
    f: 0,
    parent: null,
  };
  startNode.f = startNode.g + startNode.h;

  openSet.push(startNode);

  const directions = [
    { dx: options.step, dy: 0 },
    { dx: 0, dy: options.step },
    { dx: -options.step, dy: 0 },
    { dx: 0, dy: -options.step },
  ];

  let iterations = 0;
  while (openSet.length > 0 && iterations < options.maxIterations) {
    iterations++;
    openSet.sort((a, b) => a.f - b.f);
    const current = openSet.shift()!;

    if (Math.abs(target.x - current.x) <= options.step && target.y == current.y) {
      // path found, reconstruct and return it
      const path: Vector2[] = [];
      let node: AStarNode | null = current;
      while (node) {
        path.unshift({ x: node.x, y: node.y });
        node = node.parent;
      }
      return path;
    }
    closedSet.add(`${current.x},${current.y}`);

    for (const { dx, dy } of directions) {
      const nextPos = { x: current.x + dx, y: current.y + dy };
      if (options.hit(nextPos) || closedSet.has(`${nextPos.x},${nextPos.y}`)) {
        continue; // already hit or closed
      }

      const directionChanged = current.parent?.x !== nextPos.x && current.parent?.y !== nextPos.y;
      const g = current.g + options.step + (directionChanged ? options.step * DIRECTION_CHANGE_WEIGHT : 0);
      const h =
        manhattanDistance(nextPos, target) * MANHATTEN_WEIGHT +
        euclideanDistance(nextPos, target) * EUCLIDEAN_WEIGHT +
        (directionChanged ? 1 : 0) * options.step * DIRECTION_CHANGE_WEIGHT;

      const f = g + h;
      const existingOpenNode = openSet.find((node) => node.x === nextPos.x && node.y === nextPos.y);
      if (existingOpenNode) {
        if (g < existingOpenNode.g) {
          existingOpenNode.g = g;
          existingOpenNode.f = f;
          existingOpenNode.parent = current;
        }
      } else {
        openSet.push({
          x: nextPos.x,
          y: nextPos.y,
          g,
          h,
          f,
          parent: current,
        });
      }
    }
  }

  // no path found
  return null;
}
