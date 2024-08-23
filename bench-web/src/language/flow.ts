import type { ReadNodeGraph } from "@/language/graph";
import { makeNodeName } from "@/language/node";
import type { Transaction } from "@/language/transaction";
import { BlockData, NodeType, ObjectType, type PipeType, type StepData, type StepType } from "@/proto/wire";
import { isNode, toPlainNodeRef, type TypedNodeReferenceData } from "@/proto/wiring";
import { generateOrderKey } from "@/utils/fractional";

export const FLOW_GRID_STEP_X = 32;
export const FLOW_GRID_STEP_Y = 24;

export const STEP_WIDTH = 320;

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
