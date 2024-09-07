import type { ReadNodeGraph } from "@/language/graph";
import { makeNode } from "@/language/node";
import {
  BlockType,
  CodeData,
  FieldData,
  FieldZone,
  NodeReferenceData,
  NodeType,
  RunKind,
  RunStatus,
  Struct,
  StructType,
  TextData,
  type AnyNodeData,
  type BlockData,
  type RunData,
  type StepData,
} from "@/proto/wire";
import { describeNode, isNode, isStruct, toPlainNodeRef } from "@/proto/wiring";
import { assertNever } from "@/utils/functools";

export type RunnableNode = BlockData | StepData;
export type RunnableObject = RunnableNode | TextData | CodeData;

/** Whether the given node is runnable */
export function isRunnable(node: AnyNodeData, graph: ReadNodeGraph, fields?: FieldData[]): node is RunnableNode {
  if (isNode(node, NodeType.STEP)) {
    return true;
  } else if (isNode(node, NodeType.BLOCK)) {
    if (node.type == BlockType.CODE || node.type == BlockType.FLOW) {
      return true;
    } else if (node.type == BlockType.TEXT) {
      fields = fields ?? graph.getChildren(node, NodeType.FIELD);
      return fields.some((f) => f.zone == FieldZone.INPUT) && fields.some((f) => f.zone == FieldZone.OUTPUT);
    } else {
      return false;
    }
  } else {
    return false;
  }
}

/** Determine the type of run for some runnable object */
export function getRunKind(runnable: RunnableObject): RunKind {
  if (isNode(runnable, NodeType.BLOCK)) {
    if (runnable.type == BlockType.TEXT) {
      return RunKind.TEXT;
    } else if (runnable.type == BlockType.CODE) {
      return RunKind.CODE;
    } else if (runnable.type == BlockType.FLOW) {
      return RunKind.FLOW;
    }
  } else if (isNode(runnable, NodeType.STEP)) {
    return RunKind.STEP;
  } else if (isStruct(runnable, StructType.TEXT)) {
    return RunKind.TEXT;
  } else if (isStruct(runnable, StructType.CODE)) {
    return RunKind.CODE;
  }

  throw new Error(`unexpected runnable type: ${describeNode(runnable)}`);
}

/** Make a new Run for some runnable node */
export function makeRun(
  graph: ReadNodeGraph,
  runnable: RunnableObject,
  options?: { inputsPacked?: Record<string, any>; packagePtr?: NodeReferenceData },
): RunData {
  let packagePtr: NodeReferenceData | undefined = undefined;
  let block: BlockData | undefined = undefined;
  let step: StepData | undefined = undefined;
  if (isNode(runnable, NodeType.BLOCK)) {
    block = runnable;
    packagePtr = options?.packagePtr ?? runnable.packagePtr;
  } else if (isNode(runnable, NodeType.STEP)) {
    step = runnable;
    block = graph.getAncestors(runnable, { includeSelf: true }).find((node) => isNode(node, NodeType.BLOCK));
    packagePtr = options?.packagePtr ?? step.packagePtr;
  } else if (isStruct(runnable, StructType.TEXT) || isStruct(runnable, StructType.CODE)) {
    if (options?.packagePtr == null) throw new Error(`missing package ptr for runnable lambda: ${runnable}`);
    packagePtr = options.packagePtr;
  } else {
    assertNever(runnable);
  }
  const run = makeNode({
    metatype: NodeType.RUN,
    parentPtr: packagePtr,
    packagePtr: packagePtr,
    kind: getRunKind(runnable),
    status: RunStatus.SCHEDULED,
    blockPtr: block != null ? toPlainNodeRef(block) : undefined,
    stepPtr: isNode(runnable, NodeType.STEP) ? toPlainNodeRef(runnable) : undefined,
    inputsPacked: options?.inputsPacked != null ? Struct.fromJson(options?.inputsPacked) : undefined,
  });
  return run;
}
