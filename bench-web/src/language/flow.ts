import { INVISIBLE_STEP_TYPES, SINK_STEP_TYPES, SOURCE_STEP_TYPES } from "@/language/const";
import type { ReadNodeGraph } from "@/language/graph";
import { makeNodeName, NodeIn, unpackSubnodeProperty } from "@/language/node";
import type { Transaction, TransactionOptions } from "@/language/transaction";
import {
  BlockData,
  BlockType,
  ColorShade,
  FieldData,
  FieldType,
  NodeType,
  ObjectType,
  PipeData,
  PipeType,
  PortSide,
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
import { addTransform, addVector2, type Vector2 } from "@/ui/view";
import { generateOrderKey } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";
import { canvas } from "@/utils/globals";
import { log } from "@/utils/log";
import { computedValue } from "@/utils/ref";
import type Step from "@/views/system/Step.vue";
import { useMouse } from "@vueuse/core";
import { computed, inject, ref, shallowRef, triggerRef, watch, type Ref } from "vue";

export const STEP_CONTEXT_ACTIONS: ActionBuiltinId[] = [
  "common.edit.rename",
  "common.edit.duplicate",
  "common.edit.delete",
];
export const PIPE_CONTEXT_ACTIONS: ActionBuiltinId[] = [
  "common.edit.rename",
  "common.edit.duplicate",
  "common.edit.delete",
];

export const FLOW_GRID_STEP = 16;
export const FLOW_PORT_SIZE = 12;

export const FLOW_CANVAS_DOT_SIZE = 2;
export const FLOW_SCALE_MIN = 0.7;
export const FLOW_SCALE_MAX = 1.3;
export const FLOW_SCALE_SPEED = 0.01;

export const PIPE_WIDTH = 2;
export const STEP_SIZE = { width: FLOW_GRID_STEP * 17, height: FLOW_GRID_STEP * 3 };

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

export type Port = {
  parent: StepData;
  side: PortSide;
};

export function portEquals(a: Port, b: Port): boolean {
  return a.parent?.id == b.parent?.id && a.side == b.side;
}
export type PipePath = {
  start: Vector2;
  control1?: Vector2;
  control2?: Vector2;
  end: Vector2;
  midpoint: Vector2;
};

export type BoundingBox = { x1: number; y1: number; x2: number; y2: number; width: number; height: number };

export type FlowThing =
  | { kind: "canvas" }
  | { kind: "step"; step: StepData }
  | { kind: "pipe"; pipe: PipeData }
  | { kind: "port"; step: StepData; side: PortSide };

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
  // layout
  boundingBox: Ref<BoundingBox | null> = shallowRef(null);

  constructor(flow: FlowContext, step: StepData) {
    this.flow = flow;
    this.stepPtr = toPlainNodeRef(step);
    this.step = flow.graph.getRef(this.stepPtr, { ignoreAncestors: true });
    this.nodePtr = computedValue(() => {
      if (this.step.value?.type == StepType.ACTION) {
        const nodePtr = unpackSubnodeProperty(
          NodeType.STEP,
          StepType.ACTION,
          this.step.value?.subnodePacked,
          "delegatePtr",
        );
        return nodePtr as TypedNodeReferenceData<NodeType.BLOCK | NodeType.STEP> | null;
      } else {
        return null;
      }
    });
    this.node = flow.graph.getRef(this.nodePtr);
    this.nodeFields = flow.graph.getChildrenRef(this.nodePtr, NodeType.FIELD);
    this.fields = flow.graph.getChildrenRef(step, NodeType.FIELD);
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

    // layout
    this.path = computed(() => {
      if (this.source.value == null || this.target.value == null) return null;
      const sourceBounding = this.flow.getStepBoundingBox(this.source.value);
      const targetBounding = this.flow.getStepBoundingBox(this.target.value);
      if (sourceBounding == null || targetBounding == null) return null;
      const path = this.flow.computePath(sourceBounding, targetBounding);
      return path;
    });
    this.boundingBox = computedValue(() => {
      if (this.path.value == null) return null;
      return this.flow.computePathBoundingBox(this.path.value);
    });
  }
}
const mouse = useMouse();

/** An entire flow canvas (including steps, sub-steps, pipes, etc.) */
export class FlowContext {
  spaceGraph: ReadNodeGraph;
  graph: ReadNodeGraph;
  private spaceTxFactory: () => Transaction;
  private txFactory: () => Transaction;

  view: Ref<Partial<ViewData> | null>;
  update: (update: Partial<NodeIn<NodeType.VIEW>>, options?: TransactionOptions) => void;
  stepRefs: Ref<Record<string, InstanceType<typeof Step>>>;
  containerRef: Ref<HTMLElement | null>;
  dragging: Ref<{ thing: FlowThing; viewOffsetToThing: { x: number; y: number } } | null> = ref(null);
  cursorWorldPos: Ref<Vector2>;

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
    update: (update: Partial<NodeIn<NodeType.VIEW>>, options?: TransactionOptions) => void;
    view: Ref<ViewData | null>;
    transform: Ref<TransformData | null | undefined>;
    containerRef: Ref<HTMLElement | null>;
    stepRefs: Ref<Record<string, InstanceType<typeof Step>>>;
    flowPtr: Ref<TypedNodeReferenceData<NodeType.BLOCK>>;
  }) {
    this.spaceGraph = context.spaceGraph;
    this.spaceTxFactory = context.spaceTx;
    this.graph = context.graph;
    this.txFactory = context.tx;
    this.update = context.update;

    // view
    this.view = context.view;
    this.containerRef = context.containerRef;
    this.stepRefs = context.stepRefs;
    this.cursorWorldPos = computed(() => this.viewportToWorldVec({ x: mouse.x.value, y: mouse.y.value }));

    // flow
    this.flowPtr = context.flowPtr;
    this.flow = this.graph.getRef(context.flowPtr);
    this.fields = this.graph.getChildrenRef(this.flow, NodeType.FIELD);
    this.transform = computed(() => context.transform.value ?? makeStruct({ metatype: StructType.TRANSFORM }));
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
    return this.dragging.value?.thing.kind == "port";
  }

  isDraggingPortAt(step: StepData, side?: PortSide): boolean {
    return (
      this.dragging.value?.thing.kind == "port" &&
      this.dragging.value?.thing.step.id == step.id &&
      (!side || this.dragging.value?.thing.side == side)
    );
  }

  isDragging(step: StepData): boolean {
    return this.dragging.value?.thing.kind == "step" && this.dragging.value?.thing.step.id == step.id;
  }

  getStepComponent(step: StepData): InstanceType<typeof Step> | null {
    const stepRef = this.stepRefs.value[step.id!];
    return stepRef != null ? stepRef : null;
  }

  /** Gets the (reactive) estimated boundng box for a Step (in world coordinates). */
  getStepBoundingBox(step: StepData): BoundingBox | null {
    const stepState = this.stepsStates.value[step.id!];
    if (stepState == null) return null;
    const x1 = step.position?.x ?? 0;
    const y1 = step.position?.y ?? 0;
    const x2 = x1 + STEP_SIZE.width;
    const y2 = y1 + STEP_SIZE.height;
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
      if (INVISIBLE_STEP_TYPES.includes(step.type!)) continue;
      const state = this.stepsStates.value[step.id!];
      if (state == null) continue;
      x1 = Math.min(x1, step.position?.x ?? 0);
      y1 = Math.min(y1, step.position?.y ?? 0);
      x2 = Math.max(x2, (step.position?.x ?? 0) + STEP_SIZE.width);
      y2 = Math.max(y2, (step.position?.y ?? 0) + STEP_SIZE.height);
    }
    return { x1, y1, x2, y2, width: x2 - x1, height: y2 - y1 };
  }

  /** Computes the Pipe path */
  computePath(source: BoundingBox, target: BoundingBox): PipePath {
    const sides: Array<"top" | "right" | "bottom" | "left"> = ["top", "right", "bottom", "left"];
    const ANGLE_FACTOR = 1; // penalty for angle deviations
    const CONTROL_DISTANCE_FACTOR = 0.3; // control point distance factor
    const OFFSET_DISTANCE = FLOW_GRID_STEP / 2; // offset for opposing paths

    // get midpoint of a side
    function getMidpoint(box: BoundingBox, side: "top" | "right" | "bottom" | "left"): Vector2 {
      switch (side) {
        case "top":
          return { x: box.x1 + box.width / 2, y: box.y1 };
        case "right":
          return { x: box.x2, y: box.y1 + box.height / 2 };
        case "bottom":
          return { x: box.x1 + box.width / 2, y: box.y2 };
        case "left":
          return { x: box.x1, y: box.y1 + box.height / 2 };
      }
    }

    // get ideal direction vector for a side
    function getIdealDirection(side: "top" | "right" | "bottom" | "left"): Vector2 {
      switch (side) {
        case "top":
          return { x: 0, y: -1 };
        case "right":
          return { x: 1, y: 0 };
        case "bottom":
          return { x: 0, y: 1 };
        case "left":
          return { x: -1, y: 0 };
      }
    }

    // calculate angle deviation from orthogonal
    function calculateAngleDeviation(side: "top" | "right" | "bottom" | "left", path: Vector2): number {
      const ideal = getIdealDirection(side);
      const length = Math.hypot(path.x, path.y);
      if (length === 0) return 90;
      const dot = (path.x * ideal.x + path.y * ideal.y) / length;
      const clampedDot = Math.max(-1, Math.min(1, dot));
      const angle = Math.acos(clampedDot) * (180 / Math.PI);
      return Math.abs(angle);
    }

    // check if path intersects box
    function intersects(box: BoundingBox, start: Vector2, end: Vector2): boolean {
      const boxEdges: Array<[Vector2, Vector2]> = [
        [
          { x: box.x1, y: box.y1 },
          { x: box.x2, y: box.y1 },
        ],
        [
          { x: box.x2, y: box.y1 },
          { x: box.x2, y: box.y2 },
        ],
        [
          { x: box.x2, y: box.y2 },
          { x: box.x1, y: box.y2 },
        ],
        [
          { x: box.x1, y: box.y2 },
          { x: box.x1, y: box.y1 },
        ],
      ];
      for (const [p1, p2] of boxEdges) {
        if (lineSegmentsIntersect(p1, p2, start, end)) return true;
      }
      return false;
    }

    // check line segments intersection
    function lineSegmentsIntersect(p1: Vector2, p2: Vector2, q1: Vector2, q2: Vector2): boolean {
      function ccw(a: Vector2, b: Vector2, c: Vector2): boolean {
        return (c.y - a.y) * (b.x - a.x) > (b.y - a.y) * (c.x - a.x);
      }
      return ccw(p1, q1, q2) !== ccw(p2, q1, q2) && ccw(p1, p2, q1) !== ccw(p1, p2, q2);
    }

    let bestSource: Vector2 = getMidpoint(source, "top");
    let bestTarget: Vector2 = getMidpoint(target, "top");
    let bestCost = Infinity;
    let hasValidPath = false;

    for (const sourceSide of sides) {
      const se = getMidpoint(source, sourceSide);
      for (const targetSide of sides) {
        const te = getMidpoint(target, targetSide);
        const pathVector = { x: te.x - se.x, y: te.y - se.y };
        const distance = Math.hypot(pathVector.x, pathVector.y);
        // skip if path intersects source or target
        if (intersects(source, se, te) || intersects(target, se, te)) continue;
        const angleDevSource = calculateAngleDeviation(sourceSide, pathVector);
        const angleDevTarget = calculateAngleDeviation(targetSide, { x: -pathVector.x, y: -pathVector.y });
        // calculate cost with squared angle deviations
        const cost = distance + ANGLE_FACTOR * (angleDevSource ** 2 + angleDevTarget ** 2);
        if (cost < bestCost) {
          bestCost = cost;
          bestSource = se;
          bestTarget = te;
          hasValidPath = true;
        }
      }
    }

    // fallback to closest path if no valid path found
    if (!hasValidPath) {
      let shortestDistance = Infinity;
      let fallbackSource: Vector2 = getMidpoint(source, "top");
      let fallbackTarget: Vector2 = getMidpoint(target, "top");
      for (const sourceSide of sides) {
        const se = getMidpoint(source, sourceSide);
        for (const targetSide of sides) {
          const te = getMidpoint(target, targetSide);
          const distance = Math.hypot(se.x - te.x, se.y - te.y);
          if (distance < shortestDistance) {
            shortestDistance = distance;
            fallbackSource = se;
            fallbackTarget = te;
          }
        }
      }
      bestSource = fallbackSource;
      bestTarget = fallbackTarget;
    }

    // calculate control points with offset for opposing paths
    const pathVectorFinal = { x: bestTarget.x - bestSource.x, y: bestTarget.y - bestSource.y };
    const distanceFinal = Math.hypot(pathVectorFinal.x, pathVectorFinal.y);
    const controlDistance = distanceFinal * CONTROL_DISTANCE_FACTOR;

    // determine chosen sides
    const sourceSideChosen = sides.find(
      (side) => getMidpoint(source, side).x === bestSource.x && getMidpoint(source, side).y === bestSource.y,
    ) as "top" | "right" | "bottom" | "left";
    const targetSideChosen = sides.find(
      (side) => getMidpoint(target, side).x === bestTarget.x && getMidpoint(target, side).y === bestTarget.y,
    ) as "top" | "right" | "bottom" | "left";

    const idealSource = getIdealDirection(sourceSideChosen);
    const idealTarget = getIdealDirection(targetSideChosen);

    // determine offset direction based on side combinations
    const offset: Vector2 = { x: 0, y: 0 };
    if (
      (sourceSideChosen === "top" && targetSideChosen === "bottom") ||
      (sourceSideChosen === "bottom" && targetSideChosen === "top")
    ) {
      // offset along x-axis
      offset.x = sourceSideChosen === "top" ? OFFSET_DISTANCE : -OFFSET_DISTANCE;
    } else if (
      (sourceSideChosen === "left" && targetSideChosen === "right") ||
      (sourceSideChosen === "right" && targetSideChosen === "left")
    ) {
      // offset along y-axis
      offset.y = sourceSideChosen === "left" ? OFFSET_DISTANCE : -OFFSET_DISTANCE;
    }

    // apply offset to source and target
    const bestSourceOffset = { x: bestSource.x + offset.x, y: bestSource.y + offset.y };
    const bestTargetOffset = { x: bestTarget.x + offset.x, y: bestTarget.y + offset.y };

    // calculate control points
    const control1: Vector2 = {
      x: bestSourceOffset.x + idealSource.x * controlDistance,
      y: bestSourceOffset.y + idealSource.y * controlDistance,
    };
    const control2: Vector2 = {
      x: bestTargetOffset.x + idealTarget.x * controlDistance,
      y: bestTargetOffset.y + idealTarget.y * controlDistance,
    };

    // calculate Bezier midpoint
    const midpoint: Vector2 = {
      x: 0.125 * bestSourceOffset.x + 0.375 * control1.x + 0.375 * control2.x + 0.125 * bestTargetOffset.x,
      y: 0.125 * bestSourceOffset.y + 0.375 * control1.y + 0.375 * control2.y + 0.125 * bestTargetOffset.y,
    };

    return { start: bestSourceOffset, end: bestTargetOffset, midpoint, control1, control2 };
  }

  /** Gets the bounding box for a thing (in world coordinates). */
  getBoundingBox(thing: FlowThing): BoundingBox | null {
    if (thing.kind == "canvas") {
      return this.contentBoundingBox.value;
    } else if (thing.kind == "step") {
      return this.stepsStates.value[thing.step.id!]?.boundingBox.value;
    } else if (thing.kind == "pipe") {
      const pipeState = this.pipesStates.value[thing.pipe.id!];
      if (pipeState == null) return null;
      return pipeState.boundingBox.value;
    } else {
      throw new Error(`no bounding box for ${thing.kind}`);
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
    this.update(
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

    this.update(
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
    if (this.contentBoundingBox.value != null) {
      this.panToCenter({ kind: "canvas" });
    }
    this.zoom(1, "center", 0);
  }

  /** Pan the canvas in response to a wheel event */
  onWheel(e: WheelEvent) {
    // NOTE :UX: handle multitouch gestures in Flow better (zoom)
    const deltaX = e.deltaX;
    const deltaY = (e as any).webkitDirectionInvertedFromDevice ? -e.deltaY : e.deltaY;
    this.pan({ x: deltaX, y: deltaY });
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
    } else if (thing.kind == "port") {
      // create pending pipe
      this.dragging.value = { thing: thing, viewOffsetToThing: { x: 0, y: 0 } };
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
    const thing = this.dragging.value.thing;
    if (thing.kind == "canvas") {
      // pan canvas
      const translateX = e.movementX / (this.transform.value?.scaleX ?? 1);
      const translateY = e.movementY / (this.transform.value?.scaleY ?? 1);
      this.update({ transform: addTransform(this.transform.value, { translateX, translateY }) }, { debounce: "long" });
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
    } else if (thing.kind == "port") {
      // nothing to do
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

    // (re-)connect pipes
    try {
      if (this.draggable?.kind == "port") {
        const sourcePort = { parent: this.draggable.step, side: this.draggable.side };
        let targetPort: Port | null = null;
        if (at.kind == "port") {
          targetPort = { parent: at.step, side: at.side };
        } else if (at.kind == "step") {
          targetPort = { parent: at.step, side: getOtherSide(sourcePort.side) };
        }

        if (
          targetPort != null &&
          !portEquals(sourcePort, targetPort) &&
          !(SINK_STEP_TYPES.includes(sourcePort.parent.type) || SOURCE_STEP_TYPES.includes(targetPort?.parent.type))
        ) {
          // connect it up
          log.info("flow.drag.connect", { from: sourcePort, to: targetPort });
          const pipe = createPipe(this.tx, this.graph, {
            parent: this.flow.value,
            pipe: { type: PipeType.PASS, isNameHidden: true },
            source: sourcePort,
            target: targetPort,
          });
          if (this.view.value != null) {
            canvas.inspect({ node: pipe, view: this.view.value });
          }
        } else {
          // nothing to do
        }
      }
    } catch (e) {
      toaster.error({ title: "Invalid Pipe", text: (e as any)?.message ?? "Cannot pipe like that." });
      log.error("flow.drag.connect.error", this.draggable, at, e);
    }

    log.trace("flow.drag.end", { from: this.dragging.value, to: at });
    this.dragging.value = null;
  }

  /** Gets the (first) Step at the given position */
  getStepAt(position: Vector2, filter?: (step: StepData) => boolean): StepData | null {
    let steps = this.steps.value;
    if (filter != null) {
      steps = steps.filter(filter);
    }
    const hit = steps.find((step) => {
      const bounding = this.getStepBoundingBox(step);
      return (
        bounding != null &&
        bounding.x1 <= position.x &&
        position.x <= bounding.x2 &&
        bounding.y1 <= position.y &&
        position.y <= bounding.y2
      );
    });
    return hit ?? null;
  }

  /** Gets the bounding box for a pipe path (in world coordinates). */
  computePathBoundingBox(path: PipePath): BoundingBox | null {
    // (taking bezier control points into account)
    if (path.control1 == null || path.control2 == null) {
      const x1 = Math.min(path.start.x, path.end.x);
      const y1 = Math.min(path.start.y, path.end.y);
      const x2 = Math.max(path.start.x, path.end.x);
      const y2 = Math.max(path.start.y, path.end.y);
      return { x1, y1, x2, y2, width: x2 - x1, height: y2 - y1 };
    } else {
      const x1 = Math.min(path.start.x, path.control1.x, path.control2.x, path.end.x);
      const y1 = Math.min(path.start.y, path.control1.y, path.control2.y, path.end.y);
      const x2 = Math.max(path.start.x, path.control1.x, path.control2.x, path.end.x);
      const y2 = Math.max(path.start.y, path.control1.y, path.control2.y, path.end.y);
      return { x1, y1, x2, y2, width: x2 - x1, height: y2 - y1 };
    }
  }

  /** Gets the pipes connected to the given port. */
  getPipesAtPort(step: StepData, side: PortSide): PipeData[] {
    if (side == PortSide.INCOMING) return this.pipes.value.filter((pipe) => pipe.targetPtr?.ck == step.ck);
    else return this.pipes.value.filter((pipe) => pipe.sourcePtr?.ck == step.ck);
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

export function getOtherSide(side: PortSide): PortSide {
  return side == PortSide.INCOMING ? PortSide.OUTGOING : PortSide.INCOMING;
}

/** Gets the sides that a Step has ports on */
export function getStepSides(step: StepData): PortSide[] {
  if (step.type == StepType.TEXT) return []; // no ports
  const sides: PortSide[] = [];
  if (!SOURCE_STEP_TYPES.includes(step.type)) sides.push(PortSide.INCOMING);
  if (!SINK_STEP_TYPES.includes(step.type)) sides.push(PortSide.OUTGOING);
  return sides;
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
  type: FieldType;
  fields: FieldData[];
  fieldParent: BlockData | StepData;
} | null {
  if (related == null) {
    // get related nodes (not reactive)
    const flow = getContainingFlow(graph, step);
    if (flow == null) return null;
    let node: BlockData | StepData | undefined | null = null;
    if (step.type == StepType.ACTION) {
      const nodePtr = unpackSubnodeProperty(NodeType.STEP, StepType.ACTION, step.subnodePacked, "delegatePtr");
      node = graph.getMaybe(nodePtr) as BlockData | StepData | undefined;
    }
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
      type: FieldType.INPUT,
      fields: side == PortSide.OUTGOING ? related.flowFields.filter((f) => f.type == FieldType.INPUT) : [],
      fieldParent: related.flow!,
    };
  } else if (step.type == StepType.COMPLETE) {
    // from flow's output fields
    if (related.flow == null) return null;
    return {
      type: FieldType.OUTPUT,
      fields: side == PortSide.INCOMING ? related.flowFields.filter((f) => f.type == FieldType.OUTPUT) : [],
      fieldParent: related.flow,
    };
  } else if (step.type == StepType.ACTION) {
    // from block
    if (related.node == null) return null;
    const zone = side == PortSide.INCOMING ? FieldType.INPUT : FieldType.OUTPUT;
    return { type: zone, fields: related.nodeFields.filter((f) => f.type == zone), fieldParent: related.node };
  } else {
    // step itself
    const zone = side == PortSide.INCOMING ? FieldType.INPUT : FieldType.OUTPUT;
    return { type: zone, fields: related.stepFields.filter((f) => f.type == zone), fieldParent: step };
  }
}

/** Interpolate the given path */
export function interpolatePath(points: Vector2[], factor: number = 2): Vector2[] {
  const interpolated: Vector2[] = [];

  for (let i = 0; i < points.length - 1; i++) {
    const p1 = points[i];
    const p2 = points[i + 1];
    interpolated.push(p1);

    for (let j = 1; j < factor; j++) {
      const t = j / factor;
      interpolated.push({
        x: p1.x + (p2.x - p1.x) * t,
        y: p1.y + (p2.y - p1.y) * t,
      });
    }
  }

  interpolated.push(points[points.length - 1]);
  return interpolated;
}

export function pathToSvg(path: PipePath): string {
  if (path.control1 == null || path.control2 == null) {
    return `M ${path.start.x} ${path.start.y} L ${path.end.x} ${path.end.y}`;
  } else {
    return `M ${path.start.x} ${path.start.y} C ${path.control1.x} ${path.control1.y}, ${path.control2.x} ${path.control2.y}, ${path.end.x} ${path.end.y}`;
  }
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
    name: makeNodeName(graph, { ...options.pipe, metatype: ObjectType.PIPE, parentPtr }),
    orderKey,
    ...options.pipe,
    parentPtr,
    packagePtr,
    sourcePtr: toPlainNodeRef(source.parent),
    targetPtr: toPlainNodeRef(target.parent),
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

    if (Math.abs(target.x - current.x) <= 1 && Math.abs(target.y - current.y) <= 1) {
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
