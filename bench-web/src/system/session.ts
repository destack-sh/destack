import { NodeType, RunKind, RunStatus, Struct, type BlockData, type RunData, type StepData } from "@/proto/wire";
import { isNode, makeNode, toNodeReference } from "@/proto/wiring";
import type { ReadNodeGraph } from "@/system/graph";

export type RunnableNode = BlockData | StepData;

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
    kind: isNode(runnable, NodeType.BLOCK) ? RunKind.BLOCK : RunKind.STEP,
    status: RunStatus.SCHEDULED,
    blockPtr: block != null ? toNodeReference(block) : undefined,
    stepPtr: isNode(runnable, NodeType.STEP) ? toNodeReference(runnable) : undefined,
    inputsPacked: options?.inputsPacked != null ? Struct.fromJson(options?.inputsPacked) : undefined,
  });
  return run;
}
