import type { ReadNodeGraph } from "@/language/graph";
import { makeNodeName } from "@/language/node";
import type { Transaction } from "@/language/transaction";
import {
  BlockData,
  NodeType,
  ObjectType,
  PipeData,
  PortKeyData,
  StructType,
  TransformData,
  ViewData,
  type PipeType,
  type StepData,
  type StepType,
} from "@/proto/wire";
import { isNode, makeStruct, toPlainNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { addTransform } from "@/ui/view";
import { generateOrderKey } from "@/utils/fractional";
import { assertNever } from "@/utils/functools";
import { log } from "@/utils/log";
import type Step from "@/views/system/Step.vue";
import { computed, ref, type Ref } from "vue";

export const FLOW_GRID_STEP_X = 36;
export const FLOW_GRID_STEP_Y = 28;

export const FLOW_CANVAS_DOT_SIZE = 4;
export const FLOW_SCALE_MIN = 0.5;
export const FLOW_SCALE_MAX = 2.0;
export const FLOW_SCALE_SPEED = 0.01;

export const STEP_WIDTH = FLOW_GRID_STEP_X * 10;

export type PortId = Pick<PortKeyData, "type" | "side" | "fieldPtr">;

export class FlowContext {
  spaceGraph: ReadNodeGraph;
  graph: ReadNodeGraph;
  private spaceTxFactory: () => Transaction;
  private txFactory: () => Transaction;

  view: Ref<ViewData | null>;
  stepRefs: Ref<Record<string, InstanceType<typeof Step>>>;
  containerRef: Ref<HTMLElement | null>;
  dragging: Ref<{ thing: StepData | "canvas"; viewOffsetToThing: { x: number; y: number } } | null> = ref(null);

  flow: Ref<BlockData | null>;
  transform: Ref<TransformData>;
  scale: Ref<number>;
  steps: Ref<StepData[]>;
  pipes: Ref<PipeData[]>;

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

    this.view = context.view;
    this.containerRef = context.containerRef;
    this.stepRefs = context.stepRefs;

    this.flow = this.graph.getRef(context.flowPtr);
    this.transform = computed(() => this.view.value?.transform ?? makeStruct({ metatype: StructType.TRANSFORM }));
    this.scale = computed(() => this.transform.value.scaleX ?? this.transform.value.scaleY ?? 1);
    this.steps = this.graph.getChildrenRef(this.flow, NodeType.STEP);
    this.pipes = this.graph.getChildrenRef(this.flow, NodeType.PIPE);
  }

  get tx() {
    return this.txFactory();
  }

  get spaceTx() {
    return this.spaceTxFactory();
  }

  getStepComponent(step: StepData): InstanceType<typeof Step> | null {
    const stepRef = this.stepRefs.value[step.id!];
    return stepRef != null ? stepRef : null;
  }

  getStepBounding(step: StepData): DOMRect | null {
    const stepComponent = this.getStepComponent(step);
    if (stepComponent == null) return null;
    return stepComponent.$el.getBoundingClientRect();
  }

  /** Pan around the canvas */
  panCanvas(move: { x: number; y: number }) {
    if (this.view.value == null) return; // not a real view
    this.spaceTx.update(
      this.view.value,
      { transform: addTransform(this.transform.value, { translateX: move.x, translateY: move.y }) },
      { debounce: "long" },
    );
  }

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

  onDragging(e: MouseEvent) {
    if (this.dragging.value == null) return;
    if (this.view.value == null) return; // not a real view
    if (this.dragging.value.thing == "canvas") {
      // pan canvas
      const translateX = e.movementX / (this.transform.value?.scaleX ?? 1);
      const translateY = e.movementY / (this.transform.value?.scaleY ?? 1);
      this.spaceTx.update(
        this.view.value,
        { transform: addTransform(this.transform.value, { translateX, translateY }) },
        { debounce: "long" },
      );
    } else if (isNode(this.dragging.value.thing, NodeType.STEP)) {
      // move step (snap to grid)
      const screenVec = this.viewportToViewVec({
        x: e.clientX - this.dragging.value.viewOffsetToThing.x,
        y: e.clientY - this.dragging.value.viewOffsetToThing.y,
      });
      const worldVec = this.snapVec(this.viewToWorldVec(screenVec));
      this.tx.update(
        this.dragging.value.thing,
        { position: makeStruct({ metatype: StructType.VECTOR2, x: worldVec.x, y: worldVec.y }) },
        { debounce: "long" },
      );
    } else {
      assertNever(this.dragging.value.thing);
    }
  }

  startDragging(e: MouseEvent, thing: StepData | "canvas") {
    if (this.dragging.value != null) return; // already dragging
    if (thing == "canvas") {
      // start panning canvas
      this.dragging.value = {
        thing: "canvas",
        viewOffsetToThing: this.viewportToViewVec({ x: e.clientX, y: e.clientY }),
      };
    } else if (isNode(thing, NodeType.STEP)) {
      // start dragging step
      const stepBounding = this.getStepBounding(thing);
      if (stepBounding == null) return; // not mounted yet
      this.dragging.value = {
        thing,
        viewOffsetToThing: { x: e.clientX - stepBounding.left, y: e.clientY - stepBounding.top },
      };
    } else {
      assertNever(thing);
    }
    log.trace("flow.drag.start", this.dragging.value);
  }

  onWheel(e: WheelEvent) {
    const viewCenterVec = this.viewportToViewVec({ x: e.clientX, y: e.clientY });
    this.zoomCanvas(e.deltaY < 0 ? "in" : "out", viewCenterVec, Math.abs(e.deltaY * 0.5));
  }
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

  // create
  const step = tx.create({
    metatype: NodeType.STEP,
    ...options.step,
    type: options.step.type,
    parentPtr,
    packagePtr,
    orderKey,
    name: makeNodeName(graph, { metatype: ObjectType.STEP, type: options.step.type, parentPtr }),
  });
  return step;
}

export function createPipe(
  tx: Transaction,
  graph: ReadNodeGraph,
  options: {
    pipe: { type: PipeType; source: StepData; target: StepData } & Partial<StepData>;
  },
) {
  throw new Error("nocheckin: createPipe");
}
