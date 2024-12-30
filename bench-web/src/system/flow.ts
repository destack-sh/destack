import { INVISIBLE_ACTION_TYPES, SINK_ACTION_TYPES, SOURCE_ACTION_TYPES } from "@/language/const";
import { makeTypeInfo } from "@/language/field";
import type { ReadNodeGraph } from "@/language/graph";
import { makeNodeName, NodeIn, unpackSubnode, unpackSubnodeProperty } from "@/language/node";
import { newChangeId, type Transaction, type TransactionOptions } from "@/language/transaction";
import {
  ActionType,
  BenchType,
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
  SelectionData,
  StructType,
  TransformData,
  TypeKind,
  Vector2Data,
  ViewData,
  ViewType,
  type ActionData,
  type AnyNodeData,
} from "@/proto/wire";
import { describeNode, isNode, makeStruct, toNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { canvas, supergraph } from "@/system/globals";
import { declareActions } from "@/ui/action";
import { isDragAllowed } from "@/ui/drag";
import { ICON_BY_NODE_TYPE } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import { toaster } from "@/ui/toast";
import { addVector2, lengthVector2, subVector2, type Vector2 } from "@/ui/view";
import { generateOrderKey } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";
import { log } from "@/utils/log";
import { computedValue } from "@/utils/ref";
import type Action from "@/views/nodes/Action.vue";
import { useMouse } from "@vueuse/core";
import { computed, inject, ref, shallowRef, triggerRef, watch, type Ref } from "vue";

export const FLOW_GRID_STEP = 16;
export const FLOW_PORT_SIZE = 12;

export const FLOW_CANVAS_DOT_SIZE = 2;
export const FLOW_SCALE_MIN = 0.5;
export const FLOW_SCALE_MAX = 1.5;
export const FLOW_SCALE_SPEED = 0.01;

export const PIPE_WIDTH = 2;
export const PIPE_CONNECTION_DISTANCE = FLOW_GRID_STEP * 2; // minimum distance to consider a connection when dragging a port
export const ACTION_SIZE = { width: FLOW_GRID_STEP * 17, height: FLOW_GRID_STEP * 3 };
export const ACTION_SIZE_HALF = { width: ACTION_SIZE.width / 2, height: ACTION_SIZE.height / 2 };

// TODO :Cleanup :Architecture: enum/object descriptions should be in language (and exported from there during build)
export const DEFAULT_TEXT_BY_ACTION_TYPE: Partial<Record<ActionType, string>> = {
  // start
  [ActionType.START]: "Begin the flow.",
  [ActionType.TRIGGER]: "Trigger the flow.",
  // end
  [ActionType.COMPLETE]: "End the entire flow.",
  [ActionType.FAIL]: "Fail the entire flow.",
  // read
  [ActionType.GET]: "Get a specific Node.",
  [ActionType.SEARCH]: "Search for Nodes.",
  [ActionType.COPY]: "Copy to clipboard.",
  // write
  [ActionType.CREATE]: "Create a new Node.",
  [ActionType.UPDATE]: "Update a Node.",
  [ActionType.DELETE]: "Delete a Node.",
  [ActionType.DUPLICATE]: "Duplicate the clipboard.",
  [ActionType.PASTE]: "Paste from clipboard.",
  // run
  [ActionType.YIELD]: "Pause and wait for someone.",
  // application
  [ActionType.OBSERVE]: "Look at the screen.",
  [ActionType.CLICK]: "Click somewhere on the screen.",
  [ActionType.PRESS]: "Press some keys.",
  [ActionType.TYPE]: "Enter text.",
  [ActionType.SCROLL]: "Scroll the screen.",
  [ActionType.SELECT]: "Select something.",
  [ActionType.GO_BACKWARD]: "Go back in history.",
  [ActionType.GO_FORWARD]: "Go forward in history.",
  // web
  [ActionType.GO_TO_URL]: "Navigate to a URL.",
  [ActionType.GO_TO_TAB]: "Switch to a tab.",
  [ActionType.OPEN_TAB]: "Open a new tab.",
  [ActionType.CLOSE_TAB]: "Close a tab.",
  // containers
  [ActionType.LOOP]: "Loop over a list.",
};

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
  parent: ActionData;
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
  | { kind: "action"; action: ActionData }
  | { kind: "pipe"; pipe: PipeData }
  | { kind: "selection"; selection: SelectionData; nodes: AnyNodeData[] }
  | { kind: "port"; action: ActionData; side: PortSide };

/** Action state in a Flow. */
export class ActionState {
  // self
  flow: FlowContext;
  actionPtr: TypedNodeReferenceData<NodeType.ACTION>;
  action: Ref<ActionData | null>;
  subnode: Ref<any | null>;
  delegatePtr: Ref<TypedNodeReferenceData<NodeType.BLOCK | NodeType.ACTION> | null>;
  delegate: Ref<BlockData | ActionData | null>;
  fields: Ref<FieldData[]>;
  actionFields: Ref<FieldData[]>;
  delegateFields: Ref<FieldData[]>;
  // layout
  boundingBox: Ref<BoundingBox | null> = shallowRef(null);

  constructor(flow: FlowContext, action: ActionData) {
    this.flow = flow;
    this.actionPtr = toNodeRef(action);
    this.action = flow.graph.getRef(this.actionPtr, { ignoreAncestors: true });
    this.subnode = computed(() => {
      if (this.action.value == null) return null;
      return unpackSubnode(NodeType.ACTION, this.action.value.type, this.action.value.subnodePacked);
    });
    this.delegatePtr = computedValue(() => {
      if (this.action.value?.type == ActionType.DELEGATE) {
        return this.action.value.delegatePtr as TypedNodeReferenceData<NodeType.BLOCK | NodeType.ACTION> | null;
      } else {
        return null;
      }
    });
    this.delegate = flow.graph.getRef(this.delegatePtr);
    this.delegateFields = flow.graph.getChildrenRef(this.delegatePtr, NodeType.FIELD);
    this.actionFields = flow.graph.getChildrenRef(action, NodeType.FIELD);
    this.fields = computed(() => {
      const actionType = this.action.value?.type;
      if (actionType == ActionType.START) {
        return this.flow.fields.value.filter((f) => f.type == FieldType.INPUT);
      } else if (actionType == ActionType.COMPLETE) {
        return this.flow.fields.value.filter((f) => f.type == FieldType.OUTPUT);
      } else if (actionType == ActionType.DELEGATE) {
        if (this.action.value?.delegatePtr != null) {
          return this.delegateFields.value;
        } else {
          return this.actionFields.value;
        }
      } else if (actionType == ActionType.CREATE) {
        // TODO :Incomplete: builtin stub fields
        return [];
      } else if (actionType == ActionType.YIELD) {
        return this.actionFields.value;
      } else {
        return [];
      }
    });
    // layout
    this.boundingBox = computedValue(() =>
      this.action.value != null ? this.flow.getActionBoundingBox(this.action.value) : null,
    );
  }
}

/** Pipe state in a Flow. */
export class PipeState {
  // self
  flow: FlowContext;
  pipePtr: TypedNodeReferenceData<NodeType.PIPE>;
  pipe: Ref<PipeData | null>;
  source: Ref<ActionData | null>;
  target: Ref<ActionData | null>;

  // layout
  path: Ref<PipePath | null>;
  boundingBox: Ref<BoundingBox | null>;

  constructor(flow: FlowContext, pipe: PipeData) {
    this.flow = flow;
    this.pipePtr = toNodeRef(pipe);
    this.pipe = flow.graph.getRef(this.pipePtr);
    this.source = flow.graph.getRef(
      computed(() => this.pipe.value?.sourcePtr as TypedNodeReferenceData<NodeType.ACTION> | null),
    );
    this.target = flow.graph.getRef(
      computed(() => this.pipe.value?.targetPtr as TypedNodeReferenceData<NodeType.ACTION> | null),
    );

    // layout
    this.path = computed(() => {
      if (this.source.value == null || this.target.value == null) return null;
      const sourceBounding = this.flow.getActionBoundingBox(this.source.value);
      const targetBounding = this.flow.getActionBoundingBox(this.target.value);
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

/** An entire flow canvas (including actions, sub-actions, pipes, etc.) */
export class FlowContext {
  spaceGraph: ReadNodeGraph;
  graph: ReadNodeGraph;
  private spaceTxFactory: () => Transaction;
  private txFactory: () => Transaction;

  view: Ref<ViewData | null>;
  update: (update: Partial<NodeIn<NodeType.VIEW>>, options?: TransactionOptions) => void;
  actionRefs: Ref<Record<string, InstanceType<typeof Action>>>;
  containerRef: Ref<HTMLElement | null>;
  dragging: Ref<{ thing: FlowThing; viewOffsetByThing: Record<string, { x: number; y: number }> } | null> = ref(null);
  cursorWorldPos: Ref<Vector2>;
  viewport: Ref<{
    scale: number;
    transform: TransformData & Required<Pick<TransformData, "translateX" | "translateY">>;
  }>;

  flowPtr: Ref<TypedNodeReferenceData<NodeType.BLOCK> | null>;
  flow: Ref<BlockData | null>;
  fields: Ref<FieldData[]>;
  actions: Ref<ActionData[]>;
  pipes: Ref<PipeData[]>;

  actionsStates: Ref<Record<string, ActionState>> = shallowRef({});
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
    actionRefs: Ref<Record<string, InstanceType<typeof Action>>>;
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
    this.actionRefs = context.actionRefs;
    this.cursorWorldPos = computed(() => this.viewportToWorldVec({ x: mouse.x.value, y: mouse.y.value }));
    this.viewport = computed(() => {
      if (context.transform.value != null) {
        // manual transform
        const scale = context.transform.value.scaleX ?? 1;
        const translateX = context.transform.value.translateX ?? 0;
        const translateY = context.transform.value.translateY ?? 0;
        return { scale, transform: { metatype: ObjectType.TRANSFORM, translateX, translateY } };
      } else {
        // automatic transform
        const padding = FLOW_GRID_STEP * 2;
        const contentBoundingBox = this.contentBoundingBox.value;
        const containerBounding = this.containerRef.value?.getBoundingClientRect();
        if (contentBoundingBox == null || containerBounding == null)
          return { scale: 1.0, transform: { metatype: ObjectType.TRANSFORM, translateX: 0, translateY: 0 } };

        // calculate scale needed to fit content with padding
        const contentWidth = contentBoundingBox.width + padding * 2;
        const contentHeight = contentBoundingBox.height + padding * 2;
        const scaleX = containerBounding.width / contentWidth;
        const scaleY = containerBounding.height / contentHeight;
        let scale = Math.min(scaleX, scaleY);

        // clamp scale between min/max and round to action
        scale = Math.max(FLOW_SCALE_MIN, Math.min(1.0, scale));
        scale = Math.round(scale / FLOW_SCALE_SPEED) * FLOW_SCALE_SPEED;

        // calculate translation to center content with padding
        const translateX = -contentBoundingBox.x1 + (containerBounding.width / scale - contentBoundingBox.width) / 2;
        const translateY = -contentBoundingBox.y1 + (containerBounding.height / scale - contentBoundingBox.height) / 2;

        return {
          scale,
          transform: { metatype: ObjectType.TRANSFORM, translateX, translateY },
        };
      }
    });

    // flow
    this.flowPtr = context.flowPtr;
    this.flow = this.graph.getRef(context.flowPtr);
    this.fields = this.graph.getChildrenRef(this.flow, NodeType.FIELD);
    this.actions = this.graph.getChildrenRef(this.flow, NodeType.ACTION);
    this.pipes = this.graph.getChildrenRef(this.flow, NodeType.PIPE);

    // maintain action/pipe contexts
    watch(
      this.actions,
      () => {
        const actionsIds = this.actions.value.map((s) => s.id);
        this.actions.value
          .filter((action) => this.actionsStates.value[action.id] == null)
          .forEach(
            (action) => (
              (this.actionsStates.value[action.id] = new ActionState(this, action)), triggerRef(this.actionsStates)
            ),
          );
        Object.keys(this.actionsStates.value)
          .filter((actionId) => !actionsIds.includes(actionId))
          .forEach((actionId) => (delete this.actionsStates.value[actionId], triggerRef(this.actionsStates)));
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

  get isDragging(): boolean {
    return this.dragging.value != null;
  }

  isDraggingPortAt(action: ActionData, side?: PortSide): boolean {
    return (
      this.dragging.value?.thing.kind == "port" &&
      this.dragging.value?.thing.action.id == action.id &&
      (!side || this.dragging.value?.thing.side == side)
    );
  }

  isDraggingAction(action: ActionData): boolean {
    return this.dragging.value?.thing.kind == "action" && this.dragging.value?.thing.action.id == action.id;
  }

  getActionComponent(action: ActionData): InstanceType<typeof Action> | null {
    const actionRef = this.actionRefs.value[action.id!];
    return actionRef != null ? actionRef : null;
  }

  /** Gets the (reactive) estimated boundng box for a Action (in world coordinates). */
  getActionBoundingBox(action: ActionData): BoundingBox | null {
    const actionState = this.actionsStates.value[action.id!];
    if (actionState == null) return null;
    const x1 = action.position?.x ?? 0;
    const y1 = action.position?.y ?? 0;
    const x2 = x1 + ACTION_SIZE.width;
    const y2 = y1 + ACTION_SIZE.height;
    return { x1, y1, x2, y2, width: x2 - x1, height: y2 - y1 };
  }

  //
  // Canvas
  //

  get currentViewport(): {
    scale: number;
    transform: TransformData & Required<Pick<TransformData, "translateX" | "translateY">>;
  } {
    return this.viewport.value;
  }

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
    const actions = this.actions.value;
    if (actions.length == 0) return null;
    let x1 = actions[0].position?.x ?? 0;
    let y1 = actions[0].position?.y ?? 0;
    let x2 = actions[0].position?.x ?? 0;
    let y2 = actions[0].position?.y ?? 0;
    for (const action of actions) {
      if (INVISIBLE_ACTION_TYPES.includes(action.type!)) continue;
      const state = this.actionsStates.value[action.id!];
      if (state == null) continue;
      x1 = Math.min(x1, action.position?.x ?? 0);
      y1 = Math.min(y1, action.position?.y ?? 0);
      x2 = Math.max(x2, (action.position?.x ?? 0) + ACTION_SIZE.width);
      y2 = Math.max(y2, (action.position?.y ?? 0) + ACTION_SIZE.height);
    }
    return { x1, y1, x2, y2, width: x2 - x1, height: y2 - y1 };
  }

  /** Computes the Pipe path */
  computePath(source: BoundingBox, target: BoundingBox): PipePath {
    const sides: Array<"top" | "right" | "bottom" | "left"> = ["top", "right", "bottom", "left"];
    const ANGLE_FACTOR = 1; // penalty for angle deviations
    const CONTROL_DISTANCE_FACTOR = 0.3; // control point distance factor
    const OPPOSITE_OFFSET = FLOW_GRID_STEP / 2; // offset for opposing paths

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

    // special case: self-loop
    if (source.x1 == target.x1 && source.y1 == target.y1) {
      const sourcePos = getMidpoint(source, "right");
      const targetPos = getMidpoint(source, "bottom");
      sourcePos.y -= FLOW_GRID_STEP / 2;
      targetPos.x += FLOW_GRID_STEP / 2;
      const control1 = { x: sourcePos.x + FLOW_GRID_STEP * 5, y: sourcePos.y + FLOW_GRID_STEP * 4 };
      const control2 = { x: targetPos.x + FLOW_GRID_STEP * 1, y: targetPos.y + FLOW_GRID_STEP * 2 };
      const midpoint: Vector2 = {
        x: 0.125 * sourcePos.x + 0.375 * control1.x + 0.375 * control2.x + 0.125 * targetPos.x,
        y: 0.125 * sourcePos.y + 0.375 * control1.y + 0.375 * control2.y + 0.125 * targetPos.y,
      };
      return { start: sourcePos, end: targetPos, midpoint, control1, control2 };
    }

    let bestSourceSide: "top" | "right" | "bottom" | "left" = "top";
    let bestSource: Vector2 = getMidpoint(source, bestSourceSide);
    let bestTargetSide: "top" | "right" | "bottom" | "left" = "top";
    let bestTarget: Vector2 = getMidpoint(target, bestTargetSide);
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
          bestSourceSide = sourceSide;
          bestSource = se;
          bestTargetSide = targetSide;
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
            bestSourceSide = sourceSide;
            fallbackTarget = te;
            bestTargetSide = targetSide;
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
    const idealSource = getIdealDirection(bestSourceSide);
    const idealTarget = getIdealDirection(bestTargetSide);

    // determine offset direction based on side combinations
    const sourcePos = { x: bestSource.x, y: bestSource.y };
    const targetPos = { x: bestTarget.x, y: bestTarget.y };
    if (
      (bestSourceSide === "top" && bestTargetSide === "bottom") ||
      (bestSourceSide === "bottom" && bestTargetSide === "top")
    ) {
      // offset both the same way along x-axis
      const offsetX = bestSourceSide === "top" ? OPPOSITE_OFFSET : -OPPOSITE_OFFSET;
      sourcePos.x += offsetX;
      targetPos.x += offsetX;
    } else if (
      (bestSourceSide === "left" && bestTargetSide === "right") ||
      (bestSourceSide === "right" && bestTargetSide === "left")
    ) {
      // offset both the same way along y-axis
      const offsetY = bestSourceSide === "left" ? OPPOSITE_OFFSET : -OPPOSITE_OFFSET;
      sourcePos.y += offsetY;
      targetPos.y += offsetY;
    } else {
      // offset individually (no parallel line possible)
      if (bestSourceSide == "left" || bestSourceSide == "right") {
        sourcePos.y += OPPOSITE_OFFSET;
      } else {
        sourcePos.x += OPPOSITE_OFFSET;
      }
      if (bestTargetSide == "left" || bestTargetSide == "right") {
        targetPos.y -= OPPOSITE_OFFSET;
      } else {
        targetPos.x -= OPPOSITE_OFFSET;
      }
    }

    // calculate control points
    const control1: Vector2 = {
      x: sourcePos.x + idealSource.x * controlDistance,
      y: sourcePos.y + idealSource.y * controlDistance,
    };
    const control2: Vector2 = {
      x: targetPos.x + idealTarget.x * controlDistance,
      y: targetPos.y + idealTarget.y * controlDistance,
    };

    // calculate Bezier midpoint
    const midpoint: Vector2 = {
      x: 0.125 * sourcePos.x + 0.375 * control1.x + 0.375 * control2.x + 0.125 * targetPos.x,
      y: 0.125 * sourcePos.y + 0.375 * control1.y + 0.375 * control2.y + 0.125 * targetPos.y,
    };

    return { start: sourcePos, end: targetPos, midpoint, control1, control2 };
  }

  /** Gets the bounding box for a thing (in world coordinates). */
  getBoundingBox(thing: FlowThing): BoundingBox | null {
    if (thing.kind == "canvas") {
      return this.contentBoundingBox.value;
    } else if (thing.kind == "action") {
      return this.actionsStates.value[thing.action.id!]?.boundingBox.value;
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
      {
        transform: {
          metatype: ObjectType.TRANSFORM,
          scaleX: this.currentViewport.scale,
          translateX: this.currentViewport.transform.translateX + move.x,
          translateY: this.currentViewport.transform.translateY + move.y,
        },
      },
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
      x: worldVec.x + this.currentViewport.transform.translateX,
      y: worldVec.y + this.currentViewport.transform.translateY,
    };
    viewVec = { x: viewVec.x * this.currentViewport.scale, y: viewVec.y * this.currentViewport.scale };
    return viewVec;
  }

  /** Convert world coordinates to viewport coordinates */
  worldToViewportVec(worldVec: { x: number; y: number }): { x: number; y: number } {
    const viewVec = this.worldToViewVec(worldVec);
    return this.viewToViewportVec(viewVec);
  }

  /** Convert screen coordinates to world coordinates */
  viewToWorldVec(viewVec: { x: number; y: number }): { x: number; y: number } {
    let worldVec = { x: viewVec.x / this.currentViewport.scale, y: viewVec.y / this.currentViewport.scale };
    worldVec = {
      x: worldVec.x - this.currentViewport.transform.translateX,
      y: worldVec.y - this.currentViewport.transform.translateY,
    };
    return worldVec;
  }

  /** Zoom the convas around the given origin (panning as needed) */
  zoom(direction: "in" | "out" | number, originViewVec: { x: number; y: number } | "center", actions: number) {
    const containerBounding = this.containerRef.value?.getBoundingClientRect();
    if (containerBounding == null) throw new Error("no container bounding");
    if (originViewVec == "center") {
      originViewVec = { x: containerBounding.width / 2, y: containerBounding.height / 2 };
    }
    // figure out new zoom
    const currentZoom = this.currentViewport.scale;
    let newZoom: number;
    if (direction == "in") {
      newZoom = Math.min(FLOW_SCALE_MAX, currentZoom + FLOW_SCALE_SPEED * actions);
    } else if (direction == "out") {
      newZoom = Math.max(FLOW_SCALE_MIN, currentZoom - FLOW_SCALE_SPEED * actions);
    } else {
      newZoom = direction;
    }
    if (newZoom == currentZoom) return; // no change
    const translateX = this.currentViewport.transform.translateX;
    const translateY = this.currentViewport.transform.translateY;

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
    const newTransform = {
      ...this.currentViewport.transform,
      scaleX: newZoom,
      scaleY: newZoom,
      translateX: translateX + panVec.x,
      translateY: translateY + panVec.y,
    };
    this.update({ transform: newTransform }, { debounce: "long" });
  }

  /** Resets the viewport */
  resetViewport() {
    this.update({ transform: undefined }, { debounce: "long" });
  }

  /** Sets the viewport to the current viewport (stops auto viewport) */
  setViewport() {
    this.update(
      { transform: { ...this.currentViewport.transform, scaleX: this.currentViewport.scale } },
      { debounce: "long" },
    );
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
    if (!isDragAllowed(target, "drag")) {
      return false;
    } else {
      return this.startDragging(e, thing);
    }
  }

  /** Starts dragging a thing. */
  startDragging(e: MouseEvent, thing: FlowThing): boolean {
    if (this.dragging.value != null) return false; // already dragging
    this.setViewport(); // prevent auto viewport to avoid jankiness while dragging

    if (thing.kind == "canvas") {
      // start panning canvas
      this.dragging.value = { thing, viewOffsetByThing: {} };
    } else if (thing.kind == "action") {
      // start moving action
      if (canvas.isSelected(thing.action)) {
        // promote to selection
        const nodes = supergraph.getManyMaybe(canvas.selection!.nodesPtr);
        const actions = nodes.filter((s) => isNode(s, NodeType.ACTION));
        const viewOffsetByThing: Record<string, { x: number; y: number }> = {};
        for (const action of actions) {
          const positionViewportVec = this.worldToViewportVec({
            x: action.position?.x ?? 0,
            y: action.position?.y ?? 0,
          });
          viewOffsetByThing[action.id] = { x: e.clientX - positionViewportVec.x, y: e.clientY - positionViewportVec.y };
        }
        thing = { kind: "selection", selection: canvas.selection!, nodes };
        this.dragging.value = { thing, viewOffsetByThing };
      } else {
        const positionViewportVec = this.worldToViewportVec({
          x: thing.action.position?.x ?? 0,
          y: thing.action.position?.y ?? 0,
        });
        const viewOffsetToThing = { x: e.clientX - positionViewportVec.x, y: e.clientY - positionViewportVec.y };
        this.dragging.value = { thing, viewOffsetByThing: { [thing.action.id]: viewOffsetToThing } };
      }
    } else if (thing.kind == "port") {
      // create pending pipe
      const positionViewportVec = this.worldToViewportVec({
        x: thing.action.position?.x ?? 0,
        y: thing.action.position?.y ?? 0,
      });
      const viewOffsetToThing = { x: e.clientX - positionViewportVec.x, y: e.clientY - positionViewportVec.y };
      this.dragging.value = { thing: thing, viewOffsetByThing: { [thing.action.id]: viewOffsetToThing } };
    } else {
      throw new Error(`cannot drag ${thing.kind}`);
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
      const translateX = e.movementX / this.currentViewport.scale;
      const translateY = e.movementY / this.currentViewport.scale;
      this.update(
        {
          transform: {
            metatype: ObjectType.TRANSFORM,
            scaleX: this.currentViewport.scale,
            scaleY: this.currentViewport.scale,
            translateX: this.currentViewport.transform.translateX + translateX,
            translateY: this.currentViewport.transform.translateY + translateY,
          },
        },
        { debounce: "long" },
      );
    } else if (thing.kind == "action") {
      // move action (snap to grid)
      const viewOffset = this.dragging.value.viewOffsetByThing[thing.action.id];
      if (viewOffset == null) throw new Error(`no view offset for ${describeNode(thing.action)}`);
      const screenVec = this.viewportToViewVec({ x: e.clientX - viewOffset.x, y: e.clientY - viewOffset.y });
      const worldVec = snapVec(this.viewToWorldVec(screenVec));
      this.tx.update(
        thing.action,
        { position: makeStruct({ metatype: StructType.VECTOR2, x: worldVec.x, y: worldVec.y }) },
        { debounce: "long" },
      );
    } else if (thing.kind == "selection") {
      // move all actions in selection (snap each to grid)
      const tx = this.tx.with({ change: { key: newChangeId(), title: "Move" } });
      for (const node of thing.nodes) {
        if (!isNode(node, NodeType.ACTION)) continue;
        const viewOffset = this.dragging.value.viewOffsetByThing[node.id];
        if (viewOffset == null) throw new Error(`no view offset for ${describeNode(node)}`);
        const screenVec = this.viewportToViewVec({ x: e.clientX - viewOffset.x, y: e.clientY - viewOffset.y });
        const worldVec = snapVec(this.viewToWorldVec(screenVec));
        tx.update(
          node,
          { position: makeStruct({ metatype: StructType.VECTOR2, x: worldVec.x, y: worldVec.y }) },
          { debounce: "long" },
        );
      }
    } else if (thing.kind == "port") {
      // nothing to do?
    } else {
      throw new Error(`cannot drag ${thing.kind}`);
    }
  }

  /** Cancel dragging (and don't trigger any release events). */
  cancelDragging() {
    if (this.dragging.value == null) return;
    this.dragging.value = null;
    log.trace("flow.drag.cancel", this.dragging.value);
  }

  /** Checks if two ports can be connected. */
  canPortsConnect(sourcePort: Port, targetPort: Port) {
    return (
      !portEquals(sourcePort, targetPort) /* can't connect same port */ &&
      !SINK_ACTION_TYPES.includes(sourcePort.parent.type) /* can't go from sink */ &&
      !SOURCE_ACTION_TYPES.includes(targetPort?.parent.type) /* can't go to source */ &&
      !this.pipes.value.some(
        (pipe) => pipe.sourcePtr?.ck == sourcePort.parent.ck && pipe.targetPtr?.ck == targetPort.parent.ck,
      ) /* can't connect same two Actions twice */
    );
  }

  /** Stop dragging a thing (if any). Triggers a 'release' event to connect things. */
  endDragging(e: MouseEvent, at: FlowThing) {
    if (this.flow.value == null) throw new Error("no flow to connect");
    if (this.dragging.value == null) return;

    // (re-)connect pipes
    try {
      if (this.draggable?.kind == "port") {
        const sourceAction = this.draggable.action;
        const sourcePort = { parent: this.draggable.action, side: this.draggable.side };
        const sourceBounding = this.getActionBoundingBox(sourceAction);
        if (sourceBounding == null) throw new Error(`no bounding box for ${describeNode(sourceAction)}`);
        const sourcePos = {
          x: sourceBounding.x1 + (this.dragging.value?.viewOffsetByThing[sourceAction.id]?.x ?? 0),
          y: sourceBounding.y1 + (this.dragging.value?.viewOffsetByThing[sourceAction.id]?.y ?? 0),
        };
        const cursor = this.cursorWorldPos.value;
        const distance = lengthVector2(subVector2(sourcePos, cursor));
        if (distance < PIPE_CONNECTION_DISTANCE) return null;

        let targetPort: Port | null = null;
        if (at.kind == "port") {
          targetPort = { parent: at.action, side: at.side };
        } else if (at.kind == "action") {
          targetPort = { parent: at.action, side: getOtherSide(sourcePort.side) };
        } else {
          // dragged into emptyness, open action picker
          const targetPosition = this.viewportToWorldVec({ x: e.clientX, y: e.clientY });
          canvas.pushPopover({
            trigger: e.target as HTMLElement,
            reference: { x: e.clientX, y: e.clientY },
            kind: "view",
            placement: "bottom",
            title: "Add Action",
            component: ViewType.PICKER,
            props: {
              valueType: makeTypeInfo({ kind: TypeKind.ENUM, benchType: BenchType.ACTION_TYPE }),
            },
            onApply: (value) => {
              const tx = this.tx.with({ change: { key: newChangeId() } });
              const action = this.createAction({
                parent: this.flow.value!,
                near: { x: targetPosition.x - ACTION_SIZE.width / 2, y: targetPosition.y - ACTION_SIZE.height / 2 },
                action: { type: value },
                tx,
              });
              if (
                action.type != ActionType.TEXT &&
                this.canPortsConnect(sourcePort, { parent: action, side: PortSide.INCOMING })
              ) {
                this.createPipe({
                  parent: this.flow.value!,
                  pipe: { type: PipeType.PASS },
                  source: sourcePort,
                  target: { parent: action, side: PortSide.INCOMING },
                  tx,
                });
              }
              canvas.inspect({ node: action, view: this.view.value });
            },
          });
          return;
        }

        if (this.canPortsConnect(sourcePort, targetPort)) {
          // connect it up
          log.debug("flow.drag.connect", { from: sourcePort, to: targetPort });
          const pipe = this.createPipe({
            parent: this.flow.value,
            pipe: { type: PipeType.PASS },
            source: sourcePort,
            target: targetPort,
          });
          canvas.inspect({ node: pipe, view: this.view.value });
        } else {
          // nothing to do?
          toaster.error({
            title: "Invalid Pipe",
            text: `Cannot connect ${sourcePort.parent.name} to ${targetPort.parent.name}.`,
          });
        }
      }
    } catch (e) {
      toaster.error({ title: "Invalid Pipe", text: (e as any)?.message ?? "Cannot pipe like that." });
      log.error("flow.drag.connect.error", this.draggable, at, e);
    }

    log.trace("flow.drag.end", { from: this.dragging.value, to: at });
    this.dragging.value = null;
  }

  /** Gets the (first) Action at the given position (in world coordinates) */
  getActionAt(position: Vector2, filter?: (action: ActionData) => boolean): ActionData | null {
    let actions = this.actions.value;
    if (filter != null) {
      actions = actions.filter(filter);
    }
    const hit = actions.find((action) => {
      const bounding = this.getActionBoundingBox(action);
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
  getPipesAtPort(action: ActionData, side: PortSide): PipeData[] {
    if (side == PortSide.INCOMING) return this.pipes.value.filter((pipe) => pipe.targetPtr?.ck == action.ck);
    else return this.pipes.value.filter((pipe) => pipe.sourcePtr?.ck == action.ck);
  }

  /** Gets the hex color of the given pipe. */
  getPipeColorHex(pipe: PipeData): string | undefined {
    if (pipe.color != null) {
      return getColorHex(pipe.color, pipe.color?.shade ?? ColorShade.S600);
    } else {
      return undefined;
    }
  }

  /** Gets the computed (reactive) fields for a given action (may be from the flow or related nodes). */
  getActionFields(action: ActionData, side: PortSide) {
    const state = this.actionsStates.value[action.id!];
    if (state == null || this.flow.value == null) return null;
    return getActionFields(this.spaceGraph, action, side, {
      actionFields: state.actionFields.value,
      flow: this.flow.value,
      flowFields: this.fields.value,
      node: state.delegate.value,
      nodeFields: state.delegateFields.value,
    });
  }

  /** Moves the thing */
  move(
    thing: ActionData | PipeData,
    move: { x: number; y: number },
    options?: { tx?: Transaction } & TransactionOptions,
  ) {
    const tx = options?.tx ?? this.tx;
    if (isNode(thing, NodeType.ACTION)) {
      tx.update(thing, { position: addVector2(thing.position, move) }, { debounce: "long", ...options });
    } else if (isNode(thing, NodeType.PIPE)) {
      throw new Error(":Incomplete: move pipe");
    } else {
      assertNever(thing);
    }
  }

  /** Find empty space of the given size */
  findEmptySpace(
    near: Vector2,
    size: { width: number; height: number },
    options?: {
      bias?: "right" | "down";
      maxActions?: number;
      margin?: { x: number; y: number };
    },
  ): Vector2Data | null {
    near = snapVec(near);
    const position = { ...near };
    const { maxActions = 100, margin = { x: FLOW_GRID_STEP, y: FLOW_GRID_STEP * 4 }, bias = "down" } = options ?? {};
    const hitOffsets = [
      { x: 0, y: 0 },
      { x: size.width, y: 0 },
      { x: 0, y: size.height },
      { x: size.width, y: size.height },
      { x: size.width / 2, y: size.height / 2 },
    ];
    if (margin.x != 0 || margin.y != 0) {
      // add hitoffsets for every action [0, margin.x] and [0, margin.y]
      for (let testX = 0; testX < margin.x; testX += FLOW_GRID_STEP) {
        for (let testY = 0; testY < margin.y; testY += FLOW_GRID_STEP) {
          hitOffsets.push({ x: -testX, y: -testY });
          hitOffsets.push({ x: size.width + testX, y: -testY });
          hitOffsets.push({ x: -testX, y: size.height + testY });
          hitOffsets.push({ x: size.width + testX, y: size.height + testY });
        }
      }
    }

    let numActions = 0;
    while (numActions < maxActions) {
      // check if there's any overlapping action at the position (including bounding corners)
      let hit = false;
      for (const hitOffset of hitOffsets) {
        const hitPosition = addVector2(position, hitOffset);
        if (this.getActionAt(hitPosition) != null) {
          hit = true;
          break;
        }
      }
      if (!hit) {
        // found empty space
        return { ...position, metatype: ObjectType.VECTOR2 };
      }

      // advance action
      if (bias === "right") {
        position.x += FLOW_GRID_STEP;
      } else if (bias === "down") {
        position.y += FLOW_GRID_STEP;
      } else {
        assertNever(bias);
      }
      numActions++;
    }

    return null;
  }

  /** Creates a Action */
  createAction(options: {
    action: { type: ActionType } & Partial<ActionData>;
    parent: ActionData | TypedNodeReferenceData<NodeType.ACTION> | BlockData | TypedNodeReferenceData<NodeType.BLOCK>;
    near?: Vector2 | null; // in world coordinates
    tx?: Transaction;
  }): ActionData {
    const parent = isNode(options.parent) ? options.parent : this.graph.getOrError(options.parent);
    const parentPtr = toNodeRef(parent);
    const packagePtr = parent.packagePtr;
    const flow = getContainingFlow(this.graph, parent);
    if (flow == null) throw new Error(`no flow for ${describeNode(parent)}`);

    // position in graph
    const siblings = this.graph.getChildren(parent, NodeType.ACTION);
    const orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);
    const position =
      options.action.position ??
      this.findEmptySpace(
        options.near ?? subVector2(this.centerVec!, { x: ACTION_SIZE.width / 2, y: ACTION_SIZE.height / 2 }),
        ACTION_SIZE,
      );

    // create
    const action = (options.tx ?? this.tx).create({
      metatype: NodeType.ACTION,
      name: makeNodeName(this.graph, { metatype: ObjectType.ACTION, type: options.action.type, parentPtr }),
      orderKey,
      ...options.action,
      position: position ?? undefined,
      type: options.action.type as any,
      parentPtr,
      packagePtr,
    });
    canvas.inspect({ node: action, view: this.view.value });
    return action;
  }

  /** Creates a Pipe */
  createPipe(options: {
    parent: ActionData | TypedNodeReferenceData<NodeType.ACTION> | BlockData | TypedNodeReferenceData<NodeType.BLOCK>;
    pipe: { type: PipeType } & Partial<PipeData>;
    source: Port;
    target: Port;
    tx?: Transaction;
  }) {
    const parent = isNode(options.parent) ? options.parent : this.graph.getOrError(options.parent);
    const parentPtr = toNodeRef(parent);
    const packagePtr = parent.packagePtr;

    // swap source/target if needed
    if (options.source.side == PortSide.INCOMING && options.target.side == PortSide.OUTGOING) {
      [options.source, options.target] = [options.target, options.source];
    }
    const { source, target } = options;
    if (source.side != PortSide.OUTGOING) throw new Error(`cannot pipe from incoming port`);
    if (target.side != PortSide.INCOMING) throw new Error(`cannot pipe to outgoing port`);

    // position in graph
    const siblings = this.graph.getChildren(parent, NodeType.PIPE);
    const orderKey = generateOrderKey(siblings[siblings.length - 1]?.orderKey ?? null, null);

    // create
    const pipe = (options.tx ?? this.tx).create({
      metatype: NodeType.PIPE,
      name: makeNodeName(this.graph, { ...options.pipe, metatype: ObjectType.PIPE, parentPtr }),
      orderKey,
      ...options.pipe,
      parentPtr,
      packagePtr,
      sourcePtr: toNodeRef(source.parent),
      targetPtr: toNodeRef(target.parent),
    });
    canvas.inspect({ node: pipe, view: this.view.value });
    return pipe;
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

/** Gets the sides that a Action has ports on */
export function getActionSides(action: ActionData): PortSide[] {
  if (action.type == ActionType.TEXT) return []; // no ports
  const sides: PortSide[] = [];
  if (!SOURCE_ACTION_TYPES.includes(action.type)) sides.push(PortSide.INCOMING);
  if (!SINK_ACTION_TYPES.includes(action.type)) sides.push(PortSide.OUTGOING);
  return sides;
}

/** Gets the computed (reactive) fields for a given action (may be from the flow or related nodes). */
export function getActionFields(
  graph: ReadNodeGraph,
  action: ActionData,
  side: PortSide,
  related?: {
    actionFields: FieldData[];
    flow: BlockData;
    flowFields: FieldData[];
    node: BlockData | ActionData | undefined | null;
    nodeFields: FieldData[];
  },
): {
  type: FieldType;
  fields: FieldData[];
  fieldParent: BlockData | ActionData;
} | null {
  if (related == null) {
    // get related nodes (not reactive)
    const flow = getContainingFlow(graph, action);
    if (flow == null) return null;
    let node: BlockData | ActionData | undefined | null = null;
    if (action.type == ActionType.DELEGATE && action.delegatePtr != null) {
      node = graph.getMaybe(action.delegatePtr) as BlockData | ActionData | undefined;
    }
    related = {
      actionFields: graph.getChildren(action, NodeType.FIELD),
      flow,
      flowFields: graph.getChildren(flow, NodeType.FIELD),
      node,
      nodeFields: node != null ? graph.getChildren(node, NodeType.FIELD) : [],
    };
  }

  if (action.type == ActionType.START) {
    // from flow's input fields
    if (related.flow == null) return null;
    return {
      type: FieldType.INPUT,
      fields: side == PortSide.OUTGOING ? related.flowFields.filter((f) => f.type == FieldType.INPUT) : [],
      fieldParent: related.flow!,
    };
  } else if (action.type == ActionType.COMPLETE) {
    // from flow's output fields
    if (related.flow == null) return null;
    return {
      type: FieldType.OUTPUT,
      fields: side == PortSide.INCOMING ? related.flowFields.filter((f) => f.type == FieldType.OUTPUT) : [],
      fieldParent: related.flow,
    };
  } else if (action.type == ActionType.DELEGATE && action.delegatePtr != null) {
    // from block
    if (related.node == null) return null;
    const type = side == PortSide.INCOMING ? FieldType.INPUT : FieldType.OUTPUT;
    return { type: type, fields: related.nodeFields.filter((f) => f.type == type), fieldParent: related.node };
  } else {
    // action itself
    const type = side == PortSide.INCOMING ? FieldType.INPUT : FieldType.OUTPUT;
    return { type: type, fields: related.actionFields.filter((f) => f.type == type), fieldParent: action };
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
      interpolated.push({ x: p1.x + (p2.x - p1.x) * t, y: p1.y + (p2.y - p1.y) * t });
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

//
// Actions
//

// flow
declareActions<"flow">({
  "flow.edit.createAction": {
    icon: ICON_BY_NODE_TYPE[NodeType.ACTION],
    title: "Create Action",
    text: "Create a new action",
  },
  "flow.edit.splitPipe": {
    icon: "fas fa-scissors",
    title: "Split Pipe",
    text: "Split this pipe",
  },
});
