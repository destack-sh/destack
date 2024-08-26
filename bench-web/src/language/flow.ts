import { BOUNDARY_STEP_TYPES, INCOMING_STEP_TYPES, OUTGOING_STEP_TYPES } from "@/language/const";
import type { ReadNodeGraph } from "@/language/graph";
import { makeNodeName } from "@/language/node";
import type { Transaction } from "@/language/transaction";
import {
  BlockData,
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
  type StepData,
} from "@/proto/wire";
import { isNode, makeStruct, toPlainNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
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

export const FLOW_GRID_STEP_X = 28;
export const FLOW_GRID_STEP_Y = 28;
export const FLOW_PORT_SIZE = 12;

export const FLOW_CANVAS_DOT_SIZE = 4;
export const FLOW_SCALE_MIN = 0.5;
export const FLOW_SCALE_MAX = 4.0;
export const FLOW_SCALE_SPEED = 0.01;

export const PIPE_WIDTH = 4;
export const STEP_HEADER_HEIGHT = VIEW_DEFAULT_HEADER_HEIGHT;

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

export type FlowThing =
  | {
      kind: "canvas";
    }
  | {
      kind: "step";
      step: StepData;
    }
  | {
      kind: "step-port";
      step: StepData;
      port: Port;
      cursorWorldPos?: { x: number; y: number };
    };
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
      this.step.value != null
        ? this.flow.getPorts(this.step.value, {
            fields: this.fields.value,
            node: this.node.value!,
            nodeFields: this.nodeFields.value,
          })
        : { incoming: [], outgoing: [] },
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
  path: Ref<Vector2[] | null>;

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
      return this.flow.computePath(sourcePortPosition, targetPortPosition);
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
        this.steps.value
          .filter((step) => this.stepsStates.value[step.id] == null)
          .forEach(
            (step) => ((this.stepsStates.value[step.id] = new StepState(this, step)), triggerRef(this.stepsStates)),
          );
        Object.keys(this.stepsStates.value)
          .filter((stepId) => !this.graph.has({ id: stepId }))
          .forEach((stepId) => (delete this.stepsStates.value[stepId], triggerRef(this.stepsStates)));
      },
      { immediate: true },
    );
    watch(
      this.pipes,
      () => {
        this.pipes.value
          .filter((pipe) => this.pipesStates.value[pipe.id] == null)
          .forEach(
            (pipe) => ((this.pipesStates.value[pipe.id] = new PipeState(this, pipe)), triggerRef(this.pipesStates)),
          );
        Object.keys(this.pipesStates.value)
          .filter((pipeId) => !this.graph.has({ id: pipeId }))
          .forEach((pipeId) => (delete this.pipesStates.value[pipeId], triggerRef(this.pipesStates)));
      },
      { immediate: true },
    );
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

  getStepWidth(step: StepData): number {
    if (BOUNDARY_STEP_TYPES.includes(step.type)) return FLOW_GRID_STEP_X * 8;
    else if (step.type == StepType.TEXT || step.type == StepType.CODE) return FLOW_GRID_STEP_X * 10;
    else return FLOW_GRID_STEP_X * 10;
  }

  getStepComponent(step: StepData): InstanceType<typeof Step> | null {
    const stepRef = this.stepRefs.value[step.id!];
    return stepRef != null ? stepRef : null;
  }

  getStepComponentBounding(step: StepData): DOMRect | null {
    const stepComponent = this.getStepComponent(step);
    if (stepComponent == null) return null;
    return stepComponent.$el.getBoundingClientRect();
  }

  //
  // Canvas
  //

  /** Pan around the canvas (in world coordinates). */
  panCanvas(move: { x: number; y: number }) {
    if (this.view.value == null) return; // not a real view
    this.spaceTx.update(
      this.view.value,
      { transform: addTransform(this.transform.value, { translateX: move.x, translateY: move.y }) },
      { debounce: "long" },
    );
  }

  /** Rounds the given vector to the nearest grid position (in world coordinates). */
  snapVec(vec: { x: number; y: number }): { x: number; y: number } {
    return {
      x: Math.round(vec.x / FLOW_GRID_STEP_X) * FLOW_GRID_STEP_X,
      y: Math.round(vec.y / FLOW_GRID_STEP_Y) * FLOW_GRID_STEP_Y,
    };
  }

  /** Convert viewport coordinates to view coordinates */
  viewportToViewVec(viewportVec: { x: number; y: number }): { x: number; y: number } {
    const canvasBounding = this.containerRef.value?.getBoundingClientRect()!;
    return {
      x: viewportVec.x - canvasBounding.left,
      y: viewportVec.y - canvasBounding.top,
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
  zoomCanvas(direction: "in" | "out", originViewVec: { x: number; y: number } | "center", steps: number) {
    if (this.view.value == null) return; // not a real view
    const containerBounding = this.containerRef.value?.getBoundingClientRect()!;
    if (originViewVec == "center") {
      originViewVec = { x: containerBounding.width / 2, y: containerBounding.height / 2 };
    }
    // figure out new zoom
    const currentZoom = this.scale.value;
    const newZoom =
      direction == "in"
        ? Math.min(FLOW_SCALE_MAX, currentZoom + FLOW_SCALE_SPEED * steps)
        : Math.max(FLOW_SCALE_MIN, currentZoom - FLOW_SCALE_SPEED * steps);
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
    this.spaceTx.update(
      this.view.value,
      { transform: makeStruct({ metatype: StructType.TRANSFORM }) },
      { debounce: "short" },
    );
  }

  /** Zooms the canvas in/out in response to a "wheel" event. */
  onWheel(e: WheelEvent) {
    const viewCenterVec = this.viewportToViewVec({ x: e.clientX, y: e.clientY });
    this.zoomCanvas(e.deltaY < 0 ? "in" : "out", viewCenterVec, Math.abs(e.deltaY * 0.5));
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
      const stepBounding = this.getStepComponentBounding(thing.step);
      if (stepBounding == null) return false; // not mounted yet
      this.dragging.value = {
        thing,
        viewOffsetToThing: { x: e.clientX - stepBounding.left, y: e.clientY - stepBounding.top },
      };
    } else if (thing.kind == "step-port") {
      // create pending pipe
      this.dragging.value = {
        thing: { ...thing, cursorWorldPos: this.viewportToWorldVec({ x: e.clientX, y: e.clientY }) },
        viewOffsetToThing: { x: 0, y: 0 },
      };
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
      const worldVec = this.snapVec(this.viewToWorldVec(screenVec));
      this.tx.update(
        thing.step,
        { position: makeStruct({ metatype: StructType.VECTOR2, x: worldVec.x, y: worldVec.y }) },
        { debounce: "long" },
      );
    } else if (thing.kind == "step-port") {
      // update cursor position
      thing.cursorWorldPos = this.viewportToWorldVec({ x: e.clientX, y: e.clientY });
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
        log.debug("flow.drag.connect", { from: this.draggable.port, to: at.port });
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
      basePosition.x += this.getStepWidth(step);
    }
    // move down below header :FlowGrid
    basePosition.y += STEP_HEADER_HEIGHT + FLOW_GRID_STEP_Y - (STEP_HEADER_HEIGHT % FLOW_GRID_STEP_Y);
    // move down to port
    basePosition.y += port.idx * FLOW_GRID_STEP_Y;
    return basePosition;
  }

  /** Computes the snapped path for a pipe (in world coordinates). */
  computePath(source: Vector2, target: Vector2): Vector2[] {
    // NOTE :UX: it would be nice to coordinate pipe paths amongst each other
    // nocheckin: nicer pipe paths (pathfinding, snap to grid)
    return [source, target];
  }

  /** Computes the pipe path SVG path string. */
  pathToSvg(path: Vector2[]): string {
    const pathParts: string[] = [];
    for (let i = 0; i < path.length; i++) {
      const p = path[i];
      pathParts.push(`L${p.x},${p.y}`);
    }
    return `M${path[0].x},${path[0].y} ${pathParts.join(" ")}`;
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

  /** Gets the (reactive) ports for a given step. Pass in related to avoid re-fetching if already known. */
  getPorts(
    step: StepData,
    related: { fields: FieldData[]; node: BlockData | StepData | undefined; nodeFields: FieldData[] } | undefined,
  ): { incoming: Port[]; outgoing: Port[] } {
    const incoming: Port[] = [];
    const outgoing: Port[] = [];

    if (related == null) {
      // get related nodes (not reactive)
      related = {
        fields: this.graph.getChildren(step, NodeType.FIELD),
        node: this.graph.getMaybe(step.nodePtr) as BlockData | StepData | undefined,
        nodeFields: step.nodePtr != null ? this.graph.getChildren(step.nodePtr, NodeType.FIELD) : [],
      };
    }

    // NOTE :UX: we hide :ObjectPorts by default for now (not sure how/when to enable, always enabled is cluttery)
    if (!INCOMING_STEP_TYPES.includes(step.type)) {
      // incoming ports
      incoming.push({ parent: step, idx: 0, type: PortType.RUN, side: PortSide.INCOMING });
    }
    if (!OUTGOING_STEP_TYPES.includes(step.type)) {
      // outgoing ports
      outgoing.push({ parent: step, idx: 0, type: PortType.RUN, side: PortSide.OUTGOING });
    }

    if (step.type == StepType.START) {
      // flow input fields as outgoing ports
      for (const field of this.fields.value) {
        if (field.zone == FieldZone.INPUT) {
          outgoing.push({
            parent: step,
            idx: outgoing.length,
            type: PortType.FIELD,
            side: PortSide.OUTGOING,
            field,
            fieldPtr: toPlainNodeRef(field),
            fieldParent: this.flow.value!,
          });
        }
      }
    } else if (step.type == StepType.COMPLETE) {
      // flow output fields as incoming ports
      for (const field of this.fields.value) {
        if (field.zone == FieldZone.OUTPUT) {
          incoming.push({
            parent: step,
            idx: incoming.length,
            type: PortType.FIELD,
            side: PortSide.INCOMING,
            field,
            fieldPtr: toPlainNodeRef(field),
            fieldParent: this.flow.value!,
          });
        }
      }
    } else if (step.type == StepType.CODE || step.type == StepType.TEXT) {
      // our own fields as incoming/outgoing ports
      for (const field of related.fields) {
        if (field.zone == FieldZone.INPUT) {
          incoming.push({ parent: step, idx: incoming.length, type: PortType.FIELD, side: PortSide.INCOMING, field });
        } else if (field.zone == FieldZone.OUTPUT) {
          outgoing.push({ parent: step, idx: outgoing.length, type: PortType.FIELD, side: PortSide.OUTGOING, field });
        }
      }
    } else if (step.type == StepType.BLOCK) {
      // borrow fields as incoming/outgoing ports
      for (const field of related.nodeFields) {
        if (field.zone == FieldZone.INPUT) {
          incoming.push({
            parent: step,
            idx: incoming.length,
            type: PortType.FIELD,
            side: PortSide.INCOMING,
            field,
            fieldPtr: toPlainNodeRef(field),
            fieldParent: related.node,
          });
        } else if (field.zone == FieldZone.OUTPUT) {
          outgoing.push({
            parent: step,
            idx: outgoing.length,
            type: PortType.FIELD,
            side: PortSide.OUTGOING,
            field,
            fieldPtr: toPlainNodeRef(field),
            fieldParent: related.node,
          });
        }
      }
    }

    return { incoming: incoming, outgoing: outgoing };
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

export function createStep(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    step: { type: StepType } & Partial<StepData>;
    parent: StepData | TypedNodeReferenceData<NodeType.STEP> | BlockData | TypedNodeReferenceData<NodeType.BLOCK>;
  },
): StepData {
  const parent = isNode(options.parent) ? options.parent : graph.getOrError(options.parent);
  const parentPtr = toPlainNodeRef(parent);
  const packagePtr = parent.packagePtr;

  // position in graph
  const siblings = graph.getChildren(parent, NodeType.STEP);
  const orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);

  // nocheckin: position in flow/view
  //  (also on duplicate?)

  // create
  const step = tx.create({
    metatype: NodeType.STEP,
    name: makeNodeName(graph, { metatype: ObjectType.STEP, type: options.step.type, parentPtr }),
    orderKey,
    ...options.step,
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
