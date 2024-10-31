import { ACTIVE_RUN_STATUSES, HALTED_RUN_STATUSES, TERMINAL_RUN_STATUSES } from "@/language/const";
import type { ReadNodeGraph } from "@/language/graph";
import { makeNode } from "@/language/node";
import {
  BlockType,
  CodeData,
  FieldData,
  FieldType,
  NodeReferenceData,
  NodeType,
  RunKind,
  RunStatus,
  StructType,
  TextData,
  type AnyNodeData,
  type BlockData,
  type RunData,
  type StepData,
} from "@/proto/wire";
import { describeNode, isNode, isStruct, toPlainNodeRef } from "@/proto/wiring";
import { assertNever } from "@/utils/functools";
import { durationToMs, timestampToMs } from "@/utils/time";

export type RunnableNode = BlockData | StepData;
export type RunnableObject = RunnableNode | TextData | CodeData;

/** Whether the given node is runnable */
export function isRunnable(node: AnyNodeData, graph: ReadNodeGraph, fields?: FieldData[]): node is RunnableNode {
  if (isNode(node, NodeType.STEP)) {
    return true;
  } else if (isNode(node, NodeType.BLOCK)) {
    if (node.type == BlockType.ACTION || node.type == BlockType.FLOW) {
      return true;
    } else if (node.type == BlockType.TEXT) {
      fields = fields ?? graph.getChildren(node, NodeType.FIELD);
      return fields.some((f) => f.type == FieldType.INPUT) && fields.some((f) => f.type == FieldType.OUTPUT);
    } else {
      return false;
    }
  } else {
    return false;
  }
}

export function isRunActive(run: RunData): boolean {
  return ACTIVE_RUN_STATUSES.includes(run.status);
}

export function isRunHalted(run: RunData): boolean {
  return HALTED_RUN_STATUSES.includes(run.status);
}

export function isRunTerminal(run: RunData): boolean {
  return TERMINAL_RUN_STATUSES.includes(run.status);
}

/** Determine the type of run for some runnable object */
export function getRunKind(runnable: RunnableObject): RunKind {
  if (isNode(runnable, NodeType.BLOCK)) {
    if (runnable.type == BlockType.ACTION) {
      return RunKind.ACTION;
    } else if (runnable.type == BlockType.FLOW) {
      return RunKind.FLOW;
    }
  } else if (isNode(runnable, NodeType.STEP)) {
    return RunKind.STEP;
  }

  throw new Error(`unexpected runnable type: ${describeNode(runnable)}`);
}

/** Gets the duration of a Run */
export function getRunDurationMs(run: RunData, nowMs: number): number {
  if (TERMINAL_RUN_STATUSES.includes(run.status) && run.startedAt == null) {
    return 0; // never really started
  }
  const startedAtMs = timestampToMs(run.startedAt ?? run.createdAt!);
  let durationMs: number;
  if (run.duration != null) {
    durationMs = durationToMs(run.duration);
  } else {
    const currentMs = run.terminatedAt != null ? timestampToMs(run.terminatedAt) : nowMs;
    durationMs = currentMs - startedAtMs;
  }
  return Math.max(0, durationMs);
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
    inputsPacked: options?.inputsPacked ?? undefined,
  });
  return run;
}
