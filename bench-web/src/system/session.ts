import {
  BlockType,
  NodeType,
  RunKind,
  RunStatus,
  Struct,
  type BlockData,
  type RunData,
  type StepData,
} from "@/proto/wire";
import { describeNode, isNode, makeNode, toNodeReference } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";

export type RunnableNode = BlockData | StepData;

export function getRunKind(runnable: RunnableNode): RunKind {
  if (isNode(runnable, NodeType.BLOCK)) {
    if (runnable.type == BlockType.TEXT) {
      return RunKind.TEXT;
    } else if (runnable.type == BlockType.CODE) {
      return RunKind.CODE;
    } else if (runnable.type == BlockType.FLOW) {
      return RunKind.FLOW;
    }
  } else {
    return RunKind.STEP;
  }

  throw new Error(`unexpected runnable type: ${describeNode(runnable)}`);
}

export function makeRun(
  runnable: RunnableNode,
  graph: ReadNodeGraph,
  options?: { inputsPacked?: Record<string, any> },
): RunData {
  const block = isNode(runnable, NodeType.BLOCK)
    ? runnable
    : (graph.getAncestors(runnable, { includeSelf: true }).find((node) => isNode(node, NodeType.BLOCK)) as
        | BlockData
        | undefined);
  const run = makeNode({
    metatype: NodeType.RUN,
    parentPtr: runnable.packagePtr,
    packagePtr: runnable.packagePtr,
    kind: getRunKind(runnable),
    status: RunStatus.SCHEDULED,
    blockPtr: block != null ? toNodeReference(block) : undefined,
    stepPtr: isNode(runnable, NodeType.STEP) ? toNodeReference(runnable) : undefined,
    inputsPacked: options?.inputsPacked != null ? Struct.fromJson(options?.inputsPacked) : undefined,
  });
  return run;
}
